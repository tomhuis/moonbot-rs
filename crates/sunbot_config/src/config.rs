use serde::Deserialize;

#[derive(Debug, Deserialize, Default)]
#[serde(default)]
pub struct SunbotConfig {
    pub discord: DiscordConfig,
    pub lavalink: LavalinkConfig,
    pub database: DatabaseConfig,
    pub openai: OpenAIConfig,
    pub sentry: SentryConfig,
}

#[derive(Debug, Deserialize, Default)]
#[serde(default)]
pub struct DiscordConfig {
    // The Discord token for the bot
    pub token: String,
}

#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct LavalinkConfig {
    // Lavalink Host to connect to
    pub host: String,
    // Lavalink password to connect with
    pub password: String,
    // Lavalink port to connect to, default is 2333
    pub port: i16,
    // Whether to use SSL to connect to Lavalink
    pub use_ssl: bool,
}

impl Default for LavalinkConfig {
    fn default() -> Self {
        LavalinkConfig {
            host: String::from(""),
            password: String::from(""),
            port: 2333,
            use_ssl: false,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct DatabaseConfig {
    // The URL of the database to connect to
    pub url: String,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        DatabaseConfig {
            url: String::from("sqlite://sunbot.sqlite?mode=rwc"),
        }
    }
}

#[derive(Debug, Deserialize, Default)]
#[serde(default)]
pub struct OpenAIConfig {
    // The OpenAI API Key
    pub api_key: String,
    // Configuration for the /askgpt command
    pub askgpt: OpenAIAskgpt,
    // Configuration for the /genimage command
    pub genimage: OpenAIGenImage,
    // Configuration for the automatic replies
    pub auto: OpenAIAuto,
}

#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct OpenAIAskgpt {
    // The model to use
    pub model: String,
    // Whether to use the vision model
    pub use_vision: bool,
    // The maximum number of tokens to generate
    pub max_tokens: u32,
}

impl Default for OpenAIAskgpt {
    fn default() -> Self {
        OpenAIAskgpt {
            model: String::from("gpt-4o"),
            use_vision: true,
            max_tokens: 500,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct OpenAIGenImage {
    // The model to use
    pub model: String,
}

impl Default for OpenAIGenImage {
    fn default() -> Self {
        OpenAIGenImage {
            model: String::from("dall-e-3"),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct OpenAIAuto {
    // A list of strings to provide as context to the autoresponder
    pub system_context: Vec<String>,
    // The model to use
    pub model: String,
    // Whether to use vision
    pub use_vision: bool,
    // The maximum number of tokens to generate
    pub max_tokens: u32,
    // The maximum number of messages to collect as context
    pub max_messages: u8,
    // The maximum age of message in seconds in relation to the current message
    // to consider it as context
    pub max_message_age: i64,
    // Configuration for the random responses
    pub random: OpenAIAutoRandom,
    // Configuration for user relationship tracking
    pub relationship_tracking: RelationshipTrackingConfig,
}

impl Default for OpenAIAuto {
    fn default() -> Self {
        OpenAIAuto {
            system_context: Vec::new(),
            model: String::from("gpt-4o"),
            use_vision: true,
            max_tokens: 100,
            max_messages: 30,
            max_message_age: 86400,
            random: OpenAIAutoRandom::default(),
            relationship_tracking: RelationshipTrackingConfig::default(),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct OpenAIAutoRandom {
    pub min_length: u32,
    pub cooldown: u64,
    pub trigger_chance: f64,
}

impl Default for OpenAIAutoRandom {
    fn default() -> Self {
        OpenAIAutoRandom {
            // The minimum length of the message to trigger a random response
            min_length: 10,
            // The cooldown in seconds between random responses
            cooldown: 600,
            // The chance of triggering a random response
            trigger_chance: 0.2,
        }
    }
}

#[derive(Debug, Deserialize, Default)]
#[serde(default)]
pub struct SentryConfig {
    pub dsn: String,
}

#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct RelationshipTrackingConfig {
    // Whether to enable user relationship tracking
    pub enabled: bool,
    // Maximum number of keywords to store per user
    pub max_keywords_per_user: usize,
    // How much to adjust temperature per positive/negative sentiment detection
    pub temperature_adjustment: f32,
    // Whether to include relationship context in AI prompts
    pub include_context_in_prompts: bool,
}

impl Default for RelationshipTrackingConfig {
    fn default() -> Self {
        RelationshipTrackingConfig {
            enabled: true,
            max_keywords_per_user: 20,
            temperature_adjustment: 0.1,
            include_context_in_prompts: true,
        }
    }
}
