use std::{
    fs::File,
    io::Write,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, AtomicU32, Ordering},
        Arc,
    },
    time::SystemTime,
};

use chrono::Duration;
use crossbeam_channel::{unbounded, Receiver, Sender};
use eframe::egui::{Context, ViewportId};
use log::info;
use notify::{recommended_watcher, RecommendedWatcher, Watcher};
use timer::{Guard, Timer};

use crate::{
    analyzer::{settings::AnalysisSettings, Analyzer, Combat},
    unwrap_or_return,
};

pub struct AnalysisHandler {
    tx: Sender<Instruction>,
    rx: Receiver<AnalysisInfo>,
    is_busy: Arc<AtomicBool>,
    id: u32,
    id_counter: Arc<AtomicU32>,
}

struct AnalysisContext {
    instruction_rx: Receiver<Instruction>,
    instruction_tx: Sender<Instruction>,
    handlers: Vec<HandlerContext>,
    analyzer: Option<Analyzer>,
    ctx: Context,
    is_busy: Arc<AtomicBool>,
    auto_refresh_interval: Duration,
    auto_refresh: Option<AutoRefreshContext>,
}

#[derive(Debug)]
struct HandlerContext {
    tx: Sender<AnalysisInfo>,
    auto_refresh: bool,
    id: u32,
    viewport: ViewportId,
}

struct AutoRefreshContext {
    tx: Sender<Instruction>,
    _watcher: RecommendedWatcher,
    timer: Timer,
    state: AutoRefreshState,
    interval: Duration,
    last_refresh: SystemTime,
}

enum AutoRefreshState {
    Idle,
    RefreshScheduled(#[allow(dead_code)] Guard),
}

enum Instruction {
    Refresh(bool),
    AutoRefresh,
    GetCombat(usize, u32),
    ClearLog,
    SaveCombat(usize, PathBuf),
    EnableAutoRefresh(bool, u32),
    SetAutoRefreshInterval(f64),
    AddHandler(HandlerContext),
    RemoveHandler(u32),
    SetSettings(Arc<AnalysisSettings>),
}

#[derive(Clone)]
pub enum AnalysisInfo {
    Combat(Arc<Combat>),
    Refreshed {
        latest_combat: Arc<Combat>,
        combats: Vec<String>,
        file_size: Option<u64>,
    },
    RefreshError,
}

impl AnalysisHandler {
    pub fn new(
        settings: AnalysisSettings,
        ctx: Context,
        auto_refresh_interval_seconds: f64,
        enable_auto_refresh: bool,
    ) -> Self {
        let (instruction_tx, instruction_rx) = unbounded();
        let (info_tx, info_rx) = unbounded();
        let is_busy = Arc::new(AtomicBool::new(false));
        let handler_ctx = HandlerContext {
            auto_refresh: enable_auto_refresh,
            id: 0,
            tx: info_tx,
            viewport: ViewportId::ROOT,
        };

        let mut analysis_context = AnalysisContext::new(
            instruction_rx,
            handler_ctx,
            instruction_tx.clone(),
            settings,
            ctx,
            is_busy.clone(),
            auto_refresh_interval_seconds,
        );
        std::thread::spawn(move || {
            analysis_context.run();
        });
        Self {
            tx: instruction_tx,
            rx: info_rx,
            is_busy,
            id: 0,
            id_counter: AtomicU32::new(1).into(),
        }
    }

    pub fn is_busy(&self) -> bool {
        self.is_busy.load(Ordering::Relaxed)
    }

    pub fn check_for_info(&self) -> impl Iterator<Item = AnalysisInfo> + '_ {
        self.rx.try_iter()
    }

    pub fn refresh(&self) {
        self.tx.send(Instruction::Refresh(false)).unwrap();
    }

    pub fn get_combat(&self, combat_index: usize) {
        self.tx
            .send(Instruction::GetCombat(combat_index, self.id))
            .unwrap();
    }

    pub fn clear_log(&self) {
        self.tx.send(Instruction::ClearLog).unwrap();
    }

    pub fn save_combat(&self, combat_index: usize, file: PathBuf) {
        self.tx
            .send(Instruction::SaveCombat(combat_index, file))
            .unwrap();
    }

    pub fn set_settings(&self, settings: AnalysisSettings) {
        self.tx
            .send(Instruction::SetSettings(settings.into()))
            .unwrap();
    }

    pub fn enable_auto_refresh(&self, enable: bool) {
        self.tx
            .send(Instruction::EnableAutoRefresh(enable, self.id))
            .unwrap();
    }

    pub fn set_auto_refresh_interval(&self, refresh_interval: f64) {
        self.tx
            .send(Instruction::SetAutoRefreshInterval(refresh_interval))
            .unwrap();
    }

    pub fn get_handler(&self, auto_refresh: bool, viewport: ViewportId) -> Self {
        let (tx, rx) = unbounded();
        let id = self.id_counter.fetch_add(1, Ordering::Relaxed);
        let ctx = HandlerContext {
            auto_refresh,
            id,
            tx,
            viewport,
        };
        self.tx.send(Instruction::AddHandler(ctx)).unwrap();
        Self {
            tx: self.tx.clone(),
            rx,
            is_busy: self.is_busy.clone(),
            id,
            id_counter: self.id_counter.clone(),
        }
    }
}

impl Drop for AnalysisHandler {
    fn drop(&mut self) {
        let _ = self.tx.send(Instruction::RemoveHandler(self.id));
    }
}

impl AnalysisContext {
    fn new(
        instruction_rx: Receiver<Instruction>,
        handler_ctx: HandlerContext,
        instruction_tx: Sender<Instruction>,
        settings: AnalysisSettings,
        ctx: Context,
        is_busy: Arc<AtomicBool>,
        auto_refresh_interval_seconds: f64,
    ) -> Self {
        let mut _self = Self {
            instruction_rx,
            instruction_tx,
            handlers: vec![handler_ctx],
            analyzer: Analyzer::new(settings),
            ctx,
            is_busy,
            auto_refresh_interval: AutoRefreshContext::interval(auto_refresh_interval_seconds),
            auto_refresh: None,
        };
        _self.update_auto_refresh();
        _self
    }

    fn run(&mut self) {
        loop {
            let instruction = match self.instruction_rx.recv() {
                Ok(i) => i,
                Err(_) => return,
            };

            match instruction {
                Instruction::Refresh(auto_refresh) => self.refresh(auto_refresh),
                Instruction::AutoRefresh => self.auto_refresh(),
                Instruction::GetCombat(combat_index, handler) => {
                    self.get_combat(combat_index, handler);
                }
                Instruction::ClearLog => self.clear_log(),
                Instruction::SaveCombat(combat_index, file) => self.save_combat(combat_index, file),
                Instruction::EnableAutoRefresh(enable, handler) => {
                    self.handler_mut(handler, |h| h.auto_refresh = enable);
                    self.update_auto_refresh();
                }
                Instruction::SetAutoRefreshInterval(refresh_interval) => {
                    self.set_auto_refresh_interval(refresh_interval)
                }
                Instruction::AddHandler(tx) => {
                    self.handlers.push(tx);
                    self.update_auto_refresh();
                }
                Instruction::RemoveHandler(id) => {
                    if let Some(index) = self.handlers.iter().position(|t| t.id == id) {
                        self.handlers.remove(index);
                        if self.handlers.len() == 0 {
                            return;
                        }
                        self.update_auto_refresh();
                    }
                }
                Instruction::SetSettings(settings) => {
                    self.analyzer = Analyzer::new(Arc::into_inner(settings).unwrap())
                }
            }

            Self::set_is_busy(&self.is_busy, false);
        }
    }

    fn refresh(&mut self, only_when_auto_refresh: bool) {
        Self::set_is_busy(&self.is_busy, true);
        let info = self.try_refresh();
        if only_when_auto_refresh {
            for handler in self.handlers.iter().filter(|h| h.auto_refresh) {
                handler.send(info.clone(), &self.ctx);
            }
        } else {
            self.send_info_all(info);
        }
        if let Some(ctx) = &mut self.auto_refresh {
            ctx.state = AutoRefreshState::Idle;
            ctx.last_refresh = SystemTime::now();
        }
    }

    fn try_refresh(&mut self) -> AnalysisInfo {
        let analyzer = match self.analyzer.as_mut() {
            Some(a) => a,
            None => return AnalysisInfo::RefreshError,
        };
        analyzer.update();
        let latest_combat = match analyzer.result().last() {
            Some(c) => c.clone(),
            None => return AnalysisInfo::RefreshError,
        };
        let info = AnalysisInfo::Refreshed {
            latest_combat: latest_combat.into(),
            combats: analyzer.result().iter().map(|c| c.identifier()).collect(),
            file_size: std::fs::metadata(&analyzer.settings().combatlog_file)
                .ok()
                .map(|m| m.len()),
        };
        info
    }

    fn auto_refresh(&mut self) {
        if let Some(ctx) = &mut self.auto_refresh {
            if let AutoRefreshState::RefreshScheduled(_) = ctx.state {
                return;
            }

            let delta_time = match ctx.last_refresh.elapsed().map(|d| Duration::from_std(d)) {
                Ok(Ok(t)) => t,
                err => {
                    info!("failed to retrieve elapsed time {:?}", err);
                    return;
                }
            };

            if delta_time >= ctx.interval {
                ctx.state = AutoRefreshState::Idle;
                self.refresh(true);
                return;
            }

            let delay = ctx.interval - delta_time;
            let tx = ctx.tx.clone();
            let guard = ctx
                .timer
                .schedule_with_delay(delay, move || _ = tx.send(Instruction::Refresh(true)));
            ctx.state = AutoRefreshState::RefreshScheduled(guard);
        }
    }

    fn get_combat(&self, combat_index: usize, handler: u32) {
        let analyzer = match &self.analyzer {
            Some(a) => a,
            None => return,
        };

        let combat = match analyzer.result().get(combat_index) {
            Some(c) => c.clone(),
            None => return,
        };

        self.send_info(AnalysisInfo::Combat(combat.into()), handler);
    }

    fn clear_log(&mut self) {
        let analyzer = match &self.analyzer {
            Some(a) => a,
            None => return,
        };
        let settings = analyzer.settings().clone();

        let last_combat = analyzer.result().last();
        let last_combat_data = last_combat
            .map(|c| c.read_log_combat_data(settings.combatlog_file()))
            .flatten();

        self.analyzer = None;

        let mut file = match File::options()
            .write(true)
            .truncate(true)
            .create(false)
            .open(settings.combatlog_file())
        {
            Ok(f) => f,
            Err(_) => return,
        };

        if let Some(last_combat_data) = last_combat_data {
            let _ = file.write_all(last_combat_data.as_slice());
        }

        drop(file);
        self.analyzer = Analyzer::new(settings);
        self.refresh(false);
    }

    fn save_combat(&self, combat_index: usize, file: PathBuf) {
        let analyzer = unwrap_or_return!(&self.analyzer);
        let combat = unwrap_or_return!(analyzer.result().get(combat_index));
        Self::set_is_busy(&self.is_busy, true);
        let combat_data = match combat.read_log_combat_data(analyzer.settings().combatlog_file()) {
            Some(d) => d,
            None => {
                Self::set_is_busy(&self.is_busy, false);
                return;
            }
        };
        let _ = std::fs::write(file, combat_data.as_slice());
        Self::set_is_busy(&self.is_busy, false);
    }

    fn send_info(&self, info: AnalysisInfo, handler: u32) {
        self.handler(handler, |handler| handler.send(info, &self.ctx));
    }

    fn send_info_all(&self, info: AnalysisInfo) {
        for handler in self.handlers.iter() {
            handler.send(info.clone(), &self.ctx);
        }
    }

    fn handler(&self, handler: u32, action: impl FnOnce(&HandlerContext)) {
        if let Some(handler) = self.handlers.iter().find(|h| h.id == handler) {
            action(handler);
        }
    }

    fn handler_mut(&mut self, handler: u32, action: impl FnOnce(&mut HandlerContext)) {
        if let Some(handler) = self.handlers.iter_mut().find(|h| h.id == handler) {
            action(handler);
        }
    }

    fn set_is_busy(is_busy: &AtomicBool, value: bool) {
        is_busy.store(value, Ordering::Relaxed);
    }

    fn set_auto_refresh_interval(&mut self, refresh_interval: f64) {
        self.auto_refresh_interval = AutoRefreshContext::interval(refresh_interval);
        self.update_auto_refresh();
    }

    fn update_auto_refresh(&mut self) {
        let settings = match &self.analyzer {
            Some(analyzer) => analyzer.settings(),
            None => return,
        };
        if !self.auto_refresh_enabled() {
            self.auto_refresh = None;
            return;
        }
        self.auto_refresh = AutoRefreshContext::new(
            self.instruction_tx.clone(),
            self.auto_refresh_interval,
            &PathBuf::from(&settings.combatlog_file),
        );
    }

    fn auto_refresh_enabled(&self) -> bool {
        self.handlers.iter().any(|h| h.auto_refresh)
    }
}

impl AutoRefreshContext {
    fn new(tx: Sender<Instruction>, interval: Duration, file: &Path) -> Option<Self> {
        let tx_watcher = tx.clone();
        let mut watcher = recommended_watcher(move |_| {
            let _ = tx_watcher.send(Instruction::AutoRefresh);
        })
        .ok()?;

        watcher
            .watch(file, notify::RecursiveMode::NonRecursive)
            .ok()?;

        Some(Self {
            tx,
            timer: Timer::new(),
            state: AutoRefreshState::Idle,
            interval,
            _watcher: watcher,
            last_refresh: SystemTime::now(),
        })
    }

    fn interval(interval_seconds: f64) -> Duration {
        Duration::milliseconds((interval_seconds * 1.0e3) as _)
    }
}

impl HandlerContext {
    fn send(&self, info: AnalysisInfo, ctx: &Context) {
        match self.tx.send(info) {
            Ok(_) => ctx.request_repaint_of(self.viewport),
            Err(_) => (),
        }
    }
}
