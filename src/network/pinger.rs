use std::net::{Ipv4Addr, ToSocketAddrs};
use std::str::FromStr;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::thread;
use std::time::Duration;
use tracing::{debug, info};

use crate::config::settings::{AppConfig, HostConfig};
use crate::network::icmp::{PingResult, PingStatus, ping_ipv4};
use crate::network::stats::HostStats;

#[derive(Debug, Clone)]
pub struct PingUpdate {
    pub host_name: String,
    pub host_type: String,
    pub result: PingResult,
    pub stats: HostStats,
}

/// Resolve o destino configurado para um endereço IPv4 válido.
pub fn resolve_target(target_str: &str, dynamic_gateway: Option<Ipv4Addr>) -> Option<Ipv4Addr> {
    if target_str.eq_ignore_ascii_case("auto") {
        return dynamic_gateway;
    }

    if let Ok(ipv4) = Ipv4Addr::from_str(target_str) {
        return Some(ipv4);
    }

    // Tenta resolução DNS para hostnames (ex: "google.com")
    let socket_str = format!("{}:80", target_str);
    if let Ok(iter) = socket_str.to_socket_addrs() {
        for addr in iter {
            if let std::net::SocketAddr::V4(v4) = addr {
                return Some(*v4.ip());
            }
        }
    }

    None
}

/// Comandos de controle para a thread do pinger.
#[derive(Debug, Clone)]
pub enum PingerCommand {
    ResetStats,
    UpdateConfig(AppConfig),
}

/// Orquestrador de testes de conectividade periódicos em background.
pub struct PingerService {
    config: AppConfig,
    dynamic_gateway: Option<Ipv4Addr>,
    hosts: Vec<(HostConfig, HostStats)>,
    stop_signal: Arc<AtomicBool>,
}

impl PingerService {
    pub fn new(config: AppConfig, detected_gateway: Option<Ipv4Addr>) -> Self {
        let mut hosts = Vec::new();

        for host_cfg in &config.hosts {
            if host_cfg.enabled {
                let stats = HostStats::new(host_cfg.name.clone(), host_cfg.address.clone());
                hosts.push((host_cfg.clone(), stats));
            }
        }

        Self {
            config,
            dynamic_gateway: detected_gateway,
            hosts,
            stop_signal: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Atualiza as configurações e a lista de hosts monitorados a quente.
    /// Preserva as métricas acumuladas de hosts pré-existentes.
    pub fn update_config(&mut self, new_config: AppConfig) {
        let mut new_hosts = Vec::new();

        for host_cfg in &new_config.hosts {
            if host_cfg.enabled {
                if let Some((_, existing_stats)) = self
                    .hosts
                    .iter()
                    .find(|(c, _)| c.name == host_cfg.name && c.address == host_cfg.address)
                {
                    new_hosts.push((host_cfg.clone(), existing_stats.clone()));
                } else {
                    let stats = HostStats::new(host_cfg.name.clone(), host_cfg.address.clone());
                    new_hosts.push((host_cfg.clone(), stats));
                }
            }
        }

        self.hosts = new_hosts;
        self.config = new_config;
    }

    /// Executa uma única rodada de pings síncrona sobre todos os hosts ativos.
    pub fn probe_all(&mut self) -> Vec<PingUpdate> {
        let timeout = Duration::from_millis(self.config.monitoring.timeout_ms);
        let mut updates = Vec::new();

        for (cfg, stats) in self.hosts.iter_mut() {
            let target_ip = if cfg.is_dynamic_gateway {
                self.dynamic_gateway
            } else {
                resolve_target(&cfg.address, self.dynamic_gateway)
            };

            let result = if let Some(ip) = target_ip {
                ping_ipv4(ip, timeout)
            } else {
                PingResult {
                    target_ip: Ipv4Addr::UNSPECIFIED,
                    rtt_ms: 0.0,
                    status: PingStatus::Error,
                    error_message: Some(format!("Falha ao resolver endereço '{}'", cfg.address)),
                }
            };

            stats.record_sample(result.clone());

            updates.push(PingUpdate {
                host_name: cfg.name.clone(),
                host_type: cfg.host_type.clone(),
                result,
                stats: stats.clone(),
            });
        }

        updates
    }

    /// Processa um comando recebido na thread do worker.
    /// Retorna `true` se o ciclo de espera deve ser interrompido para sondagem imediata.
    fn handle_command(&mut self, cmd: PingerCommand, interval: &mut Duration) -> bool {
        match cmd {
            PingerCommand::ResetStats => {
                info!("Pinger: reinicializando estatísticas acumuladas de todos os hosts...");
                for (_, stats) in self.hosts.iter_mut() {
                    stats.reset();
                }
                false
            }
            PingerCommand::UpdateConfig(new_cfg) => {
                info!("Pinger: atualizando lista de hosts e parâmetros a quente...");
                *interval = Duration::from_secs(new_cfg.monitoring.interval_secs.max(1));
                self.update_config(new_cfg);
                true
            }
        }
    }

    /// Obtém referência para as estatísticas atuais de todos os hosts monitorados.
    pub fn current_stats(&self) -> Vec<HostStats> {
        self.hosts.iter().map(|(_, s)| s.clone()).collect()
    }

    /// Inicia a thread de monitoramento contínuo em background.
    /// Retorna um canal de dados, um canal de comandos, um sinal de parada e o manipulador da thread.
    pub fn start_worker(
        mut self,
    ) -> (
        Receiver<PingUpdate>,
        Sender<PingerCommand>,
        Arc<AtomicBool>,
        thread::JoinHandle<()>,
    ) {
        let (tx, rx) = channel::<PingUpdate>();
        let (cmd_tx, cmd_rx) = channel::<PingerCommand>();
        let stop_signal = self.stop_signal.clone();
        let thread_stop = stop_signal.clone();

        info!(
            "Iniciando monitoramento ICMP contínuo (intervalo: {}s)...",
            self.config.monitoring.interval_secs
        );

        let handle = thread::spawn(move || {
            let mut interval = Duration::from_secs(self.config.monitoring.interval_secs.max(1));
            while !thread_stop.load(Ordering::Relaxed) {
                // Processa eventuais comandos pendentes antes de pingar
                while let Ok(cmd) = cmd_rx.try_recv() {
                    self.handle_command(cmd, &mut interval);
                }

                let updates = self.probe_all();
                for update in updates {
                    if tx.send(update).is_err() {
                        debug!("Canal de atualizações fechado, encerrando worker ICMP.");
                        return;
                    }
                }

                // Espera pelo próximo intervalo com checagem de parada e comandos a cada 100ms
                let steps = (interval.as_millis() / 100).max(1);
                for _ in 0..steps {
                    if thread_stop.load(Ordering::Relaxed) {
                        break;
                    }
                    let mut should_break = false;
                    while let Ok(cmd) = cmd_rx.try_recv() {
                        if self.handle_command(cmd, &mut interval) {
                            should_break = true;
                        }
                    }
                    if should_break {
                        break;
                    }
                    thread::sleep(Duration::from_millis(100));
                }
            }
            info!("Worker de monitoramento ICMP finalizado.");
        });

        (rx, cmd_tx, stop_signal, handle)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pinger_update_config() {
        let mut config = AppConfig {
            hosts: vec![
                HostConfig {
                    name: "Host 1".to_string(),
                    address: "1.1.1.1".to_string(),
                    host_type: "custom".to_string(),
                    enabled: true,
                    is_dynamic_gateway: false,
                },
                HostConfig {
                    name: "Host 2".to_string(),
                    address: "8.8.8.8".to_string(),
                    host_type: "custom".to_string(),
                    enabled: true,
                    is_dynamic_gateway: false,
                },
            ],
            ..Default::default()
        };

        let mut pinger = PingerService::new(config.clone(), None);
        assert_eq!(pinger.hosts.len(), 2);

        // Altera configuração: desativa Host 2 e adiciona Host 3
        config.hosts[1].enabled = false;
        config.hosts.push(HostConfig {
            name: "Host 3".to_string(),
            address: "9.9.9.9".to_string(),
            host_type: "custom".to_string(),
            enabled: true,
            is_dynamic_gateway: false,
        });

        pinger.update_config(config);
        assert_eq!(pinger.hosts.len(), 2);
        assert_eq!(pinger.hosts[0].0.name, "Host 1");
        assert_eq!(pinger.hosts[1].0.name, "Host 3");
    }
}
