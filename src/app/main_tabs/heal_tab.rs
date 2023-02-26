use eframe::egui::Ui;

use crate::analyzer::*;

use super::tables::*;

pub struct HealTab {
    table: HealTable,
    heal_group: fn(&Player) -> &HealGroup,
}
impl HealTab {
    pub fn empty(heal_group: fn(&Player) -> &HealGroup) -> Self {
        Self {
            table: HealTable::empty(),
            heal_group,
        }
    }

    pub fn update(&mut self, combat: &Combat) {
        self.table = HealTable::new(combat, self.heal_group);
    }

    pub fn show(&mut self, ui: &mut Ui) {
        self.table.show(ui, |_| {});
    }
}
