use serde::Deserialize;


#[derive(Debug, Deserialize)]
pub struct SunbotConfig {
    pub discord: DiscordConfig,
    pub lavalink: Option<LavalinkConfig>,
    pub database: DatabaseConfig,
    pub openai: Option<OpenaiConfig>,
    pub sentry: Option<SentryConfig>
}


#[derive(Debug, Deserialize)]
pub struct DiscordConfig {
    pub token: String,
    pub default_guilds: Vec<u64>
}


#[derive(Debug, Deserialize)]
pub struct LavalinkConfig {
    pub host: String,
    pub password: String,
    #[serde(default = "default_lavalink_port")]
    pub port: i16
}


fn default_lavalink_port() -> i16 {
    return 2333;
}


#[derive(Debug, Deserialize)]
pub struct DatabaseConfig {
    pub url: String
}


#[derive(Debug, Deserialize)]
pub struct OpenaiConfig {
    pub api_key: String
}


#[derive(Debug, Deserialize)]
pub struct SentryConfig {
    pub dsn: Option<String>
}
