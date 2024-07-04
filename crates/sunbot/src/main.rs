use poise::serenity_prelude as serenity;
use sunbot_config::{self, config::SunbotConfig};
use tracing::{info, warn, Level};
use tracing_subscriber::FmtSubscriber;

mod commands;
mod handlers;
mod utils;

pub mod built_info {
    // The file has been placed there by the build script.
    include!(concat!(env!("OUT_DIR"), "/built.rs"));
}

pub struct Data {
    _config: &'static SunbotConfig,
}

type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, Data, Error>;

fn load_config() -> &'static SunbotConfig {
    let cfg_path = std::env::var("SUNBOT_CONFIG_FILE").unwrap_or(String::from("config.toml"));
    info!("Loading configuration from: {}", cfg_path);
    sunbot_config::load_config(&cfg_path);
    sunbot_config::get_config()
}

async fn on_ready(
    _ctx: &serenity::Context,
    ready: &serenity::Ready,
    _framework: &poise::Framework<Data, Error>,
) -> Result<Data, Error> {
    info!("Logged in as {}", ready.user.name);
    let config = sunbot_config::get_config();
    // Setup/Configure DB access
    Ok(Data { _config: config })
}

async fn bot_entrypoint() {
    let config = sunbot_config::get_config();

    let commands = vec![
        commands::register::register_commands(),
        commands::meta::ping(),
        commands::meta::about(),
    ];

    let options = poise::FrameworkOptions {
        commands: commands,
        prefix_options: poise::PrefixFrameworkOptions {
            prefix: Some("~".into()),
            execute_self_messages: false,
            execute_untracked_edits: true,
            mention_as_prefix: true,
            ..Default::default()
        },
        event_handler: |ctx, event, framework, data| {
            Box::pin(handlers::handler(ctx, event, framework, data))
        },
        ..Default::default()
    };

    let framework = poise::Framework::builder()
        .setup(|ctx, ready, framework| Box::pin(on_ready(ctx, ready, framework)))
        .options(options)
        .build();

    let intents = serenity::GatewayIntents::all();

    let client = serenity::ClientBuilder::new(config.discord.token.as_str(), intents)
        .framework(framework)
        .await;

    client.unwrap().start().await.unwrap()
}

fn main() {
    // Configue Logging
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    let config = load_config();

    let dsn = config
        .sentry
        .as_ref()
        .and_then(|sentry| sentry.dsn.as_deref())
        .unwrap_or("");

    if dsn.is_empty() {
        warn!("Sentry initialized with empty DSN - will be disabled")
    }

    let _guard = sentry::init((
        dsn,
        sentry::ClientOptions {
            release: sentry::release_name!(),
            ..Default::default()
        },
    ));

    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async { bot_entrypoint().await });
}
