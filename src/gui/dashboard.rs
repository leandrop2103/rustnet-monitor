use eframe::egui::{self, Color32, RichText, Ui};

use crate::config::settings::{DashboardConfig, LatencyThresholds};
use crate::i18n::Translations;
use crate::network::stats::{HostStats, LatencyQuality};
use crate::network::types::NetworkDiagnostic;
use crate::network::wan::{NatType, WanInfo};
use crate::network::wifi::WifiInfo;

/// Renderiza o botão discreto de minimizar/expandir no canto direito do cabeçalho do cartão.
fn render_collapse_button(ui: &mut Ui, expanded: &mut bool, hide_tip: &str, show_tip: &str) {
    let icon = if *expanded { "▲" } else { "▼" };
    let tip = if *expanded { hide_tip } else { show_tip };
    if ui
        .button(RichText::new(icon).size(13.0).strong())
        .on_hover_text(tip)
        .clicked()
    {
        *expanded = !*expanded;
    }
}

/// Renderiza a aba principal do Dashboard com rolagem vertical suave e cartões minimizáveis.
#[allow(clippy::too_many_arguments)]
pub fn render_dashboard(
    ui: &mut Ui,
    dashboard_cfg: &mut DashboardConfig,
    diagnostic: &NetworkDiagnostic,
    stats_list: &[HostStats],
    thresholds: &LatencyThresholds,
    wan_info: &Option<WanInfo>,
    wifi_info: &Option<WifiInfo>,
    t: &Translations,
) {
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing = egui::vec2(10.0, 10.0);

            // Barra Superior de Controles Rápidos do Dashboard
            ui.horizontal(|ui| {
                ui.label(RichText::new(t.dash_overview_title).heading().strong());
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .small_button(t.dash_expand_all)
                        .on_hover_text(t.dash_expand_all_tip)
                        .clicked()
                    {
                        dashboard_cfg.interface_card_expanded = true;
                        dashboard_cfg.wifi_card_expanded = true;
                        dashboard_cfg.wan_card_expanded = true;
                        dashboard_cfg.hosts_card_expanded = true;
                    }

                    if ui
                        .small_button(t.dash_collapse_all)
                        .on_hover_text(t.dash_collapse_all_tip)
                        .clicked()
                    {
                        dashboard_cfg.interface_card_expanded = false;
                        dashboard_cfg.wifi_card_expanded = false;
                        dashboard_cfg.wan_card_expanded = false;
                        dashboard_cfg.hosts_card_expanded = false;
                    }
                });
            });

            ui.add_space(2.0);

            // 1. Cartão de Informações da Interface Ativa
            egui::Frame::group(ui.style())
                .fill(ui.visuals().faint_bg_color)
                .inner_margin(egui::Margin::same(12))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("🖧").size(18.0));
                        ui.heading(RichText::new(t.dash_card_interface).strong());

                        if !dashboard_cfg.interface_card_expanded
                            && let Some(iface) = &diagnostic.active_physical_interface
                        {
                            ui.label(RichText::new(format!("({})", iface.friendly_name)).weak());
                        }

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            render_collapse_button(
                                ui,
                                &mut dashboard_cfg.interface_card_expanded,
                                t.dash_minimize_tip,
                                t.dash_expand_tip,
                            );

                            ui.add_space(8.0);

                            if let Some(iface) = &diagnostic.active_physical_interface {
                                let (color, text) = match iface.oper_status {
                                    crate::network::types::OperStatus::Up => {
                                        (Color32::from_rgb(46, 204, 113), t.dash_connected)
                                    }
                                    _ => (Color32::from_rgb(231, 76, 60), t.dash_disconnected),
                                };
                                ui.label(RichText::new(text).color(color).strong());
                            } else {
                                ui.label(
                                    RichText::new(t.dash_no_physical_iface)
                                        .color(Color32::from_rgb(231, 76, 60))
                                        .strong(),
                                );
                            }
                        });
                    });

                    if dashboard_cfg.interface_card_expanded {
                        ui.separator();

                        if let Some(iface) = &diagnostic.active_physical_interface {
                            egui::Grid::new("iface_grid")
                                .num_columns(4)
                                .spacing([24.0, 8.0])
                                .striped(false)
                                .show(ui, |ui| {
                                    ui.label(RichText::new(t.dash_adapter).strong());
                                    ui.label(&iface.friendly_name);

                                    ui.label(RichText::new(t.dash_description).strong());
                                    ui.label(&iface.description);
                                    ui.end_row();

                                    ui.label(RichText::new(t.dash_type).strong());
                                    ui.label(format!("{} {}", iface.if_type, t.dash_physical_suffix));

                                    ui.label(RichText::new(t.dash_link_speed).strong());
                                    ui.label(iface.formatted_speed());
                                    ui.end_row();

                                    ui.label(RichText::new(t.dash_ipv4).strong());
                                    ui.label(if iface.ipv4_addresses.is_empty() {
                                        t.dash_unassigned.to_string()
                                    } else {
                                        iface.ipv4_addresses.join(", ")
                                    });

                                    ui.label(RichText::new(t.dash_gateway).strong());
                                    ui.label(if iface.gateway_addresses.is_empty() {
                                        t.dash_none.to_string()
                                    } else {
                                        iface.gateway_addresses.join(", ")
                                    });
                                    ui.end_row();

                                    ui.label(RichText::new(t.dash_mac).strong());
                                    ui.label(iface.mac_address.as_deref().unwrap_or(t.dash_unavailable));

                                    ui.label(RichText::new(t.dash_dns).strong());
                                    ui.label(if iface.dns_addresses.is_empty() {
                                        t.dash_none.to_string()
                                    } else {
                                        iface.dns_addresses.join(", ")
                                    });
                                    ui.end_row();
                                });
                        } else {
                            ui.label(t.dash_no_physical_iface_msg);
                        }
                    }
                });

            // 1.1 Cartões Complementares de Wi-Fi e WAN/CGNAT em Colunas Paralelas
            let show_wifi = wifi_info.is_some();
            let show_wan = wan_info.is_some();

            if show_wifi || show_wan {
                ui.columns(if show_wifi && show_wan { 2 } else { 1 }, |cols| {
                    let mut col_idx = 0;

                    if let Some(wifi) = wifi_info {
                        cols[col_idx].group(|ui| {
                            ui.horizontal(|ui| {
                                ui.label(RichText::new("📶").size(18.0));
                                ui.heading(RichText::new(t.dash_card_wifi).strong());

                                if !dashboard_cfg.wifi_card_expanded {
                                    ui.label(RichText::new(format!("({})", wifi.ssid)).weak());
                                }

                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    render_collapse_button(
                                        ui,
                                        &mut dashboard_cfg.wifi_card_expanded,
                                        t.dash_minimize_tip,
                                        t.dash_expand_tip,
                                    );

                                    ui.add_space(8.0);

                                    let (badge_color, badge_label) = match wifi.signal_quality_pct {
                                        75..=100 => (Color32::from_rgb(46, 204, 113), t.dash_wifi_excellent),
                                        50..=74 => (Color32::from_rgb(52, 152, 219), t.dash_wifi_good),
                                        25..=49 => (Color32::from_rgb(241, 196, 15), t.dash_wifi_fair),
                                        _ => (Color32::from_rgb(231, 76, 60), t.dash_wifi_weak),
                                    };
                                    ui.label(
                                        RichText::new(format!("● {}", badge_label))
                                            .color(badge_color)
                                            .strong(),
                                    );
                                });
                            });

                            if dashboard_cfg.wifi_card_expanded {
                                ui.separator();

                                ui.horizontal(|ui| {
                                    ui.label(RichText::new(t.dash_wifi_ssid).strong());
                                    ui.label(RichText::new(&wifi.ssid).strong());
                                });
                                ui.horizontal(|ui| {
                                    ui.label(RichText::new(t.dash_wifi_bssid).strong());
                                    ui.label(RichText::new(&wifi.bssid).monospace());
                                });
                                ui.horizontal(|ui| {
                                    ui.label(RichText::new(t.dash_wifi_signal).strong());
                                    ui.label(format!("{}% ({} dBm)", wifi.signal_quality_pct, wifi.rssi_dbm));
                                    ui.label(
                                        RichText::new(format!("| {} {}", t.dash_wifi_standard, wifi.phy_type))
                                            .weak(),
                                    );
                                });
                                ui.horizontal(|ui| {
                                    ui.label(RichText::new(t.dash_wifi_rate).strong());
                                    ui.label(format!(
                                        "{:.0} Mbps (Tx) / {:.0} Mbps (Rx)",
                                        wifi.tx_rate_mbps, wifi.rx_rate_mbps
                                    ));
                                });
                            }
                        });
                        col_idx += 1;
                    }

                    if let Some(wan) = wan_info {
                        cols[col_idx].group(|ui| {
                            ui.horizontal(|ui| {
                                ui.label(RichText::new("🌍").size(18.0));
                                ui.heading(RichText::new(t.dash_card_wan).strong());

                                if !dashboard_cfg.wan_card_expanded
                                    && let Some(ref ip) = wan.public_ip
                                {
                                    ui.label(RichText::new(format!("({})", ip)).weak());
                                }

                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    render_collapse_button(
                                        ui,
                                        &mut dashboard_cfg.wan_card_expanded,
                                        t.dash_minimize_tip,
                                        t.dash_expand_tip,
                                    );

                                    ui.add_space(8.0);

                                    let (nat_color, nat_text) = match wan.nat_type {
                                        NatType::DirectPublicIp => {
                                            (Color32::from_rgb(46, 204, 113), t.dash_wan_public_ip_badge)
                                        }
                                        NatType::CgnatRfc6598 => {
                                            (Color32::from_rgb(243, 156, 18), t.dash_wan_cgnat_badge)
                                        }
                                        NatType::DoubleNatRfc1918 => {
                                            (Color32::from_rgb(52, 152, 219), t.dash_wan_private_nat_badge)
                                        }
                                        NatType::Unknown => (Color32::GRAY, t.dash_wan_na),
                                    };
                                    ui.label(
                                        RichText::new(format!("● {}", nat_text))
                                            .color(nat_color)
                                            .strong(),
                                    );
                                });
                            });

                            if dashboard_cfg.wan_card_expanded {
                                ui.separator();

                                ui.horizontal(|ui| {
                                    ui.label(RichText::new(t.dash_wan_external_ip).strong());
                                    ui.label(
                                        RichText::new(
                                            wan.public_ip.as_deref().unwrap_or(t.dash_wan_waiting),
                                        )
                                        .strong()
                                        .monospace(),
                                    );
                                });
                                ui.horizontal(|ui| {
                                    ui.label(RichText::new(t.dash_wan_isp).strong());
                                    ui.label(
                                        wan.isp_organization
                                            .as_deref()
                                            .unwrap_or(t.dash_wan_not_identified),
                                    );
                                });
                                ui.horizontal(|ui| {
                                    ui.label(RichText::new(t.dash_wan_location).strong());
                                    ui.label(wan.city_country.as_deref().unwrap_or(t.dash_wan_na));
                                });
                                ui.horizontal(|ui| {
                                    ui.label(RichText::new(t.dash_wan_topology).strong());
                                    let nat_desc = match wan.nat_type {
                                        NatType::DirectPublicIp => t.dash_wan_desc_direct,
                                        NatType::CgnatRfc6598 => t.dash_wan_desc_cgnat,
                                        NatType::DoubleNatRfc1918 => t.dash_wan_desc_double_nat,
                                        NatType::Unknown => t.dash_wan_desc_unknown,
                                    };
                                    ui.label(nat_desc);
                                });
                            }
                        });
                    }
                });
            }

            ui.add_space(4.0);

            // 2. Cartão da Tabela de Hosts Monitorados
            egui::Frame::group(ui.style())
                .fill(ui.visuals().panel_fill)
                .inner_margin(egui::Margin::same(10))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("📊").size(18.0));
                        ui.heading(RichText::new(t.dash_card_hosts).strong());

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            render_collapse_button(
                                ui,
                                &mut dashboard_cfg.hosts_card_expanded,
                                t.dash_minimize_table_tip,
                                t.dash_expand_table_tip,
                            );

                            ui.add_space(8.0);

                            let active_count = stats_list.len();
                            ui.label(
                                RichText::new(format!(
                                    "● {} {}",
                                    active_count, t.dash_active_hosts_badge
                                ))
                                .color(Color32::from_rgb(46, 204, 113))
                                .strong(),
                            );
                        });
                    });

                    if dashboard_cfg.hosts_card_expanded {
                        ui.separator();

                        if stats_list.is_empty() {
                            ui.add_space(10.0);
                            ui.label(RichText::new(t.dash_no_hosts_enabled).weak());
                            ui.add_space(10.0);
                        } else {
                            egui::ScrollArea::horizontal()
                                .auto_shrink([false, false])
                                .show(ui, |ui| {
                                    egui::Grid::new("hosts_table")
                                        .striped(true)
                                        .spacing([18.0, 10.0])
                                        .show(ui, |ui| {
                                            // Cabeçalho da Tabela
                                            ui.label(RichText::new(t.col_host).strong());
                                            ui.label(RichText::new(t.col_address).strong());
                                            ui.label(RichText::new(t.col_current_latency).strong());
                                            ui.label(RichText::new(t.col_avg).strong());
                                            ui.label(RichText::new(t.col_min).strong());
                                            ui.label(RichText::new(t.col_max).strong());
                                            ui.label(RichText::new(t.col_jitter).strong());
                                            ui.label(RichText::new(t.col_loss_pct).strong());
                                            ui.label(RichText::new(t.col_sent).strong());
                                            ui.label(RichText::new(t.col_status).strong());
                                            ui.end_row();

                                            for stat in stats_list {
                                                let quality = stat.quality(thresholds);
                                                let (badge_color, status_text) = match quality {
                                                    LatencyQuality::Good => {
                                                        (Color32::from_rgb(46, 204, 113), t.quality_good)
                                                    }
                                                    LatencyQuality::Fair => {
                                                        (Color32::from_rgb(241, 196, 15), t.quality_fair)
                                                    }
                                                    LatencyQuality::Poor => {
                                                        (Color32::from_rgb(230, 126, 34), t.quality_poor)
                                                    }
                                                    LatencyQuality::Offline => {
                                                        (Color32::from_rgb(231, 76, 60), t.quality_offline)
                                                    }
                                                };

                                                ui.label(RichText::new(&stat.name).strong());

                                                let addr_display = stat
                                                    .resolved_ip
                                                    .map(|ip| ip.to_string())
                                                    .unwrap_or_else(|| stat.target_str.clone());
                                                ui.label(addr_display);

                                                // Latência atual colorida
                                                if let Some(rtt) = stat.last_rtt_ms {
                                                    ui.label(
                                                        RichText::new(format!("{:.0} ms", rtt))
                                                            .color(badge_color)
                                                            .strong(),
                                                    );
                                                } else {
                                                    ui.label(RichText::new("-").color(Color32::GRAY));
                                                }

                                                ui.label(
                                                    stat.avg_rtt_ms
                                                        .map(|r| format!("{:.1} ms", r))
                                                        .unwrap_or_else(|| "-".to_string()),
                                                );

                                                ui.label(
                                                    stat.min_rtt_ms
                                                        .map(|r| format!("{:.0} ms", r))
                                                        .unwrap_or_else(|| "-".to_string()),
                                                );

                                                ui.label(
                                                    stat.max_rtt_ms
                                                        .map(|r| format!("{:.0} ms", r))
                                                        .unwrap_or_else(|| "-".to_string()),
                                                );

                                                ui.label(format!("{:.1} ms", stat.jitter_ms));

                                                let loss_color = if stat.packet_loss_pct > 5.0 {
                                                    Color32::from_rgb(231, 76, 60)
                                                } else if stat.packet_loss_pct > 0.0 {
                                                    Color32::from_rgb(241, 196, 15)
                                                } else {
                                                    ui.visuals().text_color()
                                                };
                                                ui.label(
                                                    RichText::new(format!(
                                                        "{:.1}%",
                                                        stat.packet_loss_pct
                                                    ))
                                                    .color(loss_color),
                                                );

                                                ui.label(stat.sent_packets.to_string());

                                                ui.label(
                                                    RichText::new(status_text)
                                                        .color(badge_color)
                                                        .strong(),
                                                );
                                                ui.end_row();
                                            }
                                        });
                                });
                        }
                    }
                });
        });
}
