use crate::config::SunbotConfig;
use std::fs;
use std::sync::OnceLock;
use tracing::info;

static GLOBAL_CONFIG: OnceLock<SunbotConfig> = OnceLock::new();

pub mod config;

pub fn load_config() -> &'static SunbotConfig {
    let cfg_path = std::env::var("SUNBOT_CONFIG_FILE").unwrap_or(String::from("config.toml"));
    info!("Loading configuration from: {}", cfg_path);

    let cfg_str = fs::read_to_string(cfg_path.as_str())
        .unwrap_or_else(|_| panic!("Failed to read file: {}", cfg_path));
    let config: SunbotConfig = toml::from_str(&cfg_str).expect("Failed to deserialize ");

    GLOBAL_CONFIG
        .set(config)
        .unwrap_or_else(|_| panic!("don't call `load_config()` more than once"));

    get_config()
}

pub fn get_config() -> &'static SunbotConfig {
    GLOBAL_CONFIG
        .get()
        .expect("called `get_config()` before config was initialized")
}
