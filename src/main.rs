mod config;
mod scheduler;
mod prober;
mod metrics;
mod timestamp;
mod util;

use config::ConfigManager;
use scheduler::Scheduler;
use metrics::{observe_latency, inc_timeout, initialize_metrics};
use prober::ProbeKind;

use std::sync::Arc;
use tracing::{info, error};

#[tokio::main(flavor = "multi_thread", worker_threads = 4)]
async fn main() -> anyhow::Result<()> {
    // Load config first to get log level
    let config_mgr = Arc::new(ConfigManager::start().await?);
    let log_level = config_mgr.config.read().await.get_tracing_level()?;

    println!("Starting latency_probe");

    // Initialize metrics based on configuration
    let enable_latency_history = config_mgr.config.read().await.enable_latency_history;
    initialize_metrics(enable_latency_history);
    
    if enable_latency_history {
        println!("Latency history tracking enabled");
    } else {
        println!("Latency history tracking disabled - showing current latency only");
    }
    
    // Init tracing with configured log level
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env()
                         .add_directive(format!("latency_probe={}", log_level.as_str().to_lowercase()).parse()?))
        .init();

    // Start metrics endpoint
    let metrics_addr = ([0, 0, 0, 0], 9100).into();
    tokio::spawn(metrics::serve_metrics(metrics_addr));

    // Scheduler: using interval poll from config or default
    let probe_interval_ms = config_mgr.config.read().await.probe_interval_ms;
    let scheduler = Scheduler::new(probe_interval_ms)?;

    // Targets list
    let targets = config_mgr.targets.clone();

    scheduler.run(move || {
        let targets = targets.clone();
        let config_mgr = config_mgr.clone(); // Clone config_mgr so it can be moved into the closure
        async move {
            let targets_snapshot = { targets.read().await.clone() };
            for t in targets_snapshot.into_iter() {
                let t2 = t.clone();
                let config_mgr = config_mgr.clone(); // Clone again for each spawned task
                tokio::spawn(async move {
                    match t2.kind {
                        ProbeKind::Icmp => {
                            // Get timeout from config or use default
                            let config = config_mgr.config.read().await;
                            let timeout_ms = config.default_timeout_ms;
                            drop(config);
                            
                            match prober::icmp::probe_icmp(&t2.host, timeout_ms).await {
                                Ok(latency) => {
                                    info!("icmp probe {} success: {:?}", t2.host, latency);
                                    observe_latency(&t2.name, "icmp", latency.as_secs_f64() * 1000.0);
                                }
                                Err(e) => {
                                    error!("icmp probe {} failed: {:?}", t2.host, e);
                                    inc_timeout(&t2.name, "icmp");
                                }
                            }
                        }
                        ProbeKind::TcpConnect => {
                            match prober::tcp_connect::probe_tcp(&t2.host, t2.port.unwrap_or(80)).await {
                                Ok(latency) => {
                                    info!("tcp connect {} success: {:?}", t2.host, latency);
                                    observe_latency(&t2.name, "tcp_connect", latency.as_secs_f64() * 1000.0);
                                }
                                Err(e) => {
                                    error!("tcp connect {} failed: {:?}", t2.host, e);
                                    inc_timeout(&t2.name, "tcp_connect");
                                }
                            }
                        }
                        ProbeKind::Http => {
                            let url = t2.get_http_url();
                            match prober::http::probe_http(&url).await {
                                Ok(latency) => {
                                    info!("http probe {} success: {:?}", url, latency);
                                    observe_latency(&t2.name, "http", latency.as_secs_f64() * 1000.0);
                                }
                                Err(e) => {
                                    error!("http probe {} failed: {:?}", url, e);
                                    inc_timeout(&t2.name, "http");
                                }
                            }
                        }
                        ProbeKind::Echo => {
                            match prober::echo::probe_echo(&t2.host, t2.port.unwrap_or(9000)).await {
                                Ok(latency) => {
                                    info!("echo probe {} success: {:?}", t2.host, latency);
                                    observe_latency(&t2.name, "echo", latency.as_secs_f64() * 1000.0);
                                }
                                Err(e) => {
                                    error!("echo probe {} failed: {:?}", t2.host, e);
                                    inc_timeout(&t2.name, "echo");
                                }
                            }
                        }
                    }
                });
            }
        }
    }).await?;

    Ok(())
}
