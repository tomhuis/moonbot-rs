use sea_orm::prelude::*;
use sea_orm::{ConnectOptions, Database};
use moonbot_migrations::{Migrator, MigratorTrait};
use tokio::sync::OnceCell;
use crate::entities::prelude::*;
use sea_orm::*;
use sea_query::OnConflict;
use chrono::Utc;
use serde_json;
// use sea_orm::QueryOrder; // not used currently

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

// --- Bot disposition helpers ---

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default)]
pub struct Disposition {
    pub mood: String,       // e.g., "neutral", "playful", "curt"
    pub mood_level: i32,    // -5..5
    pub notes: String,      // free text or JSON
}

pub async fn get_bot_disposition(db: &DatabaseConnection) -> Option<Disposition> {
    if let Ok(Some(model)) = BotDisposition::find_by_id(1).one(db).await {
        Some(Disposition {
            mood: model.mood,
            mood_level: model.mood_level,
            notes: model.notes,
        })
    } else {
        None
    }
}

pub async fn set_bot_disposition(db: &DatabaseConnection, d: Disposition) -> Result<(), DbErr> {
    let now = Utc::now();
    let am = crate::entities::bot_disposition::ActiveModel {
        id: ActiveValue::set(1),
        mood: ActiveValue::set(d.mood),
        mood_level: ActiveValue::set(d.mood_level),
        notes: ActiveValue::set(d.notes),
        updated_at: ActiveValue::set(now),
    };

    BotDisposition::insert(am)
        .on_conflict(
            OnConflict::column(crate::entities::bot_disposition::Column::Id)
                .update_columns([
                    crate::entities::bot_disposition::Column::Mood,
                    crate::entities::bot_disposition::Column::MoodLevel,
                    crate::entities::bot_disposition::Column::Notes,
                    crate::entities::bot_disposition::Column::UpdatedAt,
                ])
                .to_owned(),
        )
        .exec(db)
        .await
        .map(|_| ())
}

// --- User insight helpers ---

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default)]
pub struct UserProfile {
    pub traits: Vec<String>,
    pub preferences: serde_json::Value, // object
    pub summary: String,
    pub trust_level: i32, // -5..5
}

pub async fn get_user_profile(db: &DatabaseConnection, user_id: i64) -> Option<UserProfile> {
    if let Ok(Some(model)) = UserInsight::find_by_id(user_id).one(db).await {
        let traits: Vec<String> = serde_json::from_str(&model.traits).unwrap_or_default();
        let preferences: serde_json::Value = serde_json::from_str(&model.preferences).unwrap_or(serde_json::json!({}));
        Some(UserProfile {
            traits,
            preferences,
            summary: model.summary,
            trust_level: model.trust_level,
        })
    } else {
        None
    }
}

pub async fn upsert_user_profile(
    db: &DatabaseConnection,
    user_id: i64,
    mut profile: UserProfile,
) -> Result<(), DbErr> {
    // Normalize
    if !profile.preferences.is_object() {
        profile.preferences = serde_json::json!({});
    }
    let traits_json = serde_json::to_string(&profile.traits).unwrap_or("[]".to_string());
    let prefs_json = serde_json::to_string(&profile.preferences).unwrap_or("{}".to_string());
    let now = Utc::now();

    let am = crate::entities::user_insight::ActiveModel {
        user_id: ActiveValue::set(user_id),
        traits: ActiveValue::set(traits_json),
        preferences: ActiveValue::set(prefs_json),
        summary: ActiveValue::set(profile.summary),
        trust_level: ActiveValue::set(profile.trust_level),
        first_seen: ActiveValue::not_set(),
        last_seen: ActiveValue::set(now),
    };

    UserInsight::insert(am)
        .on_conflict(
            OnConflict::column(crate::entities::user_insight::Column::UserId)
                .update_columns([
                    crate::entities::user_insight::Column::Traits,
                    crate::entities::user_insight::Column::Preferences,
                    crate::entities::user_insight::Column::Summary,
                    crate::entities::user_insight::Column::TrustLevel,
                    crate::entities::user_insight::Column::LastSeen,
                ])
                .to_owned(),
        )
        .exec(db)
        .await
        .map(|_| ())
}

pub async fn append_user_traits(db: &DatabaseConnection, user_id: i64, new_traits: &[String]) -> Result<(), DbErr> {
    let mut profile = get_user_profile(db, user_id).await.unwrap_or_default();
    let mut set: std::collections::BTreeSet<String> = profile.traits.into_iter().collect();
    for t in new_traits { set.insert(t.clone()); }
    profile.traits = set.into_iter().collect();
    upsert_user_profile(db, user_id, profile).await
}

pub async fn merge_user_preferences(db: &DatabaseConnection, user_id: i64, pref_delta: serde_json::Value) -> Result<(), DbErr> {
    let mut profile = get_user_profile(db, user_id).await.unwrap_or_default();
    let obj = profile.preferences.as_object_mut().unwrap();
    if let Some(delta) = pref_delta.as_object() {
        for (k, v) in delta.iter() { obj.insert(k.clone(), v.clone()); }
    }
    upsert_user_profile(db, user_id, profile).await
}

pub async fn remove_user_traits(db: &DatabaseConnection, user_id: i64, traits_to_remove: &[String]) -> Result<(), DbErr> {
    let mut profile = get_user_profile(db, user_id).await.unwrap_or_default();
    let remove: std::collections::BTreeSet<&String> = traits_to_remove.iter().collect();
    profile.traits.retain(|t| !remove.contains(t));
    upsert_user_profile(db, user_id, profile).await
}

pub async fn set_user_preference(db: &DatabaseConnection, user_id: i64, key: &str, value: serde_json::Value) -> Result<(), DbErr> {
    let mut profile = get_user_profile(db, user_id).await.unwrap_or_default();
    let obj = profile.preferences.as_object_mut().unwrap();
    obj.insert(key.to_string(), value);
    upsert_user_profile(db, user_id, profile).await
}

pub async fn remove_user_preference(db: &DatabaseConnection, user_id: i64, key: &str) -> Result<(), DbErr> {
    let mut profile = get_user_profile(db, user_id).await.unwrap_or_default();
    let obj = profile.preferences.as_object_mut().unwrap();
    obj.remove(key);
    upsert_user_profile(db, user_id, profile).await
}

pub async fn set_user_summary(db: &DatabaseConnection, user_id: i64, summary: String) -> Result<(), DbErr> {
    let mut profile = get_user_profile(db, user_id).await.unwrap_or_default();
    profile.summary = summary;
    upsert_user_profile(db, user_id, profile).await
}

pub async fn set_user_trust_level(db: &DatabaseConnection, user_id: i64, trust_level: i32) -> Result<(), DbErr> {
    let mut profile = get_user_profile(db, user_id).await.unwrap_or_default();
    profile.trust_level = trust_level.clamp(-5, 5);
    upsert_user_profile(db, user_id, profile).await
}

pub async fn delete_user_profile(db: &DatabaseConnection, user_id: i64) -> Result<u64, DbErr> {
    let res = UserInsight::delete_by_id(user_id).exec(db).await?;
    Ok(res.rows_affected)
}

// --- User directory (RAG) helpers ---

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default)]
pub struct UserDirectoryEntry {
    pub user_id: i64,
    pub display_name: String,
    pub aliases: Vec<String>,
    pub notes: String,
}

pub async fn upsert_user_directory(db: &DatabaseConnection, entry: UserDirectoryEntry) -> Result<(), DbErr> {
    let now = chrono::Utc::now();
    let aliases_json = serde_json::to_string(&entry.aliases).unwrap_or("[]".to_string());
    let am = crate::entities::user_directory::ActiveModel {
        user_id: ActiveValue::set(entry.user_id),
        display_name: ActiveValue::set(entry.display_name),
        aliases: ActiveValue::set(aliases_json),
        notes: ActiveValue::set(entry.notes),
        updated_at: ActiveValue::set(now),
    };
    crate::entities::user_directory::Entity::insert(am)
        .on_conflict(
            OnConflict::column(crate::entities::user_directory::Column::UserId)
                .update_columns([
                    crate::entities::user_directory::Column::DisplayName,
                    crate::entities::user_directory::Column::Aliases,
                    crate::entities::user_directory::Column::Notes,
                    crate::entities::user_directory::Column::UpdatedAt,
                ])
                .to_owned(),
        )
        .exec(db)
        .await
        .map(|_| ())
}

/// Full-text search for users by a free-text query. Returns up to `limit` matches ordered by rank.
pub async fn search_users_fts(db: &DatabaseConnection, query: &str, limit: i64) -> Result<Vec<UserDirectoryEntry>, DbErr> {
    // Use FTS5 match query over the external content table; order by bm25 (rank)
    let sql = r#"
        SELECT u.user_id, u.display_name, u.aliases, u.notes
        FROM user_directory_fts f
        JOIN user_directory u ON u.user_id = f.rowid
        WHERE f.user_directory_fts MATCH ?
        ORDER BY bm25(f)
        LIMIT ?
    "#;
    let rows = db
        .query_all(Statement::from_sql_and_values(
            DbBackend::Sqlite,
            sql,
            vec![Value::from(query), Value::from(limit)],
        ))
        .await?;
    let mut out = Vec::new();
    for row in rows {
        let user_id: i64 = row.try_get("", "user_id").unwrap_or_default();
        let display_name: String = row.try_get("", "display_name").unwrap_or_default();
        let aliases_str: String = row.try_get("", "aliases").unwrap_or_else(|_| "[]".into());
        let notes: String = row.try_get("", "notes").unwrap_or_default();
        let aliases: Vec<String> = serde_json::from_str(&aliases_str).unwrap_or_default();
        out.push(UserDirectoryEntry { user_id, display_name, aliases, notes });
    }
    Ok(out)
}

/// Fallback LIKE search if FTS tables are missing
pub async fn search_users_like(db: &DatabaseConnection, query: &str, limit: i64) -> Result<Vec<UserDirectoryEntry>, DbErr> {
    let like = format!("%{}%", query);
    let sql = r#"
        SELECT user_id, display_name, aliases, notes
        FROM user_directory
        WHERE display_name LIKE ? OR aliases LIKE ? OR notes LIKE ?
        LIMIT ?
    "#;
    let rows = db
        .query_all(Statement::from_sql_and_values(
            DbBackend::Sqlite,
            sql,
            vec![Value::from(like.clone()), Value::from(like.clone()), Value::from(like), Value::from(limit)],
        ))
        .await?;
    let mut out = Vec::new();
    for row in rows {
        let user_id: i64 = row.try_get("", "user_id").unwrap_or_default();
        let display_name: String = row.try_get("", "display_name").unwrap_or_default();
        let aliases_str: String = row.try_get("", "aliases").unwrap_or_else(|_| "[]".into());
        let notes: String = row.try_get("", "notes").unwrap_or_default();
        let aliases: Vec<String> = serde_json::from_str(&aliases_str).unwrap_or_default();
        out.push(UserDirectoryEntry { user_id, display_name, aliases, notes });
    }
    Ok(out)
}

pub async fn get_user_directory_entry(db: &DatabaseConnection, user_id: i64) -> Option<UserDirectoryEntry> {
    if let Ok(Some(m)) = crate::entities::user_directory::Entity::find_by_id(user_id).one(db).await {
        let aliases: Vec<String> = serde_json::from_str(&m.aliases).unwrap_or_default();
        Some(UserDirectoryEntry { user_id: m.user_id, display_name: m.display_name, aliases, notes: m.notes })
    } else { None }
}

/// Delete a user directory entry by user_id.
pub async fn delete_user_directory(db: &DatabaseConnection, user_id: i64) -> Result<u64, DbErr> {
    let res = crate::entities::user_directory::Entity::delete_by_id(user_id).exec(db).await?;
    Ok(res.rows_affected)
}

/// Count all user directory entries.
pub async fn count_user_directory(db: &DatabaseConnection) -> Result<u64, DbErr> {
    use sea_orm::EntityTrait;
    let count = crate::entities::user_directory::Entity::find().count(db).await?;
    Ok(count as u64)
}

// --- Channel roleplay persona ---
pub async fn set_channel_roleplay(db: &DatabaseConnection, channel_id: i64, persona: String) -> Result<(), DbErr> {
    let am = crate::entities::channel_roleplay::ActiveModel {
        channel_id: ActiveValue::set(channel_id),
        persona: ActiveValue::set(persona),
        updated_at: ActiveValue::set(chrono::Utc::now()),
    };
    crate::entities::channel_roleplay::Entity::insert(am)
        .on_conflict(
            OnConflict::column(crate::entities::channel_roleplay::Column::ChannelId)
                .update_columns([
                    crate::entities::channel_roleplay::Column::Persona,
                    crate::entities::channel_roleplay::Column::UpdatedAt,
                ])
                .to_owned(),
        )
        .exec(db)
        .await
        .map(|_| ())
}

pub async fn get_channel_roleplay(db: &DatabaseConnection, channel_id: i64) -> Option<String> {
    if let Ok(Some(m)) = crate::entities::channel_roleplay::Entity::find_by_id(channel_id).one(db).await {
        Some(m.persona)
    } else { None }
}

pub async fn clear_channel_roleplay(db: &DatabaseConnection, channel_id: i64) -> Result<u64, DbErr> {
    let res = crate::entities::channel_roleplay::Entity::delete_by_id(channel_id).exec(db).await?;
    Ok(res.rows_affected)
}
// --- General RAG corpus (Discord/user/channel context) ---
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct CorpusEntry { pub id: i64, pub guild_id: Option<i64>, pub channel_id: Option<i64>, pub user_id: Option<i64>, pub kind: String, pub content: String, pub created_at: chrono::DateTime<chrono::Utc> }

pub async fn upsert_corpus_entry(
    db: &DatabaseConnection,
    guild_id: Option<i64>, channel_id: Option<i64>, user_id: Option<i64>, kind: &str, content: &str,
) -> Result<i64, DbErr> {
    // Insert new row; id auto-increments (SQLite). Return last_insert_id.
    let now = chrono::Utc::now();
    let stmt = sea_orm::Statement::from_sql_and_values(
        db.get_database_backend(),
        "INSERT INTO corpus (guild_id, channel_id, user_id, kind, content, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        vec![
            match guild_id { Some(v) => sea_orm::Value::BigInt(Some(v)), None => sea_orm::Value::BigInt(None) },
            match channel_id { Some(v) => sea_orm::Value::BigInt(Some(v)), None => sea_orm::Value::BigInt(None) },
            match user_id { Some(v) => sea_orm::Value::BigInt(Some(v)), None => sea_orm::Value::BigInt(None) },
            sea_orm::Value::String(Some(Box::new(kind.to_string()))),
            sea_orm::Value::String(Some(Box::new(content.to_string()))),
            now.into(),
        ],
    );
    let res = db.execute(stmt).await?;
    Ok(res.last_insert_id() as i64)
}

pub async fn search_corpus_fts(db: &DatabaseConnection, query: &str, k: u64, guild_id: Option<i64>, channel_id: Option<i64>) -> Result<Vec<CorpusEntry>, DbErr> {
        // FTS search with optional guild/channel filtering using nullable parameters
        let sql = "SELECT c.id, c.guild_id, c.channel_id, c.user_id, c.kind, c.content, c.created_at \
                             FROM corpus_fts f JOIN corpus c ON c.id = f.rowid \
                             WHERE corpus_fts MATCH ?1 \
                                 AND (?3 IS NULL OR c.guild_id = ?3) \
                                 AND (?4 IS NULL OR c.channel_id = ?4) \
                             ORDER BY c.created_at DESC LIMIT ?2";
        let params: Vec<sea_orm::Value> = vec![
                query.into(),
                (k as i64).into(),
                match guild_id { Some(v) => sea_orm::Value::BigInt(Some(v)), None => sea_orm::Value::BigInt(None) },
                match channel_id { Some(v) => sea_orm::Value::BigInt(Some(v)), None => sea_orm::Value::BigInt(None) },
        ];
        let stmt = sea_orm::Statement::from_sql_and_values(db.get_database_backend(), sql, params);
        let rows = db.query_all(stmt).await?;
    let mut out = Vec::new();
    for r in rows {
        out.push(CorpusEntry {
            id: r.try_get("", "id").unwrap_or_default(),
            guild_id: r.try_get("", "guild_id").ok(),
            channel_id: r.try_get("", "channel_id").ok(),
            user_id: r.try_get("", "user_id").ok(),
            kind: r.try_get("", "kind").unwrap_or_default(),
            content: r.try_get("", "content").unwrap_or_default(),
            created_at: r.try_get("", "created_at").unwrap_or_else(|_| chrono::Utc::now()),
        });
    }
    Ok(out)
}

// --- Guild roleplay persona ---
pub async fn set_guild_roleplay(db: &DatabaseConnection, guild_id: i64, persona: String) -> Result<(), DbErr> {
    let am = crate::entities::guild_roleplay::ActiveModel {
        guild_id: ActiveValue::set(guild_id),
        persona: ActiveValue::set(persona),
        updated_at: ActiveValue::set(chrono::Utc::now()),
    };
    crate::entities::guild_roleplay::Entity::insert(am)
        .on_conflict(
            OnConflict::column(crate::entities::guild_roleplay::Column::GuildId)
                .update_columns([
                    crate::entities::guild_roleplay::Column::Persona,
                    crate::entities::guild_roleplay::Column::UpdatedAt,
                ])
                .to_owned(),
        )
        .exec(db)
        .await
        .map(|_| ())
}

pub async fn get_guild_roleplay(db: &DatabaseConnection, guild_id: i64) -> Option<String> {
    if let Ok(Some(m)) = crate::entities::guild_roleplay::Entity::find_by_id(guild_id).one(db).await {
        Some(m.persona)
    } else { None }
}

pub async fn clear_guild_roleplay(db: &DatabaseConnection, guild_id: i64) -> Result<u64, DbErr> {
    let res = crate::entities::guild_roleplay::Entity::delete_by_id(guild_id).exec(db).await?;
    Ok(res.rows_affected)
}
