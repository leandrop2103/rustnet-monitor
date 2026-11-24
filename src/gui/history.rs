use eframe::egui::{self, Color32, RichText, Ui};
use rusqlite::{Connection, params};
use std::path::Path;

use crate::database::connection::open_optimized_connection;
use crate::database::models::{HostRecord, LatencySampleRecord};
use crate::database::repository::get_all_hosts;
use crate::i18n::Translations;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimeFilter {
    Last5Minutes,
    Last15Minutes,
    LastHour,
    Last6Hours,
    Last24Hours,
    Last7Days,
    All,
}

impl TimeFilter {
    pub fn sql_modifier(&self) -> Option<&'static str> {
        match self {
            TimeFilter::Last5Minutes => Some("-5 minutes"),
            TimeFilter::Last15Minutes => Some("-15 minutes"),
            TimeFilter::LastHour => Some("-1 hours"),
            TimeFilter::Last6Hours => Some("-6 hours"),
            TimeFilter::Last24Hours => Some("-24 hours"),
            TimeFilter::Last7Days => Some("-7 days"),
            TimeFilter::All => None,
        }
    }

    pub fn label(&self, t: &Translations) -> &'static str {
        match self {
            TimeFilter::Last5Minutes => t.history_filter_5m,
            TimeFilter::Last15Minutes => t.history_filter_15m,
            TimeFilter::LastHour => t.history_filter_1h,
            TimeFilter::Last6Hours => t.history_filter_6h,
            TimeFilter::Last24Hours => t.history_filter_24h,
            TimeFilter::Last7Days => t.history_filter_7d,
            TimeFilter::All => t.history_filter_all,
        }
    }
}

pub struct HistoryState {
    pub selected_filter: TimeFilter,
    pub selected_host_id: Option<i64>,
    pub cached_hosts: Vec<HostRecord>,
    pub cached_samples: Vec<LatencySampleRecord>,
    pub last_query_time: Option<std::time::Instant>,
    pub last_error: Option<String>,
}

impl Default for HistoryState {
    fn default() -> Self {
        Self {
            selected_filter: TimeFilter::Last15Minutes,
            selected_host_id: None,
            cached_hosts: Vec::new(),
            cached_samples: Vec::new(),
            last_query_time: None,
            last_error: None,
        }
    }
}

pub fn render_history(ui: &mut Ui, state: &mut HistoryState, db_path: &Path, t: &Translations) {
    ui.heading(RichText::new(t.history_title).strong());
    ui.separator();

    // Carrega lista de hosts em cache na primeira abertura (evita abrir SQLite a cada frame)
    let mut need_refresh = state.last_query_time.is_none();

    if need_refresh {
        match open_optimized_connection(db_path) {
            Ok(conn) => {
                state.last_error = None;
                state.cached_hosts = get_all_hosts(&conn).unwrap_or_default();
                if state.selected_host_id.is_none() && !state.cached_hosts.is_empty() {
                    state.selected_host_id = Some(state.cached_hosts[0].id);
                }
            }
            Err(e) => {
                state.last_error = Some(format!("{} {}", t.history_db_err_prefix, e));
            }
        }
    }

    if let Some(ref err) = state.last_error {
        ui.label(RichText::new(err).color(Color32::from_rgb(231, 76, 60)));
        if ui.button(t.history_retry).clicked() {
            state.last_query_time = None;
        }
        return;
    }

    // Controles de Filtro
    ui.horizontal(|ui| {
        ui.label(RichText::new(t.history_host).strong());
        let current_host_name = state
            .cached_hosts
            .iter()
            .find(|h| Some(h.id) == state.selected_host_id)
            .map(|h| h.name.as_str())
            .unwrap_or(t.history_select);

        egui::ComboBox::from_id_salt("history_host_combo")
            .selected_text(current_host_name)
            .show_ui(ui, |ui| {
                for host in &state.cached_hosts {
                    let is_selected = state.selected_host_id == Some(host.id);
                    if ui.selectable_label(is_selected, &host.name).clicked() {
                        state.selected_host_id = Some(host.id);
                        need_refresh = true;
                    }
                }
            });

        ui.add_space(16.0);
        ui.label(RichText::new(t.history_period).strong());
        egui::ComboBox::from_id_salt("history_time_combo")
            .selected_text(state.selected_filter.label(t))
            .show_ui(ui, |ui| {
                let filters = [
                    TimeFilter::Last5Minutes,
                    TimeFilter::Last15Minutes,
                    TimeFilter::LastHour,
                    TimeFilter::Last6Hours,
                    TimeFilter::Last24Hours,
                    TimeFilter::Last7Days,
                    TimeFilter::All,
                ];
                for f in filters {
                    let is_selected = state.selected_filter == f;
                    if ui.selectable_label(is_selected, f.label(t)).clicked() {
                        state.selected_filter = f;
                        need_refresh = true;
                    }
                }
            });

        if ui.button(t.history_refresh).clicked() {
            need_refresh = true;
        }
    });

    // Atualização da consulta apenas sob demanda (ou na primeira abertura)
    if need_refresh {
        match open_optimized_connection(db_path) {
            Ok(conn) => {
                state.last_error = None;
                state.cached_hosts = get_all_hosts(&conn).unwrap_or_default();
                if state.selected_host_id.is_none() && !state.cached_hosts.is_empty() {
                    state.selected_host_id = Some(state.cached_hosts[0].id);
                }
                if let Some(host_id) = state.selected_host_id {
                    state.cached_samples =
                        query_samples(&conn, host_id, state.selected_filter).unwrap_or_default();
                } else {
                    state.cached_samples.clear();
                }
                state.last_query_time = Some(std::time::Instant::now());
            }
            Err(e) => {
                state.last_error = Some(format!("{} {}", t.history_db_err_prefix, e));
            }
        }
    }

    ui.add_space(8.0);

    // Resumo Estatístico do Período
    let total_samples = state.cached_samples.len();
    let success_samples = state.cached_samples.iter().filter(|s| s.success).count();
    let lost_samples = total_samples - success_samples;
    let loss_pct = if total_samples > 0 {
        (lost_samples as f64 / total_samples as f64) * 100.0
    } else {
        0.0
    };

    let (min_rtt, max_rtt, avg_rtt) = if success_samples > 0 {
        let mut min = f64::MAX;
        let mut max = f64::MIN;
        let mut sum = 0.0;
        for s in state.cached_samples.iter().filter(|s| s.success) {
            min = min.min(s.latency_ms);
            max = max.max(s.latency_ms);
            sum += s.latency_ms;
        }
        (min, max, sum / success_samples as f64)
    } else {
        (0.0, 0.0, 0.0)
    };

    egui::Frame::group(ui.style())
        .fill(ui.visuals().faint_bg_color)
        .inner_margin(egui::Margin::same(10))
        .show(ui, |ui| {
            egui::Grid::new("history_summary_grid")
                .num_columns(5)
                .spacing([28.0, 6.0])
                .show(ui, |ui| {
                    ui.label(RichText::new(t.history_samples).strong());
                    ui.label(RichText::new(t.history_avg_latency).strong());
                    ui.label(RichText::new(t.history_min_latency).strong());
                    ui.label(RichText::new(t.history_max_latency).strong());
                    ui.label(RichText::new(t.history_packet_loss).strong());
                    ui.end_row();

                    ui.label(format!("{} {}", total_samples, t.history_recorded_suffix));
                    ui.label(format!("{:.1} ms", avg_rtt));
                    ui.label(format!("{:.0} ms", min_rtt));
                    ui.label(format!("{:.0} ms", max_rtt));
                    let loss_color = if loss_pct > 0.0 {
                        Color32::from_rgb(231, 76, 60)
                    } else {
                        Color32::from_rgb(46, 204, 113)
                    };
                    ui.label(
                        RichText::new(format!(
                            "{:.1}% ({} {})",
                            loss_pct, lost_samples, t.history_lost_suffix
                        ))
                        .color(loss_color),
                    );
                    ui.end_row();
                });
        });

    ui.add_space(8.0);

    // Tabela de Amostras
    ui.heading(RichText::new(t.history_records_title).strong());

    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            egui::Grid::new("history_samples_table")
                .striped(true)
                .spacing([24.0, 6.0])
                .show(ui, |ui| {
                    ui.label(RichText::new(t.history_col_datetime).strong());
                    ui.label(RichText::new(t.history_col_latency).strong());
                    ui.label(RichText::new(t.history_col_status).strong());
                    ui.label(RichText::new(t.history_col_detail).strong());
                    ui.end_row();

                    for sample in state.cached_samples.iter().rev().take(200) {
                        ui.label(&sample.timestamp);
                        if sample.success {
                            ui.label(format!("{:.1} ms", sample.latency_ms));
                            ui.label(
                                RichText::new(t.history_status_ok)
                                    .color(Color32::from_rgb(46, 204, 113)),
                            );
                        } else {
                            ui.label(RichText::new("-").weak());
                            ui.label(
                                RichText::new(t.history_status_err)
                                    .color(Color32::from_rgb(231, 76, 60))
                                    .strong(),
                            );
                        }
                        ui.label(sample.error_type.as_deref().unwrap_or("-"));
                        ui.end_row();
                    }
                });
        });
}

fn query_samples(
    conn: &Connection,
    host_id: i64,
    filter: TimeFilter,
) -> Result<Vec<LatencySampleRecord>, rusqlite::Error> {
    let mut samples = Vec::new();
    let map_row = |row: &rusqlite::Row<'_>| {
        Ok(LatencySampleRecord {
            id: Some(row.get(0)?),
            host_id: row.get(1)?,
            timestamp: row.get(2)?,
            latency_ms: row.get(3)?,
            success: row.get::<_, i32>(4)? != 0,
            error_type: row.get(5)?,
        })
    };

    if let Some(modifier) = filter.sql_modifier() {
        let mut stmt = conn.prepare(
            "SELECT id, host_id, timestamp, latency_ms, success, error_type
             FROM latency_samples
             WHERE host_id = ?1 AND timestamp >= strftime('%Y-%m-%dT%H:%M:%SZ', 'now', ?2)
             ORDER BY timestamp ASC",
        )?;
        let iter = stmt.query_map(params![host_id, modifier], map_row)?;
        for sample in iter {
            samples.push(sample?);
        }
    } else {
        let mut stmt = conn.prepare(
            "SELECT id, host_id, timestamp, latency_ms, success, error_type
             FROM latency_samples
             WHERE host_id = ?1
             ORDER BY timestamp ASC",
        )?;
        let iter = stmt.query_map(params![host_id], map_row)?;
        for sample in iter {
            samples.push(sample?);
        }
    }

    Ok(samples)
}
