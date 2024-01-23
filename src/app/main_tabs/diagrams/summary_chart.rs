use eframe::egui::Ui;
use egui_plot::*;

use super::common::*;

pub struct SummaryChart {
    identifier: String,
    players: Vec<Bar>,
}

impl SummaryChart {
    pub fn empty() -> Self {
        Self {
            identifier: String::new(),
            players: Default::default(),
        }
    }

    pub fn from_data<'a>(identifier: &str, players: impl Iterator<Item = (&'a str, f64)>) -> Self {
        let mut players: Vec<_> = players.map(|(n, v)| Bar::new(0.0, v).name(n)).collect();

        players.sort_unstable_by(|p1, p2| p1.value.total_cmp(&p2.value).reverse());

        players.iter_mut().enumerate().for_each(|(i, p)| {
            p.argument = i as f64 + 1.0;
        });

        Self {
            identifier: identifier.to_string(),
            players,
        }
    }

    pub fn show(&mut self, ui: &mut Ui) {
        Plot::new(&self.identifier)
            .auto_bounds(true.into())
            .y_axis_formatter(|_, _, _| String::new())
            .x_axis_formatter(format_axis)
            .y_axis_width(0)
            .legend(Legend::default())
            .include_y(0.0)
            .show(ui, |p| {
                for player in self.players.iter() {
                    let chart = BarChart::new(vec![player.clone()])
                        .element_formatter(Box::new(format_element))
                        .name(&player.name)
                        .horizontal();
                    p.bar_chart(chart);
                }
            });
    }
}
