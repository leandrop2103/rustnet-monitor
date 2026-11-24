use eframe::egui::{self, Color32, RichText, Ui};
use std::path::Path;
use std::sync::{Arc, Mutex};

use crate::config::settings::{AppConfig, HostConfig};
use crate::i18n::{Language, Translations};
use crate::updater::{RemoteVersionInfo, UpdateStatus};

pub struct SettingsState {
    pub new_host_name: String,
    pub new_host_address: String,
    pub status_message: Option<(String, bool)>, // (mensagem, is_error)
    pub show_reset_confirmation: bool,
    pub clear_db_history: bool,
    pub request_reset_metrics: bool,
    pub hosts_changed: bool,
}

impl Default for SettingsState {
    fn default() -> Self {
        Self {
            new_host_name: String::new(),
            new_host_address: String::new(),
            status_message: None,
            show_reset_confirmation: false,
            clear_db_history: true,
            request_reset_metrics: false,
            hosts_changed: false,
        }
    }
}

pub fn render_settings(
    ui: &mut Ui,
    config: &mut AppConfig,
    state: &mut SettingsState,
    config_path: &Path,
    available_update: &Arc<Mutex<Option<RemoteVersionInfo>>>,
    update_status: &Arc<Mutex<UpdateStatus>>,
    t: &Translations,
) {
    ui.heading(RichText::new(t.settings_title).strong());
    ui.separator();

    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            // 1. Configurações Gerais e Aparência
            ui.group(|ui| {
                ui.heading(RichText::new(t.settings_general_title).strong());
                ui.add_space(4.0);

                ui.horizontal(|ui| {
                    ui.label(RichText::new(t.settings_theme_label).strong());
                    ui.radio_value(
                        &mut config.general.theme,
                        "dark".to_string(),
                        t.settings_theme_dark,
                    );
                    ui.radio_value(
                        &mut config.general.theme,
                        "light".to_string(),
                        t.settings_theme_light,
                    );
                    ui.radio_value(
                        &mut config.general.theme,
                        "system".to_string(),
                        t.settings_theme_system,
                    );
                });

                // Seletor de Idioma (troca instantânea em tempo real e persistência automática)
                ui.horizontal(|ui| {
                    ui.label(RichText::new(t.settings_language_label).strong());
                    let current_lang = Language::from_code(&config.general.language);
                    let mut selected_lang = current_lang;

                    ui.radio_value(&mut selected_lang, Language::PtBr, "🇧🇷 Português (BR)");
                    ui.radio_value(&mut selected_lang, Language::En, "🇺🇸 English");
                    ui.radio_value(&mut selected_lang, Language::Es, "🇪🇸 Español");

                    if selected_lang != current_lang {
                        config.general.language = selected_lang.code().to_string();
                        let _ = config.save(config_path);
                    }
                });

                ui.horizontal(|ui| {
                    ui.checkbox(&mut config.general.minimize_to_tray, t.settings_minimize_tray);
                    ui.checkbox(
                        &mut config.general.start_with_windows,
                        t.settings_start_windows,
                    );
                    ui.checkbox(&mut config.general.sound_alerts, t.settings_sound_alerts);
                });
            });

            ui.add_space(8.0);

            // 2. Parâmetros de Monitoramento
            ui.group(|ui| {
                ui.heading(RichText::new(t.settings_monitoring_title).strong());
                ui.add_space(4.0);

                ui.horizontal(|ui| {
                    ui.label(RichText::new(t.settings_interval_label).strong());
                    let intervals = [1, 2, 5, 10, 30, 60];
                    for sec in intervals {
                        ui.radio_value(
                            &mut config.monitoring.interval_secs,
                            sec,
                            format!("{}s", sec),
                        );
                    }
                });

                ui.horizontal(|ui| {
                    ui.label(RichText::new(t.settings_timeout_label).strong());
                    let timeouts = [500, 1000, 2000, 3000];
                    for ms in timeouts {
                        ui.radio_value(&mut config.monitoring.timeout_ms, ms, format!("{} ms", ms));
                    }
                });
            });

            ui.add_space(8.0);

            // 3. Limites de Classificação de Latência
            ui.group(|ui| {
                ui.heading(RichText::new(t.settings_thresholds_title).strong());
                ui.add_space(4.0);

                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(t.settings_green_limit)
                            .color(Color32::from_rgb(46, 204, 113))
                            .strong(),
                    );
                    ui.add(
                        egui::DragValue::new(&mut config.thresholds.green_max_ms)
                            .range(5..=200)
                            .suffix(" ms"),
                    );

                    ui.add_space(20.0);
                    ui.label(
                        RichText::new(t.settings_yellow_limit)
                            .color(Color32::from_rgb(241, 196, 15))
                            .strong(),
                    );
                    ui.add(
                        egui::DragValue::new(&mut config.thresholds.yellow_max_ms)
                            .range((config.thresholds.green_max_ms + 5)..=500)
                            .suffix(" ms"),
                    );
                    if config.thresholds.yellow_max_ms <= config.thresholds.green_max_ms {
                        config.thresholds.yellow_max_ms = config.thresholds.green_max_ms + 5;
                    }
                });
                ui.label(RichText::new(t.settings_thresholds_hint).weak());
            });

            ui.add_space(8.0);

            // 4. Retenção do Banco de Dados
            ui.group(|ui| {
                ui.heading(RichText::new(t.settings_persistence_title).strong());
                ui.add_space(4.0);

                ui.horizontal(|ui| {
                    ui.label(RichText::new(t.settings_retention_label).strong());
                    let retentions = [
                        (7, t.settings_ret_7d),
                        (30, t.settings_ret_30d),
                        (90, t.settings_ret_90d),
                        (365, t.settings_ret_1y),
                        (0, t.settings_ret_unlimited),
                    ];
                    for (days, label) in retentions {
                        ui.radio_value(&mut config.database.retention_days, days, label);
                    }
                });
            });

            ui.add_space(8.0);

            // 5. Gerenciamento de Hosts Monitorados
            ui.group(|ui| {
                ui.heading(RichText::new(t.settings_hosts_title).strong());
                ui.add_space(4.0);

                let mut to_remove = None;
                let mut host_toggled = false;
                for (i, host) in config.hosts.iter_mut().enumerate() {
                    ui.horizontal(|ui| {
                        if ui.checkbox(&mut host.enabled, "").changed() {
                            host_toggled = true;
                        }
                        ui.label(RichText::new(&host.name).strong());
                        ui.label(format!("({})", host.address));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if !host.is_dynamic_gateway {
                                if ui
                                    .button(
                                        RichText::new(t.settings_remove)
                                            .color(Color32::from_rgb(231, 76, 60)),
                                    )
                                    .clicked()
                                {
                                    to_remove = Some(i);
                                }
                            } else {
                                ui.label(RichText::new(t.settings_dynamic_default).weak());
                            }
                        });
                    });
                }

                if host_toggled {
                    state.hosts_changed = true;
                    let _ = config.save(config_path);
                }

                if let Some(idx) = to_remove {
                    config.hosts.remove(idx);
                    state.hosts_changed = true;
                    if let Err(e) = config.save(config_path) {
                        state.status_message = Some((
                            format!("{} {}", t.settings_msg_host_removed_err, e),
                            true,
                        ));
                    } else {
                        state.status_message = Some((t.settings_msg_host_removed.to_string(), false));
                    }
                }

                ui.separator();
                ui.label(RichText::new(t.settings_add_host_title).strong());
                ui.horizontal(|ui| {
                    ui.label(t.settings_host_name);
                    ui.text_edit_singleline(&mut state.new_host_name);
                    ui.label(t.settings_host_addr);
                    ui.text_edit_singleline(&mut state.new_host_address);

                    if ui.button(t.settings_add_btn).clicked() {
                        let name = state.new_host_name.trim();
                        let addr = state.new_host_address.trim();
                        if name.is_empty() || addr.is_empty() {
                            state.status_message =
                                Some((t.settings_msg_fill_fields.to_string(), true));
                        } else if config
                            .hosts
                            .iter()
                            .any(|h| h.name.eq_ignore_ascii_case(name))
                        {
                            state.status_message = Some((
                                format!("{} '{}'.", t.settings_msg_duplicate_host, name),
                                true,
                            ));
                        } else {
                            config.hosts.push(HostConfig {
                                name: name.to_string(),
                                address: addr.to_string(),
                                host_type: "custom".to_string(),
                                enabled: true,
                                is_dynamic_gateway: false,
                            });
                            state.new_host_name.clear();
                            state.new_host_address.clear();
                            state.hosts_changed = true;

                            if let Err(e) = config.save(config_path) {
                                state.status_message = Some((
                                    format!("{} {}", t.settings_msg_host_added_err, e),
                                    true,
                                ));
                            } else {
                                state.status_message =
                                    Some((t.settings_msg_host_added.to_string(), false));
                            }
                        }
                    }
                });
            });

            ui.add_space(8.0);

            // 6. Manutenção de Dados e Reinicialização de Métricas
            ui.group(|ui| {
                ui.heading(RichText::new(t.settings_maintenance_title).strong());
                ui.add_space(4.0);

                if !state.show_reset_confirmation {
                    ui.horizontal(|ui| {
                        if ui
                            .button(
                                RichText::new(t.settings_reset_btn)
                                    .color(Color32::from_rgb(231, 76, 60))
                                    .strong(),
                            )
                            .clicked()
                        {
                            state.show_reset_confirmation = true;
                        }

                        ui.label(RichText::new(t.settings_reset_hint).weak());
                    });
                } else {
                    egui::Frame::group(ui.style())
                        .fill(Color32::from_rgba_unmultiplied(231, 76, 60, 20))
                        .stroke(egui::Stroke::new(1.0, Color32::from_rgb(231, 76, 60)))
                        .inner_margin(egui::Margin::same(12))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.label(RichText::new("⚠️").size(24.0));
                                ui.vertical(|ui| {
                                    ui.label(
                                        RichText::new(t.settings_reset_confirm_title)
                                            .size(16.0)
                                            .color(Color32::from_rgb(231, 76, 60))
                                            .strong(),
                                    );
                                    ui.label(t.settings_reset_confirm_q1);
                                    ui.label(t.settings_reset_confirm_q2);
                                    ui.add_space(4.0);
                                    ui.checkbox(
                                        &mut state.clear_db_history,
                                        t.settings_reset_clear_db,
                                    );
                                });
                            });

                            ui.add_space(8.0);
                            ui.horizontal(|ui| {
                                if ui
                                    .add(
                                        egui::Button::new(
                                            RichText::new(t.settings_reset_confirm_yes)
                                                .color(Color32::WHITE)
                                                .strong(),
                                        )
                                        .fill(Color32::from_rgb(192, 57, 43)),
                                    )
                                    .clicked()
                                {
                                    state.request_reset_metrics = true;
                                    state.show_reset_confirmation = false;
                                }

                                if ui.button(t.settings_cancel).clicked() {
                                    state.show_reset_confirmation = false;
                                }
                            });
                        });
                }
            });

            ui.add_space(8.0);

            // 7. Atualização Automática do Aplicativo
            ui.group(|ui| {
                ui.heading(RichText::new(t.settings_updates_title).strong());
                ui.add_space(4.0);

                let current_version = env!("CARGO_PKG_VERSION");
                let status = update_status
                    .lock()
                    .map(|s| s.clone())
                    .unwrap_or(crate::updater::UpdateStatus::Idle);
                let opt_info = available_update
                    .lock()
                    .map(|u| u.clone())
                    .unwrap_or(None);

                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(format!(
                            "{} v{}",
                            t.settings_installed_version, current_version
                        ))
                        .strong(),
                    );
                    ui.label(
                        RichText::new(format!(
                            "({} {})",
                            t.settings_repository,
                            crate::updater::GITHUB_REPO
                        ))
                        .weak(),
                    );
                });

                ui.add_space(4.0);

                match &status {
                    crate::updater::UpdateStatus::Idle => {
                        if opt_info.is_none()
                            && ui
                                .button(RichText::new(t.settings_check_updates).strong())
                                .clicked()
                        {
                            crate::updater::spawn_update_check(
                                available_update.clone(),
                                update_status.clone(),
                            );
                        }
                    }
                    crate::updater::UpdateStatus::Checking => {
                        ui.horizontal(|ui| {
                            ui.spinner();
                            ui.label(
                                RichText::new(t.settings_checking_updates)
                                    .color(Color32::from_rgb(52, 152, 219)),
                            );
                        });
                    }
                    crate::updater::UpdateStatus::UpToDate => {
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new(t.settings_up_to_date)
                                    .color(Color32::from_rgb(46, 204, 113))
                                    .strong(),
                            );
                            if ui.button(t.settings_check_again).clicked() {
                                crate::updater::spawn_update_check(
                                    available_update.clone(),
                                    update_status.clone(),
                                );
                            }
                        });
                    }
                    crate::updater::UpdateStatus::Downloading(p) => {
                        ui.vertical(|ui| {
                            ui.label(
                                RichText::new(format!(
                                    "{} {:.0}%",
                                    t.settings_downloading,
                                    p * 100.0
                                ))
                                .strong(),
                            );
                            ui.add(egui::ProgressBar::new(*p).show_percentage());
                        });
                    }
                    crate::updater::UpdateStatus::Success(msg) => {
                        ui.label(
                            RichText::new(format!("🎉 {}", msg))
                                .color(Color32::from_rgb(46, 204, 113))
                                .strong(),
                        );
                    }
                    crate::updater::UpdateStatus::Error(err) => {
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new(format!("❌ {}", err))
                                    .color(Color32::from_rgb(231, 76, 60))
                                    .strong(),
                            );
                            if ui.button(t.settings_try_again).clicked()
                                && let Ok(mut st) = update_status.lock()
                            {
                                *st = crate::updater::UpdateStatus::Idle;
                            }
                        });
                    }
                }

                if let Some(info) = opt_info {
                    ui.add_space(6.0);
                    egui::Frame::group(ui.style())
                        .fill(Color32::from_rgba_unmultiplied(46, 204, 113, 25))
                        .stroke(egui::Stroke::new(1.0, Color32::from_rgb(46, 204, 113)))
                        .inner_margin(egui::Margin::same(12))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.label(RichText::new("🚀").size(24.0));
                                ui.vertical(|ui| {
                                    ui.label(
                                        RichText::new(format!(
                                            "{}{}{}",
                                            t.settings_new_version_prefix,
                                            info.version,
                                            t.settings_new_version_suffix
                                        ))
                                        .size(16.0)
                                        .color(Color32::from_rgb(46, 204, 113))
                                        .strong(),
                                    );
                                    if !info.release_notes.is_empty() {
                                        ui.label(
                                            RichText::new(format!(
                                                "{} {}",
                                                t.settings_release_notes, info.release_notes
                                            ))
                                            .weak(),
                                        );
                                    }
                                });
                            });

                            ui.add_space(8.0);
                            if matches!(status, crate::updater::UpdateStatus::Downloading(_)) {
                                // Já está realizando o download
                            } else if ui
                                .add(
                                    egui::Button::new(
                                        RichText::new(t.settings_update_now)
                                            .color(Color32::WHITE)
                                            .strong(),
                                    )
                                    .fill(Color32::from_rgb(46, 204, 113)),
                                )
                                .clicked()
                            {
                                let dl_url = info.download_url.clone();
                                let status_cb_clone = update_status.clone();
                                std::thread::spawn(move || {
                                    let status_inner = status_cb_clone.clone();
                                    let result = crate::updater::perform_auto_update(
                                        Some(dl_url),
                                        move |st| {
                                            if let Ok(mut guard) = status_inner.lock() {
                                                *guard = st;
                                            }
                                        },
                                    );
                                    if let Err(e) = result
                                        && let Ok(mut guard) = status_cb_clone.lock()
                                    {
                                        *guard = crate::updater::UpdateStatus::Error(e);
                                    }
                                });
                            }
                        });
                }
            });

            ui.add_space(12.0);

            // Botão de Salvar
            ui.horizontal(|ui| {
                if ui.button(RichText::new(t.settings_save_btn).strong()).clicked() {
                    match config.save(config_path) {
                        Ok(()) => {
                            state.hosts_changed = true;
                            state.status_message = Some((t.settings_saved_ok.to_string(), false));
                        }
                        Err(e) => {
                            state.status_message =
                                Some((format!("{} {}", t.settings_save_err, e), true));
                        }
                    }
                }

                if let Some((msg, is_error)) = &state.status_message {
                    let color = if *is_error {
                        Color32::from_rgb(231, 76, 60)
                    } else {
                        Color32::from_rgb(46, 204, 113)
                    };
                    ui.label(RichText::new(msg).color(color).strong());
                }
            });
        });
}
