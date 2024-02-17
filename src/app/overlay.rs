use std::sync::Arc;

use eframe::{egui::*, epaint::mutex::Mutex};

use crate::{
    analyzer::{Combat, Player},
    custom_widgets::{popup_button::PopupButton, table::Table},
    helpers::number_formatting::NumberFormatter,
};

use super::analysis_handling::{AnalysisHandler, AnalysisInfo};

pub struct Overlay(Arc<Mutex<OverlayInner>>);

struct OverlayInner {
    position: Option<Pos2>,
    current_size: Vec2,
    data: DisplayData,
    show: bool,
    move_around: bool,
    columns: Vec<ColumnDescriptor>,
    analysis_handler: AnalysisHandler,
    state: State,
}

#[derive(Default)]
enum State {
    Update(Arc<Combat>),
    Idle(Arc<Combat>),
    #[default]
    Empty,
}

#[derive(Default)]
struct DisplayData {
    columns: Vec<ColumnDescriptor>,
    players: Vec<DisplayPlayer>,
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
    col!("Dmg Out", |p, f| {
        val(
            p.damage_out.damage_metrics.total_damage.all,
            f.format(p.damage_out.damage_metrics.total_damage.all, 2),
        )
    }),
    col!("Dmg Out %", |p, f| {
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
    col!("Dmg In", |p, f| {
        val(
            p.damage_in.damage_metrics.total_damage.all,
            f.format(p.damage_in.damage_metrics.total_damage.all, 2),
        )
    }),
    col!("Dmg In %", |p, f| {
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
    pub fn new(root_handler: &AnalysisHandler) -> Self {
        Self(Arc::new(Mutex::new(OverlayInner {
            move_around: true,
            columns: COLUMNS.iter().cloned().collect(),
            current_size: Vec2::ZERO,
            data: Default::default(),
            position: None,
            show: false,
            analysis_handler: root_handler.get_handler(true, Self::viewport_id()),
            state: State::Empty,
        })))
    }

    pub fn show(self: &Self, ui: &mut Ui) {
        let mut inner = self.0.lock();

        if Button::new("Overlay")
            .selected(inner.show)
            .ui(ui)
            .on_hover_text("Enables an Overlay, that you can move in front of the game window. Note that the it will always show the newest combat.")
            .clicked()
        {
            inner.toggle_show();
        }

        PopupButton::new("⛭").show(ui, |ui| {
            ui.label("Configure what columns are displayed in the Overlay");
            let mut config_changed = false;
            for column in inner.columns.iter_mut() {
                if ui.checkbox(&mut column.enabled, column.name).clicked() {
                    config_changed = true;
                }
            }
            if config_changed {
                inner.force_update(ui.ctx());
            }
        });

        ui.add_enabled_ui(inner.show, |ui: &mut Ui| {
            if Button::new("✋")
                .selected(inner.move_around)
                .ui(ui)
                .on_hover_text("Move the Overlay")
                .clicked()
            {
                inner.move_around = !inner.move_around;
            }
        });

        inner.poll_update(ui.ctx());
        if !inner.show {
            return;
        }

        let mut builder = ViewportBuilder::default()
            .with_title("CLA Overlay")
            .with_decorations(inner.move_around)
            .with_minimize_button(false)
            .with_maximize_button(false)
            .with_close_button(true)
            .with_resizable(false)
            .with_min_inner_size(vec2(240.0, 80.0))
            .with_inner_size(inner.current_size)
            .with_always_on_top()
            .with_taskbar(false)
            .with_mouse_passthrough(!inner.move_around);
        builder.position = inner.position;
        drop(inner);
        let inner = self.0.clone();
        ui.ctx()
            .show_viewport_deferred(Self::viewport_id(), builder, move |ctx, _| {
                inner.lock().show_overlay(ctx);
            });
    }

    pub fn viewport_id() -> ViewportId {
        ViewportId("overlay".into())
    }

    pub fn request_repaint(ctx: &Context) {
        ctx.request_repaint_of(Self::viewport_id());
    }
}

impl OverlayInner {
    fn show_overlay(&mut self, ctx: &Context) {
        self.check_update(ctx);
        CentralPanel::default().show(ctx, |ui| {
            if ctx.input_for(Overlay::viewport_id(), |i| i.viewport().close_requested()) {
                self.toggle_show();
            }
            self.position = ctx.input_for(Overlay::viewport_id(), |i| {
                i.viewport().outer_rect.map(|r| r.left_top())
            });
            let required_size = Table::new(ui)
                .min_scroll_height(f32::MAX)
                .header(15.0, |h| {
                    h.cell(|ui| {
                        ui.label("Player");
                    });

                    for column in self.data.columns.iter() {
                        h.cell(|ui| {
                            ui.label(column.name);
                        });
                    }
                })
                .body(25.0, |t| {
                    for player in self.data.players.iter() {
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
            let required_size = required_size.ceil();
            if self.current_size != required_size {
                ctx.send_viewport_cmd_to(
                    Overlay::viewport_id(),
                    ViewportCommand::InnerSize(required_size),
                );
                self.current_size = required_size;
            }
        });
    }

    fn toggle_show(&mut self) {
        self.show = !self.show;
        self.analysis_handler.enable_auto_refresh(self.show);
    }

    fn check_update(&mut self, ctx: &Context) {
        self.poll_update(ctx);
        let combat = match &self.state {
            State::Update(c) => c.clone(),
            State::Idle(_) | State::Empty => return,
        };
        self.perform_update(ctx, &combat);
        self.state = State::Idle(combat.clone());
    }

    fn poll_update(&mut self, ctx: &Context) {
        let combat = match self.analysis_handler.check_for_info().last() {
            Some(AnalysisInfo::Refreshed {
                latest_combat,
                combats: _,
                file_size: _,
            }) => latest_combat,
            _ => return,
        };
        self.state = State::Update(combat);
        if self.show {
            ctx.request_repaint_of(Overlay::viewport_id());
        }
    }

    fn force_update(&mut self, ctx: &Context) {
        match &self.state {
            State::Update(c) | State::Idle(c) => self.perform_update(ctx, &c.clone()),
            State::Empty => (),
        }
    }

    fn perform_update(&mut self, ctx: &Context, combat: &Combat) {
        if self.show {
            ctx.request_repaint_of(Overlay::viewport_id());
        }

        let mut display_data = DisplayData::default();
        display_data.columns = self.columns.iter().filter(|c| c.enabled).cloned().collect();
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
        self.data = display_data;
    }
}

impl DisplayPlayer {
    fn sort_value(&self) -> f64 {
        self.columns.first().map(|c| c.value).unwrap_or(0.0)
    }
}
