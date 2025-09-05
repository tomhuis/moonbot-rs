use crate::{utils::is_reply_or_mention, Data, Error};
use crate::context;
use async_openai::types::{
    ChatCompletionRequestAssistantMessageArgs, ChatCompletionRequestMessageContentPartImageArgs,
    ChatCompletionRequestMessageContentPartTextArgs, ChatCompletionRequestSystemMessageArgs,
    ChatCompletionRequestUserMessageArgs, ChatCompletionRequestUserMessageContentPart,
    CreateChatCompletionRequestArgs,
};
use sea_orm::DatabaseConnection;
use poise::serenity_prelude as serenity;
use rand::Rng;
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::time::{sleep, Duration};
use tracing::{error, info};
use moonbot_db as db;

static LAST_RESPONSE: AtomicU64 = AtomicU64::new(0);

// Generate a response to a message
pub async fn analyze_and_update(
    db: &DatabaseConnection,
    user_id: i64,
    user_name: &str,
    user_msg: &str,
    personalize: bool,
) {
    if !personalize { return; }
    // Very lightweight heuristics; safe and bounded
    let mut traits = vec![];
    let lc = user_msg.to_lowercase();
    if user_msg.len() > 120 { traits.push("verbose".to_string()); } else if user_msg.len() < 25 { traits.push("terse".to_string()); }
    if lc.contains("please") { traits.push("polite".to_string()); }
    if lc.ends_with('?') || lc.contains("? ") { traits.push("inquisitive".to_string()); }
    if lc.contains("thank") { traits.push("grateful".to_string()); }
    if lc.contains("lol") || lc.contains("haha") || lc.contains("😂") { traits.push("playful".to_string()); }
    // Interests / domains
    if lc.contains("rust ") || lc.contains("cargo ") { traits.push("rustacean".to_string()); }
    if lc.contains("python ") { traits.push("pythonista".to_string()); }
    if lc.contains("docker") || lc.contains("kubernetes") { traits.push("devops".to_string()); }
    if lc.contains("music") || lc.contains("playlist") { traits.push("music_fan".to_string()); }
    // Preference hints
    let mut pref_delta = serde_json::Map::new();
    if lc.contains("concise") { pref_delta.insert("style".into(), serde_json::Value::from("concise")); }
    if lc.contains("detailed") { pref_delta.insert("style".into(), serde_json::Value::from("detailed")); }
    if lc.contains("emoji") { pref_delta.insert("emoji".into(), serde_json::Value::from(true)); }
    if lc.contains("no emoji") || lc.contains("no emojis") { pref_delta.insert("emoji".into(), serde_json::Value::from(false)); }
    if lc.contains("code block") || lc.contains("code-block") { pref_delta.insert("code_blocks".into(), serde_json::Value::from(true)); }
    if lc.contains("no code block") { pref_delta.insert("code_blocks".into(), serde_json::Value::from(false)); }

    let _ = moonbot_db::append_user_traits(db, user_id, &traits).await;
    if !pref_delta.is_empty() { let _ = moonbot_db::merge_user_preferences(db, user_id, serde_json::Value::Object(pref_delta)).await; }

    // Update profile summary and trust
    let mut profile = moonbot_db::get_user_profile(db, user_id).await.unwrap_or_default();
    if profile.summary.is_empty() { profile.summary = format!("Known as {}. {} trait(s).", user_name, traits.len()); }
    // naive trust tweak
    let polite = lc.contains("please");
    profile.trust_level = (profile.trust_level + if polite {1} else {0}).clamp(-5, 5);
    let _ = moonbot_db::upsert_user_profile(db, user_id, profile).await;

    // Mood shifts based on recent interaction volume (toy example)
    if let Some(mut d) = moonbot_db::get_bot_disposition(db).await { 
        if user_msg.len() > 200 { d.mood_level = (d.mood_level + 1).clamp(-5,5); }
        let _ = moonbot_db::set_bot_disposition(db, d).await;
    } else {
        let _ = moonbot_db::set_bot_disposition(db, moonbot_db::Disposition { mood: "neutral".into(), mood_level: 0, notes: "".into() }).await;
    }
}
pub async fn generate_response(
    ctx: &serenity::Context,
    framework: poise::FrameworkContext<'_, Data, Error>,
    message: &serenity::Message,
) -> Result<(), Error> {
    // Gather some context
    let mut messages = ctx
        .http
        .get_messages(
            message.channel_id,
            Some(serenity::MessagePagination::Before(message.id)),
            Some(framework.user_data.config.openai.auto.max_messages),
        )
        .await?;

    messages.insert(0, message.clone());

    let mut chat_messages: Vec<async_openai::types::ChatCompletionRequestMessage> = vec![];

    // Centralized system prompt
    let sys_base = context::build_system_prompt(
        framework.user_data,
        message.author.id.get() as i64,
        &message.author.name,
    ).await;
    let sys_text = if let Some(guild_id) = message.guild_id {
        if let Some(persona) = moonbot_db::get_guild_roleplay(framework.user_data.db, guild_id.get() as i64).await {
            info!("roleplay=guild scope applied");
            format!("### Roleplay persona (guild-wide)\n{}\n\n### Instruction\nStay in the above persona for this conversation. Reflect its style and diction consistently. Avoid generic chatbot greetings.\n\n{}", persona, sys_base)
        } else if let Some(persona) = moonbot_db::get_channel_roleplay(framework.user_data.db, message.channel_id.get() as i64).await {
            info!("roleplay=channel scope applied");
            format!("### Roleplay persona\n{}\n\n### Instruction\nStay in the above persona for this conversation. Reflect its style and diction consistently. Avoid generic chatbot greetings.\n\n{}", persona, sys_base)
        } else { sys_base }
    } else { sys_base };
    // Lightweight retrieval from corpus (guild/channel scoped)
    let mut retrieved = String::new();
    if let Ok(hits) = db::search_corpus_fts(
        framework.user_data.db,
        &message.content,
        6,
        message.guild_id.map(|g| g.get() as i64),
        Some(message.channel_id.get() as i64),
    ).await {
        if !hits.is_empty() {
            for h in hits.iter().take(6) { retrieved.push_str(&format!("- [{}] {}\n", h.kind, h.content)); }
        }
    }

    if !sys_text.is_empty() {
        chat_messages.push(
            ChatCompletionRequestSystemMessageArgs::default()
                .content(if retrieved.is_empty() { sys_text.clone() } else { format!("{}\n\n### Retrieved context\n{}", sys_text, retrieved) })
                .build()
                .unwrap()
                .into(),
        );
    }

    // If the triggering message calls the bot "Sunbot", add a stronger, safe instruction.
    let sunbot_misname = message.content.to_lowercase().contains("sunbot");
    let directed = is_reply_or_mention(ctx, message, framework.bot_id).await;
    if sunbot_misname && directed {
        chat_messages.push(
            ChatCompletionRequestSystemMessageArgs::default()
                .content("If the user refers to you as 'Sunbot', respond with exactly one curt sentence that corrects the name to 'Moonbot'. Use 2-6 words. No emojis or flourish. Output only that sentence—nothing else. Keep it PG-13, no profanity or slurs, and do not target or insult any person or group.")
                .build()
                .unwrap()
                .into(),
        );
    }

    for msg in messages.iter().rev() {
        // If this message is too old ignore it
        let diff = message.timestamp.timestamp() - msg.timestamp.timestamp();
        if diff > framework.user_data.config.openai.auto.max_message_age {
            continue;
        }

        // If this is sent by us use ChatCompletionRequestAssistantMessage
        if msg.author.id == framework.bot_id {
            chat_messages.push(
                ChatCompletionRequestAssistantMessageArgs::default()
                    .content(msg.content.as_str())
                    .name(msg.author.name.as_str())
                    .build()
                    .unwrap()
                    .into(),
            );
            continue;
        }
        // Otherwise ignore messages from other bots
        else if msg.author.bot {
            continue;
        }

        // Otherwise, this is a user message
        let mut user_content: Vec<ChatCompletionRequestUserMessageContentPart> =
            vec![ChatCompletionRequestMessageContentPartTextArgs::default()
                .text(msg.content.as_str())
                .build()
                .unwrap()
                .into()];

        // If we have use_vision enabled
        if framework.user_data.config.openai.auto.use_vision {
            for attachment in msg.attachments.iter() {
                if let Some(content_type) = attachment.content_type.as_deref() {
                    if content_type.to_lowercase().starts_with("image") {
                        info!("Found image attachment: {}", attachment.url.as_str());
                        user_content.push(
                            ChatCompletionRequestMessageContentPartImageArgs::default()
                                .image_url(attachment.url.as_str())
                                .build()
                                .unwrap()
                                .into(),
                        );
                    }
                }
            }
        }

        // OpenAI is very strict about the name, we need to make sure it matches ^[a-zA-Z0-9_-]+$
        // Remove any special characters, and replace spaces with underscores
        let username = msg
            .author
            .name
            .replace(' ', "_")
            .chars()
            .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
            .collect::<String>();

        // Not sure what is causing this, log the changes so we might know more
        if username != msg.author.name {
            info!("Changed username from {} to {}", msg.author.name, username);
        }

        chat_messages.push(
            ChatCompletionRequestUserMessageArgs::default()
                .content(user_content)
                .name(format!("{}__{}", username, msg.author.id))
                .build()
                .unwrap()
                .into(),
        );
    }

    let openai_tasks = async {
        let client = framework.user_data.openai_client.as_ref().unwrap();

        // Simple retries for transient failures
        let mut last_err: Option<String> = None;
        let resp = {
            let mut out = None;
            for (i, delay_ms) in [200u64, 500, 1000].into_iter().enumerate() {
                // Adaptive generation params based on profile/preferences
                let (temp, freq_pen) = crate::context::compute_generation_params(
                    framework.user_data,
                    message.author.id.get() as i64,
                    framework.user_data.config.openai.auto.temperature,
                    framework.user_data.config.openai.auto.frequency_penalty,
                ).await;
                let request = CreateChatCompletionRequestArgs::default()
                    .model(framework.user_data.config.openai.auto.model.as_str())
                    .messages(chat_messages.clone())
                    .max_tokens(framework.user_data.config.openai.auto.max_tokens)
                    .temperature(temp)
                    .frequency_penalty(freq_pen)
                    .user(format!("guild:{}|chan:{}|user:{}",
                        message.guild_id.map(|g| g.get()).unwrap_or_default(),
                        message.channel_id.get(),
                        message.author.id.get()
                    ))
                    .build()?;
                match client.chat().create(request).await {
                    Ok(r) => { out = Some(r); break; },
                    Err(e) => {
                        last_err = Some(format!("{}", e));
                        if i < 2 { sleep(Duration::from_millis(delay_ms)).await; }
                    }
                }
            }
            if let Some(r) = out { r } else {
                return Err(std::io::Error::new(std::io::ErrorKind::Other, last_err.unwrap_or_else(|| "unknown error".into())).into());
            }
        };

        // Send the response
        let reply_text = resp.choices
            .first()
            .unwrap()
            .message
            .content
            .as_ref()
            .unwrap()
            .clone();
        message.reply(ctx, &reply_text).await?;

        // Fire and forget: naive analysis to update dispositions and user insight
        let db = framework.user_data.db.clone();
        let user_id = message.author.id.get() as i64;
        let user_name = message.author.name.clone();
        let user_msg = message.content.clone();
        let personalize = framework.user_data.config.openai.auto.personalize;
    tokio::spawn(async move { analyze_and_update(&db, user_id, &user_name, &user_msg, personalize).await; });
        Ok::<(), Error>(())
    };

    if let Err(e) = openai_tasks.await {
        error!("Error generating response: {:?}", e);
        info!("Request: {:?}", chat_messages);
    }

    Ok(())
}

// Analyze every user message (even if we don't reply)
pub async fn handle_analysis_only(
    _ctx: &serenity::Context,
    framework: poise::FrameworkContext<'_, Data, Error>,
    message: &serenity::Message,
) -> Result<(), Error> {
    if message.author.bot || message.content.is_empty() {
        return Ok(());
    }
    // Ingest message into RAG corpus (best-effort)
    let _ = db::upsert_corpus_entry(
        framework.user_data.db,
        message.guild_id.map(|g| g.get() as i64),
        Some(message.channel_id.get() as i64),
        Some(message.author.id.get() as i64),
        "discord_message",
        &message.content,
    ).await;
    let personalize = framework.user_data.config.openai.auto.personalize;
    let db = framework.user_data.db;
    let user_id = message.author.id.get() as i64;
    let user_name = message.author.name.clone();
    let user_msg = message.content.clone();
    tokio::spawn(async move { analyze_and_update(db, user_id, &user_name, &user_msg, personalize).await; });
    Ok(())
}
// Handle replies to the Bot
pub async fn handle_reply(
    ctx: &serenity::Context,
    framework: poise::FrameworkContext<'_, Data, Error>,
    message: &serenity::Message,
) -> Result<(), Error> {
    if message.content.is_empty() {
        return Ok(());
    }

    if is_reply_or_mention(ctx, message, framework.bot_id).await {
        // Skip replies to low-content/insult-only messages to avoid annoyance
        let lc = message.content.to_lowercase();
        let word_count = lc.split_whitespace().count();
        let insults = ["stupid", "idiot", "dumb", "moron", "oaf"];
        let insult_only = word_count <= 3 && insults.iter().any(|w| lc.contains(w));
        if insult_only {
            return Ok(());
        }

        // Intent: tag <query> — return mentions based on user directory/RAG
        // Looks for the first occurrence of the token "tag" and treats the remainder of the message as the query.
        if let Some(idx) = message
            .content
            .split_whitespace()
            .position(|t| t.eq_ignore_ascii_case("tag"))
        {
            let parts: Vec<&str> = message.content.split_whitespace().collect();
            if parts.len() > idx + 1 {
                let query = parts[idx + 1..].join(" ");
                // If the message already mentions users (other than the bot), just echo them (ensures proper tags)
                let mut mentions: Vec<String> = message
                    .mentions
                    .iter()
                    .filter(|u| u.id != framework.bot_id)
                    .map(|u| format!("<@{}>", u.id.get()))
                    .collect();

                if mentions.is_empty() {
                    // Use FTS first, fallback to LIKE
                    let list = match db::search_users_fts(framework.user_data.db, &query, 5).await {
                        Ok(v) => v,
                        Err(_) => db::search_users_like(framework.user_data.db, &query, 5).await.unwrap_or_default(),
                    };
                    for e in list { mentions.push(format!("<@{}>", e.user_id)); }
                }

                if mentions.is_empty() {
                    message.reply(ctx, "No matches.").await?;
                } else {
                    message.reply(ctx, mentions.join(" ")).await?;
                }
                return Ok(());
            }
        }
    info!("Triggered Reply on message: {}", message.content);
        return generate_response(ctx, framework, message).await;
    }

    Ok(())
}

pub async fn handle_random_message(
    ctx: &serenity::Context,
    framework: poise::FrameworkContext<'_, Data, Error>,
    message: &serenity::Message,
) -> Result<(), Error> {
    if message.author.bot || message.content.is_empty() {
        return Ok(());
    }

    if is_reply_or_mention(ctx, message, framework.bot_id).await {
        return Ok(());
    }

    // Message must be longer than min length
    if message.content.len() < framework.user_data.config.openai.auto.random.min_length as usize {
        return Ok(());
    }

    // Heuristic: only consider random replies if user shows intent (a question or actionable cue)
    let lc = message.content.to_lowercase();
    let looks_like_question = lc.contains('?');
    let has_intent_keyword = [
        "how", "what", "why", "fix", "explain", "help", "debug", "solve", "idea", "show", "code",
    ]
    .iter()
    .any(|w| lc.contains(w));
    if !(looks_like_question || has_intent_keyword) {
        return Ok(());
    }

    // Check Cooldown
    let last_response = LAST_RESPONSE.load(Ordering::Relaxed);
    if last_response + framework.user_data.config.openai.auto.random.cooldown
        > std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
    {
        return Ok(());
    }

    // Roll the dice
    if rand::rng().random::<f64>() < framework.user_data.config.openai.auto.random.trigger_chance {
        info!(
            "Trigggered Random Reply on random message: {}",
            message.content
        );
        let result = generate_response(ctx, framework, message).await;
        // If we responded, update the last response time
        if result.is_ok() {
            LAST_RESPONSE.store(
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
                Ordering::Relaxed,
            );
        }
        return result;
    }

    Ok(())
}
