use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use crossbeam_channel::{unbounded, Receiver, Sender};
use eframe::egui::Context;

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
}

enum Instruction {
    Refresh,
    UpdateSettingsAndRefresh(AnalysisSettings),
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
    pub fn new(settings: AnalysisSettings, ctx: Context) -> Self {
        let (instruction_tx, instruction_rx) = unbounded();
        let (info_tx, info_rx) = unbounded();
        let is_busy = Arc::new(AtomicBool::new(false));
        let mut analysis_context =
            AnalysisContext::new(instruction_rx, info_tx, settings, ctx, is_busy.clone());
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

    pub fn update_settings_and_refresh(&self, settings: AnalysisSettings) {
        self.tx
            .send(Instruction::UpdateSettingsAndRefresh(settings))
            .unwrap();
    }

    pub fn get_combat(&self, combat_index: usize) {
        self.tx.send(Instruction::GetCombat(combat_index)).unwrap();
    }
}

impl AnalysisContext {
    fn new(
        rx: Receiver<Instruction>,
        tx: Sender<AnalysisInfo>,
        settings: AnalysisSettings,
        ctx: Context,
        is_busy: Arc<AtomicBool>,
    ) -> Self {
        Self {
            rx,
            tx,
            analyzer: Analyzer::new(settings),
            ctx,
            is_busy,
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
                Instruction::UpdateSettingsAndRefresh(s) => {
                    self.analyzer = Analyzer::new(s);
                    self.refresh();
                }
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
            combats: analyzer
                .result()
                .iter()
                .map(|c| c.identifier.clone())
                .collect(),
        };
        self.is_busy.store(false, Ordering::Release);
        self.send_info(info);
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
