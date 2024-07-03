use std::fs;
use once_cell::sync::OnceCell;
use crate::config::SunbotConfig;

static GLOBAL_CONFIG: OnceCell<SunbotConfig> = OnceCell::new();

pub mod config;

pub fn load_config(path: &str) {
    let cfg_str = fs::read_to_string(path).expect(&format!("Failed to read file: {}", path));
    let config: SunbotConfig = toml::from_str(&cfg_str).expect("Failed to deserialize ");

    GLOBAL_CONFIG
		.set(config)
		.unwrap_or_else(|_| panic!("don't call `load_config()` more than once"));
}

pub fn get_config() -> &'static SunbotConfig {
    GLOBAL_CONFIG
        .get()
        .expect("called `get_config()` before config was initialized")
}
