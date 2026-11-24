pub mod en;
pub mod es;
#[path = "pt-br.rs"]
pub mod pt_br;

/// Idiomas suportados pela interface do RustNet Monitor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Language {
    PtBr,
    En,
    Es,
}

impl Language {
    /// Converte o código salvo no `config.toml` (ex: `"pt-BR"`, `"en"`, `"es"`) para `Language`.
    pub fn from_code(code: &str) -> Self {
        match code.trim().to_ascii_lowercase().as_str() {
            "en" | "en-us" | "en_us" => Language::En,
            "es" | "es-es" | "es_es" | "es-419" => Language::Es,
            _ => Language::PtBr,
        }
    }

    pub fn code(&self) -> &'static str {
        match self {
            Language::PtBr => "pt-BR",
            Language::En => "en",
            Language::Es => "es",
        }
    }

    pub fn translations(&self) -> &'static Translations {
        match self {
            Language::PtBr => &pt_br::TRANSLATIONS,
            Language::En => &en::TRANSLATIONS,
            Language::Es => &es::TRANSLATIONS,
        }
    }
}

/// Dicionário estático de todas as strings da interface.
/// Garantido em tempo de compilação: todos os idiomas devem preencher 100% dos campos.
pub struct Translations {
    // Barra Superior e Status
    pub tab_dashboard: &'static str,
    pub tab_charts: &'static str,
    pub tab_tools: &'static str,
    pub tab_history: &'static str,
    pub tab_events: &'static str,
    pub tab_settings: &'static str,
    pub update_available_fmt: &'static str, // prefixo: "✨ v"
    pub update_available_suffix: &'static str,
    pub updated_ago_prefix: &'static str,
    pub waiting_samples: &'static str,
    pub status_connection: &'static str,
    pub status_link: &'static str,
    pub status_active_hosts: &'static str,
    pub status_offline: &'static str,

    // Dashboard
    pub dash_overview_title: &'static str,
    pub dash_expand_all: &'static str,
    pub dash_expand_all_tip: &'static str,
    pub dash_collapse_all: &'static str,
    pub dash_collapse_all_tip: &'static str,
    pub dash_minimize_tip: &'static str,
    pub dash_expand_tip: &'static str,
    pub dash_minimize_table_tip: &'static str,
    pub dash_expand_table_tip: &'static str,
    pub dash_card_interface: &'static str,
    pub dash_connected: &'static str,
    pub dash_disconnected: &'static str,
    pub dash_no_physical_iface: &'static str,
    pub dash_no_physical_iface_msg: &'static str,
    pub dash_adapter: &'static str,
    pub dash_description: &'static str,
    pub dash_type: &'static str,
    pub dash_physical_suffix: &'static str,
    pub dash_link_speed: &'static str,
    pub dash_ipv4: &'static str,
    pub dash_unassigned: &'static str,
    pub dash_gateway: &'static str,
    pub dash_none: &'static str,
    pub dash_mac: &'static str,
    pub dash_unavailable: &'static str,
    pub dash_dns: &'static str,
    pub dash_card_wifi: &'static str,
    pub dash_wifi_ssid: &'static str,
    pub dash_wifi_bssid: &'static str,
    pub dash_wifi_signal: &'static str,
    pub dash_wifi_standard: &'static str,
    pub dash_wifi_rate: &'static str,
    pub dash_wifi_excellent: &'static str,
    pub dash_wifi_good: &'static str,
    pub dash_wifi_fair: &'static str,
    pub dash_wifi_weak: &'static str,
    pub dash_card_wan: &'static str,
    pub dash_wan_public_ip_badge: &'static str,
    pub dash_wan_cgnat_badge: &'static str,
    pub dash_wan_private_nat_badge: &'static str,
    pub dash_wan_na: &'static str,
    pub dash_wan_external_ip: &'static str,
    pub dash_wan_waiting: &'static str,
    pub dash_wan_isp: &'static str,
    pub dash_wan_not_identified: &'static str,
    pub dash_wan_location: &'static str,
    pub dash_wan_topology: &'static str,
    pub dash_wan_desc_direct: &'static str,
    pub dash_wan_desc_cgnat: &'static str,
    pub dash_wan_desc_double_nat: &'static str,
    pub dash_wan_desc_unknown: &'static str,
    pub dash_card_hosts: &'static str,
    pub dash_active_hosts_badge: &'static str,
    pub dash_no_hosts_enabled: &'static str,
    pub col_host: &'static str,
    pub col_address: &'static str,
    pub col_current_latency: &'static str,
    pub col_avg: &'static str,
    pub col_min: &'static str,
    pub col_max: &'static str,
    pub col_jitter: &'static str,
    pub col_loss_pct: &'static str,
    pub col_sent: &'static str,
    pub col_status: &'static str,
    pub quality_good: &'static str,
    pub quality_fair: &'static str,
    pub quality_poor: &'static str,
    pub quality_offline: &'static str,

    // Gráficos
    pub charts_title: &'static str,
    pub charts_hint: &'static str,
    pub charts_shown_hosts: &'static str,
    pub charts_y_axis: &'static str,
    pub charts_x_axis: &'static str,

    // Ferramentas
    pub tools_title: &'static str,
    pub tools_sub_mtr: &'static str,
    pub tools_sub_mtu: &'static str,
    pub tools_sub_dns: &'static str,
    pub tools_sub_bufferbloat: &'static str,
    pub tools_sub_report: &'static str,
    pub mtr_title: &'static str,
    pub mtr_desc: &'static str,
    pub mtr_target_label: &'static str,
    pub mtr_start: &'static str,
    pub mtr_stop: &'static str,
    pub mtr_clear: &'static str,
    pub mtr_running_msg: &'static str,
    pub mtr_empty_msg: &'static str,
    pub mtr_invalid_target: &'static str,
    pub mtr_hops_detected: &'static str,
    pub mtr_final_dest: &'static str,
    pub mtr_dest_lost: &'static str,
    pub mtr_of: &'static str,
    pub mtr_col_hop: &'static str,
    pub mtr_col_ip: &'static str,
    pub mtr_col_sent: &'static str,
    pub mtr_col_recv: &'static str,
    pub mtr_col_lost: &'static str,
    pub mtr_col_loss: &'static str,
    pub mtr_col_cur: &'static str,
    pub mtr_col_avg: &'static str,
    pub mtr_col_min: &'static str,
    pub mtr_col_max: &'static str,
    pub mtr_no_reply: &'static str,
    pub mtu_title: &'static str,
    pub mtu_desc: &'static str,
    pub mtu_target_label: &'static str,
    pub mtu_start: &'static str,
    pub mtu_running_msg: &'static str,
    pub mtu_result_max: &'static str,
    pub mtu_probes_sent: &'static str,
    pub mtu_probes_suffix: &'static str,
    pub dns_title: &'static str,
    pub dns_desc: &'static str,
    pub dns_start: &'static str,
    pub dns_running_msg: &'static str,
    pub dns_col_server: &'static str,
    pub dns_col_ip: &'static str,
    pub dns_col_rtt: &'static str,
    pub dns_col_status: &'static str,
    pub dns_status_ok: &'static str,
    pub dns_status_timeout: &'static str,
    pub bb_title: &'static str,
    pub bb_desc: &'static str,
    pub bb_target_label: &'static str,
    pub bb_start: &'static str,
    pub bb_running_fallback: &'static str,
    pub bb_grade_label: &'static str,
    pub bb_unloaded_label: &'static str,
    pub bb_loaded_label: &'static str,
    pub report_title: &'static str,
    pub report_desc: &'static str,
    pub report_generate: &'static str,
    pub report_generated_ok: &'static str,
    pub report_copy: &'static str,
    pub report_copied_ok: &'static str,
    pub report_save: &'static str,
    pub report_saved_ok_prefix: &'static str,
    pub report_save_err_prefix: &'static str,
    pub report_preview: &'static str,

    // Histórico
    pub history_title: &'static str,
    pub history_db_err_prefix: &'static str,
    pub history_retry: &'static str,
    pub history_host: &'static str,
    pub history_select: &'static str,
    pub history_period: &'static str,
    pub history_refresh: &'static str,
    pub history_filter_5m: &'static str,
    pub history_filter_15m: &'static str,
    pub history_filter_1h: &'static str,
    pub history_filter_6h: &'static str,
    pub history_filter_24h: &'static str,
    pub history_filter_7d: &'static str,
    pub history_filter_all: &'static str,
    pub history_samples: &'static str,
    pub history_avg_latency: &'static str,
    pub history_min_latency: &'static str,
    pub history_max_latency: &'static str,
    pub history_packet_loss: &'static str,
    pub history_recorded_suffix: &'static str,
    pub history_lost_suffix: &'static str,
    pub history_records_title: &'static str,
    pub history_col_datetime: &'static str,
    pub history_col_latency: &'static str,
    pub history_col_status: &'static str,
    pub history_col_detail: &'static str,
    pub history_status_ok: &'static str,
    pub history_status_err: &'static str,

    // Eventos
    pub events_title: &'static str,
    pub events_refresh: &'static str,
    pub events_empty: &'static str,
    pub events_col_datetime: &'static str,
    pub events_col_event: &'static str,
    pub events_col_iface: &'static str,
    pub events_col_type: &'static str,
    pub events_col_details: &'static str,

    // Configurações
    pub settings_title: &'static str,
    pub settings_general_title: &'static str,
    pub settings_theme_label: &'static str,
    pub settings_theme_dark: &'static str,
    pub settings_theme_light: &'static str,
    pub settings_theme_system: &'static str,
    pub settings_language_label: &'static str,
    pub settings_minimize_tray: &'static str,
    pub settings_start_windows: &'static str,
    pub settings_sound_alerts: &'static str,
    pub settings_monitoring_title: &'static str,
    pub settings_interval_label: &'static str,
    pub settings_timeout_label: &'static str,
    pub settings_thresholds_title: &'static str,
    pub settings_green_limit: &'static str,
    pub settings_yellow_limit: &'static str,
    pub settings_thresholds_hint: &'static str,
    pub settings_persistence_title: &'static str,
    pub settings_retention_label: &'static str,
    pub settings_ret_7d: &'static str,
    pub settings_ret_30d: &'static str,
    pub settings_ret_90d: &'static str,
    pub settings_ret_1y: &'static str,
    pub settings_ret_unlimited: &'static str,
    pub settings_hosts_title: &'static str,
    pub settings_remove: &'static str,
    pub settings_dynamic_default: &'static str,
    pub settings_add_host_title: &'static str,
    pub settings_host_name: &'static str,
    pub settings_host_addr: &'static str,
    pub settings_add_btn: &'static str,
    pub settings_msg_fill_fields: &'static str,
    pub settings_msg_duplicate_host: &'static str,
    pub settings_msg_host_added: &'static str,
    pub settings_msg_host_added_err: &'static str,
    pub settings_msg_host_removed: &'static str,
    pub settings_msg_host_removed_err: &'static str,
    pub settings_maintenance_title: &'static str,
    pub settings_reset_btn: &'static str,
    pub settings_reset_hint: &'static str,
    pub settings_reset_confirm_title: &'static str,
    pub settings_reset_confirm_q1: &'static str,
    pub settings_reset_confirm_q2: &'static str,
    pub settings_reset_clear_db: &'static str,
    pub settings_reset_confirm_yes: &'static str,
    pub settings_cancel: &'static str,
    pub settings_reset_done: &'static str,
    pub settings_updates_title: &'static str,
    pub settings_installed_version: &'static str,
    pub settings_repository: &'static str,
    pub settings_check_updates: &'static str,
    pub settings_checking_updates: &'static str,
    pub settings_up_to_date: &'static str,
    pub settings_check_again: &'static str,
    pub settings_downloading: &'static str,
    pub settings_try_again: &'static str,
    pub settings_new_version_prefix: &'static str,
    pub settings_new_version_suffix: &'static str,
    pub settings_release_notes: &'static str,
    pub settings_update_now: &'static str,
    pub settings_save_btn: &'static str,
    pub settings_saved_ok: &'static str,
    pub settings_save_err: &'static str,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_language_code_roundtrip() {
        assert_eq!(Language::from_code("pt-BR"), Language::PtBr);
        assert_eq!(Language::from_code("en"), Language::En);
        assert_eq!(Language::from_code("en-US"), Language::En);
        assert_eq!(Language::from_code("es"), Language::Es);
        assert_eq!(Language::from_code("es-ES"), Language::Es);
        assert_eq!(Language::from_code("unknown"), Language::PtBr);

        assert_eq!(Language::PtBr.code(), "pt-BR");
        assert_eq!(Language::En.code(), "en");
        assert_eq!(Language::Es.code(), "es");
    }
}
