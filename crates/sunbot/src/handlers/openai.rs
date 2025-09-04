use crate::{utils::is_reply_or_mention, Data, Error};
use async_openai::types::{
    ChatCompletionRequestAssistantMessageArgs, ChatCompletionRequestMessageContentPartImageArgs,
    ChatCompletionRequestMessageContentPartTextArgs, ChatCompletionRequestSystemMessageArgs,
    ChatCompletionRequestUserMessageArgs, ChatCompletionRequestUserMessageContentPart,
    CreateChatCompletionRequestArgs,
};
use poise::serenity_prelude as serenity;
use rand::Rng;
use std::sync::atomic::{AtomicU64, Ordering};
use tracing::{error, info};

static LAST_RESPONSE: AtomicU64 = AtomicU64::new(0);

// Generate a response to a message
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

    // Include system context (prefer global override if present)
    let sys_ctx = if let Some(db_ctx) =
        moonbot_db::get_global_system_context(framework.user_data.db).await
    {
        db_ctx
    } else {
        framework
            .user_data
            .config
            .openai
            .auto
            .system_context
            .clone()
    };

    // Send system context as a single consolidated system message to improve adherence
    if !sys_ctx.is_empty() {
        let sys_text = sys_ctx.join("\n");
        chat_messages.push(
            ChatCompletionRequestSystemMessageArgs::default()
                .content(sys_text)
                .build()
                .unwrap()
                .into(),
        );
    }

    // If the triggering message calls the bot "Sunbot", add a stronger, safe instruction.
    if message
        .content
        .to_lowercase()
        .contains("sunbot")
    {
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

        let request = CreateChatCompletionRequestArgs::default()
            .model(framework.user_data.config.openai.auto.model.as_str())
            .messages(chat_messages.clone())
            .max_tokens(framework.user_data.config.openai.auto.max_tokens)
            .build()?;

        let resp = client.chat().create(request).await?;

        // Send the response
        message
            .reply(
                ctx,
                resp.choices
                    .first()
                    .unwrap()
                    .message
                    .content
                    .as_ref()
                    .unwrap(),
            )
            .await?;
        Ok::<(), Error>(())
    };

    if let Err(e) = openai_tasks.await {
        error!("Error generating response: {:?}", e);
        info!("Request: {:?}", chat_messages);
    }

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
