use lavalink_rs::{model::events, prelude::*};
use poise::serenity_prelude as serenity;
use sea_orm::DatabaseConnection;
use songbird::SerenityInit;
use moonbot_config::{self, config::SunbotConfig};
use moonbot_db::{get_db, init_db};
use tracing::{info, warn, Level};
use tracing_subscriber::{filter, prelude::*};

mod commands;
mod handlers;
mod utils;
mod context;

pub mod built_info {
    // The file has been placed there by the build script.
    include!(concat!(env!("OUT_DIR"), "/built.rs"));
}

pub struct Data {
    config: &'static SunbotConfig,
    openai_client: Option<async_openai::Client<async_openai::config::OpenAIConfig>>,
    lavalink: Option<LavalinkClient>,
    db: &'static DatabaseConnection,
}

type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, Data, Error>;

async fn on_ready(
    ctx: &serenity::Context,
    ready: &serenity::Ready,
    _framework: &poise::Framework<Data, Error>,
) -> Result<Data, Error> {
    info!("Logged in as {}", ready.user.name);
    let config: &SunbotConfig = moonbot_config::get_config();

    // Initialize the database
    init_db(&config.database.url).await;

    // Configure OpenAI
    let openai_client = if !config.openai.api_key.is_empty() || !config.openai.api_base.is_empty() {
        let mut cfg = async_openai::config::OpenAIConfig::new();
        if !config.openai.api_key.is_empty() {
            cfg = cfg.with_api_key(config.openai.api_key.as_str());
        }
        if !config.openai.api_base.is_empty() {
            cfg = cfg.with_api_base(config.openai.api_base.as_str());
        }
        Some(async_openai::Client::with_config(cfg))
    } else {
        None
    };

    // Setup Lavalink
    let lavalink_client = if !config.lavalink.host.is_empty() {
        let events = events::Events {
            raw: Some(handlers::lavalink::raw_event),
            ready: Some(handlers::lavalink::ready_event),
            track_start: Some(handlers::lavalink::track_start),
            ..Default::default()
        };

        let lavalink_host = format!("{}:{}", config.lavalink.host, config.lavalink.port);

        let node_local = NodeBuilder {
            hostname: lavalink_host,
            is_ssl: config.lavalink.use_ssl,
            events: events::Events::default(),
            password: config.lavalink.password.clone(),
            user_id: ctx.cache.current_user().id.into(),
            session_id: None,
        };

        Some(
            LavalinkClient::new(
                events,
                vec![node_local],
                NodeDistributionStrategy::round_robin(),
            )
            .await,
        )
    } else {
        None
    };

    // Ensure no global commands to avoid duplicates alongside guild-scoped commands
    // Use serenity API to set an empty global command set
    if let Err(e) = serenity::Command::set_global_commands(&ctx.http, vec![]).await {
        warn!("Failed to clear global commands: {}", e);
    }
    // Register slash commands in all guilds we're currently in for fast availability
    // (Global propagation can take up to an hour; guild keeps things instant.)
    for g in &ready.guilds {
        let gid = g.id;
        if let Err(e) = poise::builtins::register_in_guild(ctx, &_framework.options().commands, gid).await {
            warn!("Failed to register commands in guild {}: {}", gid, e);
        }
    }

    Ok(Data {
        config,
        openai_client,
        lavalink: lavalink_client,
        db: get_db().await,
    })
}

async fn bot_entrypoint() {
    let config = moonbot_config::get_config();

    let commands = vec![
        commands::register::register_commands(),
        commands::meta::ping(),
        commands::meta::about(),
        commands::openai::askgpt(),
        commands::openai::genimage(),
    commands::openai::prompt_show(),
    commands::openai::prompt_set(),
    commands::openai::prompt_add(),
    commands::openai::prompt_clear(),
    commands::openai::status(),
    commands::openai::mood(),
    commands::openai::profile(),
    commands::userdir::command(),
    // Public RAG commands disabled; bot-only usage retained internally
    commands::userdir_admin::command(),
    commands::profile_admin::command(),
    commands::roleplay::command(),
        commands::music::join(),
        commands::music::leave(),
        commands::music::play(),
        commands::music::pause(),
        commands::music::resume(),
        commands::music::skip(),
        commands::music::queue(),
    ];

    let options = poise::FrameworkOptions {
        commands,
        prefix_options: poise::PrefixFrameworkOptions {
            prefix: Some("~".into()),
            execute_self_messages: false,
            execute_untracked_edits: true,
            mention_as_prefix: false,
            ..Default::default()
        },
        event_handler: |ctx, event, framework, data| {
            Box::pin(handlers::handler(ctx, event, framework, data))
        },
        on_error: |error| Box::pin(handlers::error_handler(error)),
        ..Default::default()
    };

    let framework = poise::Framework::builder()
        .setup(|ctx, ready, framework| Box::pin(on_ready(ctx, ready, framework)))
        .options(options)
        .build();

    let intents = serenity::GatewayIntents::all();

    let client = serenity::ClientBuilder::new(config.discord.token.as_str(), intents)
        .register_songbird()
        .framework(framework)
        .await;

    client.unwrap().start().await.unwrap()
}

fn main() {
    let config = moonbot_config::load_config();

    if config.discord.token.is_empty() {
        panic!("Discord token is not set in the configuration file");
    }

    if config.sentry.dsn.is_empty() {
        warn!("Sentry initialized with empty DSN - will be disabled")
    }

    let _guard = sentry::init((
        config.sentry.dsn.as_str(),
        sentry::ClientOptions {
            release: sentry::release_name!(),
            ..Default::default()
        },
    ));

    // Configure logging with Sentry
    let stdout_log = tracing_subscriber::fmt::layer()
        .compact()
        .with_filter(filter::LevelFilter::from(Level::INFO));

    tracing_subscriber::registry()
        .with(stdout_log)
        .with(sentry_tracing::layer())
        .init();

    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async { bot_entrypoint().await });
}
