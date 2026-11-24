use crate::i18n::Translations;
use eframe::egui::{self, Color32, RichText, Ui};
use egui_plot::{Legend, Line, Plot, PlotPoints};
use std::collections::HashMap;

/// Cores distintas para as linhas de cada host no gráfico
const HOST_COLORS: &[Color32] = &[
    Color32::from_rgb(52, 152, 219), // Azul
    Color32::from_rgb(46, 204, 113), // Verde
    Color32::from_rgb(241, 196, 15), // Amarelo
    Color32::from_rgb(155, 89, 182), // Roxo
    Color32::from_rgb(230, 126, 34), // Laranja
    Color32::from_rgb(26, 188, 156), // Turquesa
];

pub fn render_charts(
    ui: &mut Ui,
    history_points: &HashMap<String, Vec<(f64, f64)>>,
    active_hosts: &mut HashMap<String, bool>,
    t: &Translations,
) {
    ui.horizontal(|ui| {
        ui.heading(RichText::new(t.charts_title).strong());
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(RichText::new(t.charts_hint).weak());
        });
    });

    ui.separator();

    // Ordena os nomes de hosts para manter ordem e cores determinísticas entre frames
    let mut sorted_hosts: Vec<&String> = history_points.keys().collect();
    sorted_hosts.sort();

    // Filtros de seleção de hosts visíveis
    ui.horizontal_wrapped(|ui| {
        ui.label(RichText::new(t.charts_shown_hosts).strong());
        for host_name in &sorted_hosts {
            let is_active = active_hosts.entry((*host_name).clone()).or_insert(true);
            ui.checkbox(is_active, *host_name);
        }
    });

    ui.add_space(8.0);

    let plot = Plot::new("latency_realtime_plot")
        .legend(Legend::default())
        .y_axis_label(t.charts_y_axis)
        .x_axis_label(t.charts_x_axis)
        .height((ui.available_height() - 20.0).max(120.0));

    plot.show(ui, |plot_ui| {
        for (i, host_name) in sorted_hosts.iter().enumerate() {
            if !*active_hosts.get(*host_name).unwrap_or(&true) {
                continue;
            }
            if let Some(points) = history_points.get(*host_name) {
                let color = HOST_COLORS[i % HOST_COLORS.len()];
                let plot_points: PlotPoints = points.iter().map(|&(x, y)| [x, y]).collect();
                let line = Line::new((*host_name).clone(), plot_points)
                    .color(color)
                    .width(2.0);

                plot_ui.line(line);
            }
        }
    });
}
