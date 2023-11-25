use std::{
    fs::File,
    io::{BufRead, BufReader, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::SystemTime,
};

use chrono::Duration;
use crossbeam_channel::{unbounded, Receiver, Sender};
use eframe::egui::Context;
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
}

struct AnalysisContext {
    instruction_rx: Receiver<Instruction>,
    instruction_tx: Sender<Instruction>,
    info_tx: Sender<AnalysisInfo>,
    analyzer: Option<Analyzer>,
    ctx: Context,
    is_busy: Arc<AtomicBool>,
    auto_refresh: Option<AutoRefreshContext>,
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
    RefreshScheduled(Guard),
}

enum Instruction {
    Exit,
    Refresh,
    AutoRefresh,
    GetCombat(usize),
    ClearLog,
    SaveCombat(usize, PathBuf),
    SetAutoRefresh(Option<f64>),
}

pub enum AnalysisInfo {
    Combat(Combat),
    Refreshed {
        latest_combat: Combat,
        combats: Vec<String>,
        file_size: Option<u64>,
    },
    RefreshError,
}

impl AnalysisHandler {
    pub fn new(
        settings: AnalysisSettings,
        ctx: Context,
        auto_refresh_interval_seconds: Option<f64>,
    ) -> Self {
        let (instruction_tx, instruction_rx) = unbounded();
        let (info_tx, info_rx) = unbounded();
        let is_busy = Arc::new(AtomicBool::new(false));

        let mut analysis_context = AnalysisContext::new(
            instruction_rx,
            info_tx,
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
        }
    }

    pub fn is_busy(&self) -> bool {
        self.is_busy.load(Ordering::Relaxed)
    }

    pub fn check_for_info(&self) -> impl Iterator<Item = AnalysisInfo> + '_ {
        self.rx.try_iter()
    }

    pub fn refresh(&self) {
        self.tx.send(Instruction::Refresh).unwrap();
    }

    pub fn get_combat(&self, combat_index: usize) {
        self.tx.send(Instruction::GetCombat(combat_index)).unwrap();
    }

    pub fn clear_log(&self) {
        self.tx.send(Instruction::ClearLog).unwrap();
    }

    pub fn save_combat(&self, combat_index: usize, file: PathBuf) {
        self.tx
            .send(Instruction::SaveCombat(combat_index, file))
            .unwrap();
    }

    pub fn set_auto_refresh(&self, refresh_interval: Option<f64>) {
        self.tx
            .send(Instruction::SetAutoRefresh(refresh_interval))
            .unwrap();
    }
}

impl Drop for AnalysisHandler {
    fn drop(&mut self) {
        let _ = self.tx.send(Instruction::Exit);
    }
}

impl AnalysisContext {
    fn new(
        instruction_rx: Receiver<Instruction>,
        info_tx: Sender<AnalysisInfo>,
        instruction_tx: Sender<Instruction>,
        settings: AnalysisSettings,
        ctx: Context,
        is_busy: Arc<AtomicBool>,
        auto_refresh_interval_seconds: Option<f64>,
    ) -> Self {
        let auto_refresh = auto_refresh_interval_seconds
            .map(|i| {
                AutoRefreshContext::new(
                    instruction_tx.clone(),
                    i,
                    &PathBuf::from(&settings.combatlog_file),
                )
            })
            .flatten();
        Self {
            instruction_rx,
            instruction_tx,
            info_tx,
            analyzer: Analyzer::new(settings),
            ctx,
            is_busy,
            auto_refresh,
        }
    }

    fn run(&mut self) {
        loop {
            let instruction = match self.instruction_rx.recv() {
                Ok(i) => i,
                Err(_) => return,
            };

            match instruction {
                Instruction::Exit => return,
                Instruction::Refresh => self.refresh(),
                Instruction::AutoRefresh => self.auto_refresh(),
                Instruction::GetCombat(combat_index) => {
                    self.get_combat(combat_index);
                }
                Instruction::ClearLog => self.clear_log(),
                Instruction::SaveCombat(combat_index, file) => self.save_combat(combat_index, file),
                Instruction::SetAutoRefresh(refresh_interval) => {
                    self.set_auto_refresh(refresh_interval)
                }
            }

            Self::set_is_busy(&self.is_busy, false);
        }
    }

    fn refresh(&mut self) {
        Self::set_is_busy(&self.is_busy, true);
        let info = self.try_refresh();
        self.send_info(info);
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
            latest_combat,
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
                self.refresh();
                return;
            }

            let delay = ctx.interval - delta_time;
            let tx = ctx.tx.clone();
            let guard = ctx
                .timer
                .schedule_with_delay(delay, move || _ = tx.send(Instruction::Refresh));
            ctx.state = AutoRefreshState::RefreshScheduled(guard);
        }
    }

    fn get_combat(&self, combat_index: usize) {
        let analyzer = match &self.analyzer {
            Some(a) => a,
            None => return,
        };

        let combat = match analyzer.result().get(combat_index) {
            Some(c) => c.clone(),
            None => return,
        };

        self.send_info(AnalysisInfo::Combat(combat));
    }

    fn clear_log(&mut self) {
        let analyzer = match &self.analyzer {
            Some(a) => a,
            None => return,
        };
        let settings = analyzer.settings().clone();

        let last_combat = analyzer.result().last();
        let last_combat_data = last_combat
            .map(|c| Self::read_log_combat_data(settings.combatlog_file(), c))
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
            let _ = file.write_all(last_combat_data.as_bytes());
        }

        drop(file);
        self.analyzer = Analyzer::new(settings);
        self.refresh();
    }

    fn save_combat(&self, combat_index: usize, file: PathBuf) {
        let analyzer = unwrap_or_return!(&self.analyzer);
        let combat = unwrap_or_return!(analyzer.result().get(combat_index));
        Self::set_is_busy(&self.is_busy, true);
        let combat_data =
            match Self::read_log_combat_data(analyzer.settings().combatlog_file(), combat) {
                Some(d) => d,
                None => {
                    Self::set_is_busy(&self.is_busy, false);
                    return;
                }
            };
        let _ = std::fs::write(file, combat_data.as_bytes());
        Self::set_is_busy(&self.is_busy, false);
    }

    fn read_log_combat_data(file_path: &Path, combat: &Combat) -> Option<String> {
        let pos = match combat.log_pos.clone() {
            Some(p) => p,
            None => return None,
        };

        let file = match File::options().create(false).read(true).open(file_path) {
            Ok(f) => f,
            Err(_) => return None,
        };

        let mut combat_data = String::new();
        let mut reader = BufReader::with_capacity(1 << 20, file);
        reader.seek(SeekFrom::Start(pos.start)).ok()?;

        loop {
            let count = reader.read_line(&mut combat_data).ok()?;
            if count == 0 || reader.stream_position().ok()? >= pos.end {
                break;
            }
        }

        Some(combat_data)
    }

    fn send_info(&self, info: AnalysisInfo) {
        self.info_tx.send(info).unwrap();
        self.ctx.request_repaint();
    }

    fn set_is_busy(is_busy: &AtomicBool, value: bool) {
        is_busy.store(value, Ordering::Relaxed);
    }

    fn set_auto_refresh(&mut self, refresh_interval: Option<f64>) {
        let settings = match &self.analyzer {
            Some(analyzer) => analyzer.settings(),
            None => return,
        };
        self.auto_refresh = refresh_interval
            .map(|i| {
                AutoRefreshContext::new(
                    self.instruction_tx.clone(),
                    i,
                    &PathBuf::from(&settings.combatlog_file),
                )
            })
            .flatten();
    }
}

impl AutoRefreshContext {
    fn new(tx: Sender<Instruction>, interval_seconds: f64, file: &Path) -> Option<Self> {
        let interval = Self::interval(interval_seconds);
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
