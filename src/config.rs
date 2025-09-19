use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::{RwLock, watch};
use anyhow::Result;
use aws_config::meta::region::RegionProviderChain;
use aws_sdk_appconfigdata::Client as AppConfigClient;
use crate::prober::ProbeKind;
use crate::util::parse_host_port;
use std::path::Path;
use tokio::fs;

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct TargetConfig {
    pub name: String,
    pub kind: ProbeKind,
    pub host: String,
    pub port: Option<u16>,
    // Remove the url field - we'll construct it from host + port
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct ProbeConfig {
    pub probe_interval_ms: u64,
    pub default_timeout_ms: u64,
    pub targets: Vec<TargetConfig>,
    #[serde(default = "default_log_level")]
    pub log_level: String,
    #[serde(default = "default_enable_latency_history")]
    pub enable_latency_history: bool,
}

fn default_log_level() -> String {
    "info".to_string()
}

fn default_enable_latency_history() -> bool {
    false // Default to show current latency only
}

impl ProbeConfig {
    /// Get the log level as a tracing::Level
    pub fn get_tracing_level(&self) -> Result<tracing::Level> {
        match self.log_level.to_lowercase().as_str() {
            "trace" => Ok(tracing::Level::TRACE),
            "debug" => Ok(tracing::Level::DEBUG),
            "info" => Ok(tracing::Level::INFO),
            "warn" | "warning" => Ok(tracing::Level::WARN),
            "error" => Ok(tracing::Level::ERROR),
            _ => Err(anyhow::anyhow!("Invalid log level: {}. Valid levels are: trace, debug, info, warn, error", self.log_level))
        }
    }

    /// Validate the log level is one of the supported values
    pub fn validate_log_level(&self) -> Result<()> {
        self.get_tracing_level().map(|_| ())
    }
}

pub struct ConfigManager {
    pub config: Arc<RwLock<ProbeConfig>>,
    pub targets: Arc<RwLock<Vec<TargetConfig>>>,

    // for shutdown if needed
    _shutdown: watch::Receiver<()>,
}

impl ConfigManager {
    pub async fn start() -> Result<Self> {
        // Check if we should use AppConfig or local file
        let use_app_config = std::env::var("USE_APP_CONFIG")
            .unwrap_or_else(|_| "false".to_string())
            .parse::<bool>()
            .unwrap_or(false);

        if use_app_config {
            Self::start_with_app_config().await
        } else {
            Self::start_with_local_file().await
        }
    }

    async fn start_with_app_config() -> Result<Self> {
        println!("Starting with AWS AppConfig");
        
        // Load AWS config
        let region_provider = RegionProviderChain::default_provider().or_else("us-east-1");
        let aws_cfg = aws_config::defaults(aws_config::BehaviorVersion::latest())
            .region(region_provider)
            .load()
            .await;
        let client = AppConfigClient::new(&aws_cfg);

        // Env vars or default
        let app_id = std::env::var("APP_CONFIG_APPLICATION_ID")?;
        let env_id = std::env::var("APP_CONFIG_ENVIRONMENT_ID")?;
        let profile_id = std::env::var("APP_CONFIG_PROFILE_ID")?;
        let poll_interval_sec: u64 = std::env::var("APP_CONFIG_POLL_INTERVAL_SECONDS")
            .unwrap_or_else(|_| "60".to_string())
            .parse()
            .unwrap_or(60);

        // Load initial
        let initial = Self::fetch_app_config(&client, &app_id, &env_id, &profile_id).await?;
        let config = Arc::new(RwLock::new(initial.clone()));
        let targets = Arc::new(RwLock::new(initial.targets.clone()));

        // optional: shutdown signal channel (not used here)
        let (_shutdown_tx, shutdown_rx) = watch::channel(());

        // Spawn background task to poll
        {
            let config_clone = config.clone();
            let targets_clone = targets.clone();
            tokio::spawn(async move {
                loop {
                    tokio::time::sleep(tokio::time::Duration::from_secs(poll_interval_sec)).await;
                    match Self::fetch_app_config(&client, &app_id, &env_id, &profile_id).await {
                        Ok(new_cfg) => {
                            // check if changed
                            let mut c = config_clone.write().await;
                            if *c != new_cfg {
                                tracing::info!("AppConfig updated");
                                *c = new_cfg.clone();
                                // update targets list
                                let mut t = targets_clone.write().await;
                                *t = new_cfg.targets.clone();
                            }
                        }
                        Err(e) => {
                            tracing::error!("Error polling AppConfig: {:?}", e);
                        }
                    }
                }
            });
        }

        Ok(ConfigManager {
            config,
            targets,
            _shutdown: shutdown_rx,
        })
    }

    async fn start_with_local_file() -> Result<Self> {
        let config_file = std::env::var("TARGET_CONFIG")
            .unwrap_or_else(|_| "targets.json".to_string());
        
        println!("Starting with local file: {}", config_file);

        // Load initial config from file
        let initial = Self::load_file_config(&config_file).await?;
        let config = Arc::new(RwLock::new(initial.clone()));
        let targets = Arc::new(RwLock::new(initial.targets.clone()));

        let poll_interval_sec: u64 = std::env::var("CONFIG_POLL_INTERVAL_SECONDS")
            .unwrap_or_else(|_| "30".to_string())
            .parse()
            .unwrap_or(30);

        // optional: shutdown signal channel (not used here)
        let (_shutdown_tx, shutdown_rx) = watch::channel(());

        // Spawn background task to watch file for changes
        {
            let config_clone = config.clone();
            let targets_clone = targets.clone();
            let config_file_clone = config_file.clone();
            
            tokio::spawn(async move {
                loop {
                    tokio::time::sleep(tokio::time::Duration::from_secs(poll_interval_sec)).await;
                    match Self::load_file_config(&config_file_clone).await {
                        Ok(new_cfg) => {
                            // check if changed
                            let mut c = config_clone.write().await;
                            if *c != new_cfg {
                                tracing::info!("Local config file updated");
                                *c = new_cfg.clone();
                                // update targets list
                                let mut t = targets_clone.write().await;
                                *t = new_cfg.targets.clone();
                            }
                        }
                        Err(e) => {
                            tracing::error!("Error reading config file {}: {:?}", config_file_clone, e);
                        }
                    }
                }
            });
        }

        Ok(ConfigManager {
            config,
            targets,
            _shutdown: shutdown_rx,
        })
    }

    async fn load_file_config(file_path: &str) -> Result<ProbeConfig> {
        if !Path::new(file_path).exists() {
            return Err(anyhow::anyhow!("Config file not found: {}", file_path));
        }
        
        let content = fs::read_to_string(file_path).await?;
        let config: ProbeConfig = serde_json::from_str(&content)?;
        Ok(config)
    }

    async fn fetch_app_config(
        client: &AppConfigClient,
        app_id: &str,
        env_id: &str,
        profile_id: &str,
    ) -> Result<ProbeConfig> {
        // Start session
        let session_resp = client
            .start_configuration_session()
            .application_identifier(app_id)
            .environment_identifier(env_id)
            .configuration_profile_identifier(profile_id)
            .send()
            .await?;

        let token = session_resp
            .initial_configuration_token()
            .ok_or_else(|| anyhow::anyhow!("No initial token from AppConfigData"))?;

        let latest = client
            .get_latest_configuration()
            .configuration_token(token)
            .send()
            .await?;

        let cfg_bytes = latest
            .configuration()
            .map(|c| c.as_ref())
            .unwrap_or_default();

        let cfg: ProbeConfig = serde_json::from_slice(cfg_bytes)?;
        Ok(cfg)
    }
}

impl TargetConfig {
    pub fn get_host_port(&self, default_port: u16) -> (String, u16) {
        parse_host_port(&self.host, self.port.unwrap_or(default_port))
    }

    // Updated method to just concatenate host + port
    pub fn get_http_url(&self) -> String {
        let port = self.port.unwrap_or(80);
        format!("{}:{}", self.host, port)
    }
}
