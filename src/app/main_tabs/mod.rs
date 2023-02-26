use eframe::egui::*;

use crate::analyzer::Combat;

use self::{damage_tab::DamageTab, heal_tab::HealTab, summary_tab::SummaryTab};

mod common;
mod damage_tab;
mod diagrams;
mod heal_tab;
mod summary_tab;
mod tables;

pub struct MainTabs {
    pub identifier: String,
    pub summary_tab: SummaryTab,
    pub damage_out_tab: DamageTab,
    pub damage_in_tab: DamageTab,
    pub heal_out_tab: HealTab,
    pub heal_in_tab: HealTab,

    active_tab: MainTab,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum MainTab {
    #[default]
    Summary,
    DamageOut,
    DamageIn,
    HealOut,
    HealIn,
}

impl MainTabs {
    pub fn empty() -> Self {
        Self {
            identifier: String::new(),
            damage_out_tab: DamageTab::empty(|p| &p.damage_out),
            damage_in_tab: DamageTab::empty(|p| &p.damage_in),
            heal_out_tab: HealTab::empty(|p| &p.heal_out),
            heal_in_tab: HealTab::empty(|p| &p.heal_in),
            active_tab: Default::default(),
            summary_tab: SummaryTab::empty(),
        }
    }

    pub fn update(&mut self, combat: &Combat) {
        self.identifier = combat.identifier();
        self.summary_tab.update(combat);
        self.damage_out_tab.update(combat);
        self.damage_in_tab.update(combat);
        self.heal_out_tab.update(combat);
        self.heal_in_tab.update(combat);
    }

    pub fn show(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.selectable_value(&mut self.active_tab, MainTab::Summary, "Summary");

            ui.selectable_value(&mut self.active_tab, MainTab::DamageOut, "Outgoing Damage");
            ui.selectable_value(&mut self.active_tab, MainTab::DamageIn, "Incoming Damage");

            ui.selectable_value(&mut self.active_tab, MainTab::HealOut, "Outgoing Healing");
            ui.selectable_value(&mut self.active_tab, MainTab::HealIn, "Incoming Healing");
        });

        match self.active_tab {
            MainTab::Summary => self.summary_tab.show(ui),
            MainTab::DamageOut => self.damage_out_tab.show(ui),
            MainTab::DamageIn => self.damage_in_tab.show(ui),
            MainTab::HealOut => self.heal_out_tab.show(ui),
            MainTab::HealIn => self.heal_in_tab.show(ui),
        }
    }
}
