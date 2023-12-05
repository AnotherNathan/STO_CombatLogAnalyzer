use std::sync::Arc;

use eframe::{egui::*, epaint::mutex::Mutex};

use crate::{
    analyzer::{Combat, Player},
    custom_widgets::{popup_button::PopupButton, table::Table},
    helpers::number_formatting::NumberFormatter,
};

pub struct Overlay {
    show: bool,
    startup: bool,
    move_around: bool,
    display_data: Arc<Mutex<DisplayData>>,
    columns: Vec<ColumnDescriptor>,
}

#[derive(Default)]
struct DisplayData {
    columns: Vec<ColumnDescriptor>,
    players: Vec<DisplayPlayer>,
    current_size: Vec2,
}

struct DisplayPlayer {
    name: String,
    columns: Vec<ColumnValue>,
}

struct ColumnValue {
    value: f64,
    value_string: String,
}

fn val(value: f64, value_string: String) -> ColumnValue {
    ColumnValue {
        value,
        value_string,
    }
}

#[derive(Clone)]
struct ColumnDescriptor {
    name: &'static str,
    enabled: bool,
    select: fn(&Player, &mut NumberFormatter) -> ColumnValue,
}

macro_rules! col {
    ($name:expr, $select:expr $(,)?) => {
        ColumnDescriptor {
            name: $name,
            enabled: false,
            select: $select,
        }
    };
    ($name:expr, $enabled:expr, $select:expr $(,)?) => {
        ColumnDescriptor {
            name: $name,
            enabled: $enabled,
            select: $select,
        }
    };
}

static COLUMNS: &[ColumnDescriptor] = &[
    col!("DPS", true, |p, f| {
        val(
            p.damage_out.damage_metrics.dps.all,
            f.format(p.damage_out.damage_metrics.dps.all, 2),
        )
    }),
    col!("Total Damage Out", |p, f| {
        val(
            p.damage_out.damage_metrics.total_damage.all,
            f.format(p.damage_out.damage_metrics.total_damage.all, 2),
        )
    }),
    col!("Damage Out %", |p, f| {
        val(
            p.damage_out.damage_percentage.all.unwrap_or(0.0),
            p.damage_out
                .damage_percentage
                .all
                .map(|p| f.format(p, 3))
                .unwrap_or(String::new()),
        )
    }),
    col!("Max One-Hit", |p, f| {
        val(
            p.damage_out.max_one_hit.damage,
            f.format(p.damage_out.max_one_hit.damage, 2),
        )
    }),
    col!("Total Damage In", |p, f| {
        val(
            p.damage_in.damage_metrics.total_damage.all,
            f.format(p.damage_in.damage_metrics.total_damage.all, 2),
        )
    }),
    col!("Damage In %", |p, f| {
        val(
            p.damage_in.damage_percentage.all.unwrap_or(0.0),
            p.damage_in
                .damage_percentage
                .all
                .map(|p| f.format(p, 3))
                .unwrap_or(String::new()),
        )
    }),
    col!("Hits Out", |p, _| {
        val(
            p.damage_out.damage_metrics.hits.all as _,
            p.damage_out.damage_metrics.hits.all.to_string(),
        )
    }),
    col!("Hits Out %", |p, f| {
        val(
            p.damage_out.hits_percentage.all.unwrap_or(0.0),
            p.damage_out
                .hits_percentage
                .all
                .map(|p| f.format(p, 3))
                .unwrap_or(String::new()),
        )
    }),
    col!("Hits In", |p, _| {
        val(
            p.damage_out.damage_metrics.hits.all as _,
            p.damage_out.damage_metrics.hits.all.to_string(),
        )
    }),
    col!("Hits In %", |p, f| {
        val(
            p.damage_in.hits_percentage.all.unwrap_or(0.0),
            p.damage_in
                .hits_percentage
                .all
                .map(|p| f.format(p, 3))
                .unwrap_or(String::new()),
        )
    }),
    col!("Kills", |p, _| {
        let count: u32 = p.damage_out.kills.values().copied().sum();
        val(count as _, count.to_string())
    }),
    col!("Deaths", |p, _| {
        let count: u32 = p.damage_in.kills.values().copied().sum();
        val(count as _, count.to_string())
    }),
];

impl Overlay {
    pub fn show(&mut self, ui: &mut Ui, combat: Option<&Combat>) {
        if Button::new("Overlay")
            .selected(self.show)
            .ui(ui)
            .on_hover_text("Enables an Overlay, that you can move in front of the game window. Note that for the Overlay to update, Auto Refresh must be enabled.")
            .clicked()
        {
            self.show = !self.show;
            self.update(ui.ctx(), combat);
        }

        PopupButton::new("⛭").show(ui, |ui| {
            ui.label("Configure what columns are display in the Overlay");
            let mut config_changed = false;
            for column in self.columns.iter_mut() {
                if ui.checkbox(&mut column.enabled, column.name).clicked() {
                    config_changed = true;
                }
            }
            if config_changed {
                self.update(ui.ctx(), combat);
            }
        });

        ui.add_enabled_ui(self.show, |ui: &mut Ui| {
            if Button::new("✋")
                .selected(self.move_around)
                .ui(ui)
                .on_hover_text("Move the Overlay")
                .clicked()
            {
                self.move_around = !self.move_around;
                self.update(ui.ctx(), combat);
            }
        });

        let display_data = self.display_data.clone();
        ui.ctx().show_viewport_deferred(
            Self::viewport_id(),
            ViewportBuilder::default()
                .with_decorations(self.move_around)
                .with_minimize_button(false)
                .with_close_button(false)
                .with_resizable(false)
                .with_min_inner_size(vec2(240.0, 80.0))
                .with_visible(self.show || self.startup)
                .with_always_on_top()
                .with_mouse_passthrough(!self.move_around),
            move |ctx, _| {
                Self::show_overlay(ctx, &display_data);
            },
        );

        self.startup = self.startup
            && !ui.ctx().input(|i| {
                i.events.iter().any(|e| match e {
                    Event::WindowFocused(_) => true,
                    _ => false,
                })
            });
    }

    pub fn update(&mut self, ctx: &Context, combat: Option<&Combat>) {
        let mut display_data = DisplayData::default();
        display_data.columns = self.columns.iter().filter(|c| c.enabled).cloned().collect();
        let combat = match combat {
            Some(c) => c,
            None => {
                *self.display_data.lock() = Default::default();
                return;
            }
        };
        let mut formatter = NumberFormatter::new();
        for (&player_name, player) in combat.players.iter() {
            let mut display_player = DisplayPlayer {
                name: combat
                    .name_manager
                    .get_name(player_name)
                    .unwrap()
                    .to_string(),
                columns: Vec::new(),
            };
            for column in display_data.columns.iter() {
                display_player
                    .columns
                    .push((column.select)(player, &mut formatter));
            }
            display_data.players.push(display_player);
        }

        if display_data.columns.len() > 0 {
            display_data
                .players
                .sort_by(|p1, p2| p1.sort_value().total_cmp(&p2.sort_value()).reverse());
        }
        *self.display_data.lock() = display_data;
        ctx.request_repaint_of(Self::viewport_id());
    }

    fn show_overlay(ctx: &Context, display_data: &Mutex<DisplayData>) {
        CentralPanel::default().show(ctx, |ui| {
            let mut display_data = display_data.lock();
            let required_size = Table::new(ui)
                .min_scroll_height(f32::MAX)
                .header(15.0, |h| {
                    h.cell(|ui| {
                        ui.label("Player");
                    });

                    for column in display_data.columns.iter() {
                        h.cell(|ui| {
                            ui.label(column.name);
                        });
                    }
                })
                .body(25.0, |t| {
                    for player in display_data.players.iter() {
                        t.row(|r| {
                            r.cell(|ui| {
                                ui.label(player.name.as_str());
                            });

                            for column in player.columns.iter() {
                                r.cell(|ui| {
                                    ui.label(column.value_string.as_str());
                                });
                            }
                        });
                    }
                })
                .size();
            let required_size = required_size
                + ui.spacing().window_margin.left_top()
                + ui.spacing().window_margin.right_bottom()
                + ui.spacing().item_spacing;
            let required_size =
                (required_size * ctx.native_pixels_per_point().unwrap_or(1.0)).ceil();
            if display_data.current_size != required_size {
                ctx.send_viewport_cmd_to(
                    Self::viewport_id(),
                    ViewportCommand::InnerSize(required_size),
                );
                display_data.current_size = required_size;
            }
        });
    }

    pub fn viewport_id() -> ViewportId {
        ViewportId("overlay".into())
    }
}

impl Default for Overlay {
    fn default() -> Self {
        Self {
            show: false,
            startup: true,
            move_around: true,
            display_data: Default::default(),
            columns: COLUMNS.iter().cloned().collect(),
        }
    }
}

impl DisplayPlayer {
    fn sort_value(&self) -> f64 {
        self.columns.first().map(|c| c.value).unwrap_or(0.0)
    }
}
