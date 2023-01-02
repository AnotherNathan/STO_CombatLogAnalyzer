use std::{
    ops::Add,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::SystemTime,
};

use chrono::{Duration, NaiveDateTime, NaiveTime};
use crossbeam_channel::{unbounded, Receiver, Sender};
use eframe::{egui::Context, epaint::mutex::RwLock};
use log::info;
use notify::{recommended_watcher, RecommendedWatcher, Watcher};
use timer::{Guard, Timer};

use crate::analyzer::{self, settings::AnalysisSettings, Analyzer, Combat};

pub struct AnalysisHandler {
    tx: Sender<Instruction>,
    rx: Receiver<AnalysisInfo>,
    is_busy: Arc<AtomicBool>,
}

struct AnalysisContext {
    rx: Receiver<Instruction>,
    tx: Sender<AnalysisInfo>,
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
    Refresh,
    AutoRefresh,
    GetCombat(usize),
}

pub enum AnalysisInfo {
    Combat(Combat),
    Refreshed {
        latest_combat: Combat,
        combats: Vec<String>,
    },
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
}

impl AnalysisContext {
    fn new(
        rx: Receiver<Instruction>,
        info_tx: Sender<AnalysisInfo>,
        instruction_tx: Sender<Instruction>,
        settings: AnalysisSettings,
        ctx: Context,
        is_busy: Arc<AtomicBool>,
        auto_refresh_interval_seconds: Option<f64>,
    ) -> Self {
        let auto_refresh = auto_refresh_interval_seconds
            .map(|i| {
                AutoRefreshContext::new(instruction_tx, i, &PathBuf::from(&settings.combatlog_file))
            })
            .flatten();
        Self {
            rx,
            tx: info_tx,
            analyzer: Analyzer::new(settings),
            ctx,
            is_busy,
            auto_refresh,
        }
    }

    fn run(&mut self) {
        loop {
            let instruction = match self.rx.recv() {
                Ok(i) => i,
                Err(_) => break,
            };

            match instruction {
                Instruction::Refresh => self.refresh(),
                Instruction::AutoRefresh => self.auto_refresh(),
                Instruction::GetCombat(combat_index) => {
                    self.get_combat(combat_index);
                }
            }
        }
    }

    fn refresh(&mut self) {
        let analyzer = match self.analyzer.as_mut() {
            Some(a) => a,
            None => return,
        };
        self.is_busy.store(true, Ordering::Relaxed);
        analyzer.update();
        let latest_combat = match analyzer.result().last() {
            Some(c) => c.clone(),
            None => return,
        };
        let info = AnalysisInfo::Refreshed {
            latest_combat,
            combats: analyzer.result().iter().map(|c| c.identifier()).collect(),
        };
        self.is_busy.store(false, Ordering::Release);
        self.send_info(info);
        if let Some(ctx) = &mut self.auto_refresh {
            ctx.state = AutoRefreshState::Idle;
            ctx.last_refresh = SystemTime::now();
        }
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

    fn send_info(&self, info: AnalysisInfo) {
        self.tx.send(info).unwrap();
        self.ctx.request_repaint();
    }
}

impl AutoRefreshContext {
    fn new(tx: Sender<Instruction>, interval_seconds: f64, file: &Path) -> Option<Self> {
        let interval = Self::interval(interval_seconds);
        let tx_watcher = tx.clone();
        let mut watcher = recommended_watcher(move |_| {
            tx_watcher.send(Instruction::AutoRefresh).unwrap();
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
