use sea_orm::prelude::*;
use sea_orm::{ConnectOptions, Database};
use moonbot_migrations::{Migrator, MigratorTrait};
use tokio::sync::OnceCell;
use crate::entities::prelude::*;
use sea_orm::*;
use sea_query::OnConflict;
use chrono::Utc;
use serde_json;

static DB_CLIENT: OnceCell<DatabaseConnection> = OnceCell::const_new();

pub mod entities;

pub async fn init_db(database_url: &str) {
    let opt = ConnectOptions::new(database_url);
    let db = Database::connect(opt).await.unwrap();
    DB_CLIENT
        .set(db)
        .unwrap_or_else(|_| panic!("don't call `init_db()` more than once"));

    Migrator::up(get_db().await, None).await.unwrap();
}

pub async fn get_db() -> &'static DatabaseConnection {
    DB_CLIENT
        .get()
        .expect("called `get_db()` before db was initialized")
}

/// Fetch the global system_context if present. Returns None if not set or on parse error.
pub async fn get_global_system_context(db: &DatabaseConnection) -> Option<Vec<String>> {
    if let Ok(Some(model)) = GlobalPrompt::find_by_id(1).one(db).await {
        if let Ok(vec) = serde_json::from_str::<Vec<String>>(&model.prompt_json) {
            return Some(vec);
        }
    }
    None
}

/// Set/replace the global system_context (Vec<String>), stored as JSON.
pub async fn set_global_system_context(
    db: &DatabaseConnection,
    ctx: Vec<String>,
) -> Result<(), DbErr> {
    let json = serde_json::to_string(&ctx).unwrap_or("[]".to_string());
    let now = Utc::now();

    let am = crate::entities::global_prompt::ActiveModel {
        id: ActiveValue::set(1),
        prompt_json: ActiveValue::set(json),
        updated_at: ActiveValue::set(now),
    };

    GlobalPrompt::insert(am)
        .on_conflict(
            OnConflict::column(crate::entities::global_prompt::Column::Id)
                .update_columns([
                    crate::entities::global_prompt::Column::PromptJson,
                    crate::entities::global_prompt::Column::UpdatedAt,
                ])
                .to_owned(),
        )
        .exec(db)
        .await
        .map(|_| ())
}

/// Append a single line to the global system_context.
pub async fn add_global_system_context_line(
    db: &DatabaseConnection,
    line: String,
) -> Result<(), DbErr> {
    let mut current = get_global_system_context(db).await.unwrap_or_default();
    current.push(line);
    set_global_system_context(db, current).await
}

/// Clear the global system_context.
pub async fn clear_global_system_context(db: &DatabaseConnection) -> Result<(), DbErr> {
    set_global_system_context(db, Vec::new()).await
}

/// Fetch per-channel system_context if present. Returns None if not set or on parse error.
pub async fn get_channel_system_context(
    db: &DatabaseConnection,
    channel_id: i64,
) -> Option<Vec<String>> {
    if let Ok(Some(model)) = ChannelPrompt::find_by_id(channel_id).one(db).await {
        if let Ok(vec) = serde_json::from_str::<Vec<String>>(&model.prompt_json) {
            return Some(vec);
        }
    }
    None
}

/// Set/replace the per-channel system_context (Vec<String>), stored as JSON.
pub async fn set_channel_system_context(
    db: &DatabaseConnection,
    channel_id: i64,
    ctx: Vec<String>,
) -> Result<(), DbErr> {
    let json = serde_json::to_string(&ctx).unwrap_or("[]".to_string());
    let now = Utc::now();

    let am = crate::entities::channel_prompt::ActiveModel {
        channel_id: ActiveValue::set(channel_id),
        prompt_json: ActiveValue::set(json),
        updated_at: ActiveValue::set(now),
    };

    ChannelPrompt::insert(am)
        .on_conflict(
            OnConflict::column(crate::entities::channel_prompt::Column::ChannelId)
                .update_columns([
                    crate::entities::channel_prompt::Column::PromptJson,
                    crate::entities::channel_prompt::Column::UpdatedAt,
                ])
                .to_owned(),
        )
        .exec(db)
        .await
        .map(|_| ())
}

/// Add a single line to the per-channel system_context (append). Creates row if not exists.
pub async fn add_channel_system_context_line(
    db: &DatabaseConnection,
    channel_id: i64,
    line: String,
) -> Result<(), DbErr> {
    let mut current = get_channel_system_context(db, channel_id).await.unwrap_or_default();
    current.push(line);
    set_channel_system_context(db, channel_id, current).await
}

/// Clear the per-channel system_context.
pub async fn clear_channel_system_context(
    db: &DatabaseConnection,
    channel_id: i64,
) -> Result<(), DbErr> {
    set_channel_system_context(db, channel_id, Vec::new()).await
}
