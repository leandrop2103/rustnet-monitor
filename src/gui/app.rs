use eframe::egui::{self, Color32, RichText};
use std::collections::HashMap;
use std::net::Ipv4Addr;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::config::AppConfig;
use crate::database::DatabaseHandle;
use crate::gui::charts::render_charts;
use crate::gui::dashboard::render_dashboard;
use crate::gui::events::{EventsState, render_events};
use crate::gui::history::{HistoryState, render_history};
use crate::gui::settings::{SettingsState, render_settings};
use crate::gui::tools::{ToolsState, render_tools};
use crate::i18n::Language;
use crate::network::alerts::AlertManager;
use crate::network::icmp::PingStatus;
use crate::network::pinger::{PingUpdate, PingerCommand, PingerService};
use crate::network::stats::HostStats;
use crate::network::types::NetworkDiagnostic;
use crate::network::wan::WanInfo;
use crate::network::wifi::WifiInfo;
use crate::updater::{RemoteVersionInfo, UpdateStatus};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActiveTab {
    Dashboard,
    Charts,
    History,
    Events,
    Tools,
    Settings,
}

pub struct RustNetApp {
    config: AppConfig,
    config_path: PathBuf,
    diagnostic: NetworkDiagnostic,
    active_tab: ActiveTab,
    stats_list: Vec<HostStats>,
    history_points: HashMap<String, Vec<(f64, f64)>>, // Host -> [(segundos_decorridos, rtt_ms)]
    active_hosts_in_chart: HashMap<String, bool>,
    start_time: Instant,
    last_update_time: Option<Instant>,
    rx_updates: Option<Receiver<PingUpdate>>,
    tx_pinger_cmd: Option<Sender<PingerCommand>>,
    stop_signal: Option<Arc<AtomicBool>>,
    db_handle: Option<DatabaseHandle>,
    history_state: HistoryState,
    events_state: EventsState,
    settings_state: SettingsState,
    tools_state: ToolsState,
    wan_info: Arc<Mutex<Option<WanInfo>>>,
    wifi_info: Arc<Mutex<Option<WifiInfo>>>,
    alert_manager: AlertManager,
    logo_texture: Option<egui::TextureHandle>,
    available_update: Arc<Mutex<Option<RemoteVersionInfo>>>,
    update_status: Arc<Mutex<UpdateStatus>>,
    applied_theme: String,
}

impl RustNetApp {
    pub fn new(
        config: AppConfig,
        config_path: PathBuf,
        diagnostic: NetworkDiagnostic,
        db_handle: Option<DatabaseHandle>,
    ) -> Self {
        let detected_gateway = diagnostic
            .default_route_ipv4
            .as_ref()
            .and_then(|r| Ipv4Addr::from_str(&r.next_hop).ok())
            .or_else(|| {
                diagnostic
                    .active_physical_interface
                    .as_ref()
                    .and_then(|iface| iface.gateway_addresses.first())
                    .and_then(|gw| Ipv4Addr::from_str(gw).ok())
            });

        let pinger = PingerService::new(config.clone(), detected_gateway);
        let initial_stats = pinger.current_stats();
        let (rx, tx_cmd, stop_signal, _worker) = pinger.start_worker();

        let mut history_points = HashMap::new();
        let mut active_hosts_in_chart = HashMap::new();
        for stat in &initial_stats {
            history_points.insert(stat.name.clone(), Vec::new());
            active_hosts_in_chart.insert(stat.name.clone(), true);
        }

        let available_update = Arc::new(Mutex::new(None));
        let update_status = Arc::new(Mutex::new(UpdateStatus::Idle));
        let wan_info = Arc::new(Mutex::new(None));
        let wifi_info = Arc::new(Mutex::new(None));
        let alert_manager = AlertManager::new(config.general.sound_alerts);

        // Inicia verificação assíncrona de atualização no GitHub em background
        crate::updater::spawn_update_check(available_update.clone(), update_status.clone());

        // Inicia consulta assíncrona de Wi-Fi e WAN/CGNAT em background
        let wan_clone = wan_info.clone();
        let wifi_clone = wifi_info.clone();
        let local_ip = diagnostic
            .active_physical_interface
            .as_ref()
            .and_then(|i| i.ipv4_addresses.first())
            .and_then(|ip_str| Ipv4Addr::from_str(ip_str).ok());

        std::thread::spawn(move || {
            let wifi = crate::network::wifi::query_wifi_telemetry();
            if let Ok(mut guard) = wifi_clone.lock() {
                *guard = wifi;
            }

            let wan = crate::network::wan::query_wan_info(local_ip);
            if let Ok(mut guard) = wan_clone.lock() {
                *guard = Some(wan);
            }
        });

        Self {
            config,
            config_path,
            diagnostic,
            active_tab: ActiveTab::Dashboard,
            stats_list: initial_stats,
            history_points,
            active_hosts_in_chart,
            start_time: Instant::now(),
            last_update_time: None,
            rx_updates: Some(rx),
            tx_pinger_cmd: Some(tx_cmd),
            stop_signal: Some(stop_signal),
            db_handle,
            history_state: HistoryState::default(),
            events_state: EventsState::default(),
            settings_state: SettingsState::default(),
            tools_state: ToolsState::default(),
            wan_info,
            wifi_info,
            alert_manager,
            logo_texture: None,
            available_update,
            update_status,
            applied_theme: String::new(),
        }
    }

    fn base_dir(&self) -> &Path {
        self.config_path.parent().unwrap_or(Path::new("."))
    }

    fn resolved_db_path(&self) -> PathBuf {
        if Path::new(&self.config.database.path).is_relative() {
            self.base_dir().join(&self.config.database.path)
        } else {
            PathBuf::from(&self.config.database.path)
        }
    }

    /// Sincroniza em tempo real as configurações e a lista de hosts com o Dashboard e o Pinger.
    pub fn sync_hosts_and_config(&mut self) {
        // 1. Atualiza lista local de stats_list preservando métricas de hosts existentes
        let mut updated_stats = Vec::new();
        for host_cfg in &self.config.hosts {
            if host_cfg.enabled {
                if let Some(existing) = self
                    .stats_list
                    .iter()
                    .find(|s| s.name == host_cfg.name && s.target_str == host_cfg.address)
                {
                    updated_stats.push(existing.clone());
                } else {
                    let new_stat = HostStats::new(host_cfg.name.clone(), host_cfg.address.clone());
                    self.history_points
                        .entry(host_cfg.name.clone())
                        .or_default();
                    self.active_hosts_in_chart
                        .entry(host_cfg.name.clone())
                        .or_insert(true);
                    updated_stats.push(new_stat);
                }
            }
        }
        self.stats_list = updated_stats;
        self.history_points
            .retain(|name, _| self.config.hosts.iter().any(|h| h.enabled && &h.name == name));
        self.active_hosts_in_chart
            .retain(|name, _| self.config.hosts.iter().any(|h| h.enabled && &h.name == name));

        // 2. Notifica o pinger worker em background para aplicar imediatamente a nova lista
        if let Some(ref tx_cmd) = self.tx_pinger_cmd {
            let _ = tx_cmd.send(PingerCommand::UpdateConfig(self.config.clone()));
        }

        // 3. Garante a persistência imediata em disco no config.toml
        let _ = self.config.save(&self.config_path);
    }
}

impl Drop for RustNetApp {
    fn drop(&mut self) {
        let _ = self.config.save(&self.config_path);
        self.tools_state.mtr_running.store(false, Ordering::Relaxed);
        if let Some(ref stop) = self.stop_signal {
            stop.store(true, Ordering::Relaxed);
        }
        if let Some(ref db) = self.db_handle {
            db.stop();
        }
    }
}

impl eframe::App for RustNetApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();

        // 1. Processa amostras assíncronas recebidas do pinger worker
        if let Some(ref rx) = self.rx_updates {
            while let Ok(update) = rx.try_recv() {
                self.last_update_time = Some(Instant::now());

                let is_enabled_host = self
                    .config
                    .hosts
                    .iter()
                    .any(|h| h.enabled && h.name == update.host_name);

                // Atualiza ou insere nas estatísticas em memória apenas se o host estiver ativo
                if let Some(existing) = self
                    .stats_list
                    .iter_mut()
                    .find(|s| s.name == update.host_name)
                {
                    *existing = update.stats.clone();
                } else if is_enabled_host {
                    self.stats_list.push(update.stats.clone());
                }

                // Adiciona ponto na série temporal do gráfico apenas para hosts ativos e pings bem-sucedidos
                if is_enabled_host && update.result.status == PingStatus::Success {
                    let elapsed_secs = self.start_time.elapsed().as_secs_f64();
                    let points = self
                        .history_points
                        .entry(update.host_name.clone())
                        .or_default();
                    points.push((elapsed_secs, update.result.rtt_ms));
                    // Mantém até 300 amostras na memória para o gráfico em tempo real
                    if points.len() > 300 {
                        points.remove(0);
                    }
                }

                // Persiste amostra no SQLite via Database Worker
                if let Some(ref db) = self.db_handle {
                    let addr = update
                        .stats
                        .resolved_ip
                        .map(|ip| ip.to_string())
                        .unwrap_or_else(|| update.stats.target_str.clone());

                    db.record_sample(
                        update.host_name,
                        addr,
                        update.host_type,
                        update.result.rtt_ms,
                        update.result.status == PingStatus::Success,
                        update.result.error_message,
                    );
                }
            }
        }

        // 2. Aplica tema configurado somente quando houver alteração (evita churn visual por frame)
        if self.applied_theme != self.config.general.theme {
            match self.config.general.theme.as_str() {
                "dark" => ctx.set_visuals(egui::Visuals::dark()),
                "light" => ctx.set_visuals(egui::Visuals::light()),
                _ => {} // Mantém padrão do sistema
            }
            self.applied_theme = self.config.general.theme.clone();
        }

        // Carrega o logotipo da aplicação na GPU se ainda não estiver carregado
        if self.logo_texture.is_none() {
            let icon_bytes = include_bytes!("../../assets/app.png");
            if let Ok(img) = image::load_from_memory(icon_bytes) {
                let rgba = img.to_rgba8();
                let (w, h) = rgba.dimensions();
                let color_image =
                    egui::ColorImage::from_rgba_unmultiplied([w as usize, h as usize], &rgba);
                self.logo_texture = Some(ctx.load_texture(
                    "app_logo_top",
                    color_image,
                    egui::TextureOptions::LINEAR,
                ));
            }
        }

        // Verificação periódica de alertas sonoros (apenas após o primeiro ciclo de amostras)
        self.alert_manager.enabled = self.config.general.sound_alerts;
        if self.last_update_time.is_some() {
            let is_online = self.diagnostic.active_physical_interface.is_some()
                && self.stats_list.iter().any(|s| s.last_rtt_ms.is_some());
            let max_loss = self
                .stats_list
                .iter()
                .map(|s| s.packet_loss_pct as f32)
                .fold(0.0f32, f32::max);
            let _ = self.alert_manager.check_and_alert(is_online, max_loss);
        }

        let wan_val = self.wan_info.lock().ok().and_then(|g| g.clone());
        let wifi_val = self.wifi_info.lock().ok().and_then(|g| g.clone());
        let t = Language::from_code(&self.config.general.language).translations();

        // 3. Barra Superior com Navegação em Abas
        egui::Panel::top("top_navigation_bar").show(ui, |ui| {
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                if let Some(ref tex) = self.logo_texture {
                    ui.image((tex.id(), egui::vec2(24.0, 24.0)));
                }

                ui.label(
                    RichText::new("RustNet Monitor")
                        .size(20.0)
                        .strong()
                        .color(Color32::from_rgb(230, 126, 34)),
                );

                // 2. Painel da Direita: Botões de Navegação alinhados à direita e Mensagem no Meio
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let tabs = [
                        (ActiveTab::Dashboard, t.tab_dashboard),
                        (ActiveTab::Charts, t.tab_charts),
                        (ActiveTab::Tools, t.tab_tools),
                        (ActiveTab::History, t.tab_history),
                        (ActiveTab::Events, t.tab_events),
                        (ActiveTab::Settings, t.tab_settings),
                    ];

                    for (tab, label) in tabs.iter().rev() {
                        let is_active = self.active_tab == *tab;
                        if ui
                            .selectable_label(is_active, RichText::new(*label).strong())
                            .clicked()
                        {
                            self.active_tab = *tab;
                        }
                    }

                    // 3. Mensagem de Atualização centralizada no meio (entre título e botões)
                    ui.with_layout(
                        egui::Layout::left_to_right(egui::Align::Center)
                            .with_main_align(egui::Align::Center),
                        |ui| {
                            let has_update =
                                self.available_update.lock().ok().and_then(|g| g.clone());
                            if let Some(ref info) = has_update {
                                if ui
                                    .button(
                                        RichText::new(format!(
                                            "{}{}{}",
                                            t.update_available_fmt,
                                            info.version,
                                            t.update_available_suffix
                                        ))
                                        .color(Color32::from_rgb(46, 204, 113))
                                        .strong(),
                                    )
                                    .clicked()
                                {
                                    self.active_tab = ActiveTab::Settings;
                                }
                                ui.add_space(8.0);
                                ui.separator();
                                ui.add_space(8.0);
                            }

                            if let Some(last) = self.last_update_time {
                                let ago = last.elapsed().as_secs();
                                ui.label(
                                    RichText::new(format!("{}{}s", t.updated_ago_prefix, ago))
                                        .weak(),
                                );
                            } else {
                                ui.label(RichText::new(t.waiting_samples).weak());
                            }
                        },
                    );
                });
            });
            ui.add_space(4.0);
        });

        // 4. Barra Inferior de Status
        egui::Panel::bottom("bottom_status_bar").show(ui, |ui| {
            ui.horizontal(|ui| {
                if let Some(iface) = &self.diagnostic.active_physical_interface {
                    ui.label(
                        RichText::new(format!("{} {}", t.status_connection, iface.friendly_name))
                            .weak(),
                    );
                    ui.separator();
                    ui.label(
                        RichText::new(format!("{} {}", t.status_link, iface.formatted_speed()))
                            .weak(),
                    );
                    ui.separator();
                    ui.label(
                        RichText::new(format!(
                            "{} {}",
                            t.status_active_hosts,
                            self.stats_list.len()
                        ))
                        .weak(),
                    );
                } else {
                    ui.label(RichText::new(t.status_offline).color(Color32::from_rgb(231, 76, 60)));
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(
                        RichText::new(format!("RustNet Monitor v{}", env!("CARGO_PKG_VERSION")))
                            .weak(),
                    );
                });
            });
        });

        // 5. Painel Central (Conteúdo da Aba Ativa)
        egui::CentralPanel::default().show(ui, |ui| match self.active_tab {
            ActiveTab::Dashboard => {
                render_dashboard(
                    ui,
                    &mut self.config.dashboard,
                    &self.diagnostic,
                    &self.stats_list,
                    &self.config.thresholds,
                    &wan_val,
                    &wifi_val,
                    t,
                );
            }
            ActiveTab::Charts => {
                render_charts(ui, &self.history_points, &mut self.active_hosts_in_chart, t);
            }
            ActiveTab::Tools => {
                let base_dir = self.base_dir().to_path_buf();
                render_tools(
                    ui,
                    &mut self.tools_state,
                    &self.diagnostic,
                    &self.stats_list,
                    &wan_val,
                    &wifi_val,
                    &base_dir,
                    t,
                );
            }
            ActiveTab::History => {
                let db_path = self.resolved_db_path();
                render_history(ui, &mut self.history_state, &db_path, t);
            }
            ActiveTab::Events => {
                let db_path = self.resolved_db_path();
                render_events(ui, &mut self.events_state, &db_path, t);
            }
            ActiveTab::Settings => {
                render_settings(
                    ui,
                    &mut self.config,
                    &mut self.settings_state,
                    &self.config_path,
                    &self.available_update,
                    &self.update_status,
                    t,
                );

                if self.settings_state.hosts_changed {
                    self.settings_state.hosts_changed = false;
                    self.sync_hosts_and_config();
                }

                // Executa a reinicialização de métricas com base na confirmação do usuário
                if self.settings_state.request_reset_metrics {
                    self.settings_state.request_reset_metrics = false;

                    // 1. Notifica o pinger worker em background para reiniciar acumuladores
                    if let Some(ref tx_cmd) = self.tx_pinger_cmd {
                        let _ = tx_cmd.send(PingerCommand::ResetStats);
                    }

                    // 2. Reseta estatísticas locais em memória
                    for stat in &mut self.stats_list {
                        stat.reset();
                    }

                    // 3. Limpa pontos das séries temporais dos gráficos
                    for points in self.history_points.values_mut() {
                        points.clear();
                    }
                    self.start_time = Instant::now();

                    // 4. Se solicitado, limpa o histórico persistido no SQLite
                    if self.settings_state.clear_db_history {
                        if let Some(ref db) = self.db_handle {
                            db.clear_samples();
                            db.record_event(
                                "Sistema".to_string(),
                                "Manutenção".to_string(),
                                "METRICS_RESET".to_string(),
                                Some(
                                    "Métricas e histórico de amostras zerados pelo usuário"
                                        .to_string(),
                                ),
                            );
                        }
                        self.history_state.cached_samples.clear();
                        self.history_state.last_query_time = None;
                    }

                    self.settings_state.status_message = Some((t.settings_reset_done.to_string(), false));
                }
            }
        });

        // Requisita repaint a cada 500ms para manter animações e gráficos suaves sem consumo excessivo de CPU
        ctx.request_repaint_after(Duration::from_millis(500));
    }
}
