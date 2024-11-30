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
use tracing::info;

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

    // Include system context
    for ctx in framework.user_data.config.openai.auto.system_context.iter() {
        chat_messages.push(
            ChatCompletionRequestSystemMessageArgs::default()
                .content(ctx.as_str())
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

        chat_messages.push(
            ChatCompletionRequestUserMessageArgs::default()
                .content(user_content)
                .name(format!("{}__{}", msg.author.name.as_str(), msg.author.id))
                .build()
                .unwrap()
                .into(),
        );
    }

    let client = framework.user_data.openai_client.as_ref().unwrap();

    let request = CreateChatCompletionRequestArgs::default()
        .model(framework.user_data.config.openai.auto.model.as_str())
        .messages(chat_messages)
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
    Ok(())
}

// Handle replies to the Bot
pub async fn handle_reply(
    ctx: &serenity::Context,
    framework: poise::FrameworkContext<'_, Data, Error>,
    message: &serenity::Message,
) -> Result<(), Error> {
    if message.author.bot || message.content.is_empty() {
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
    if rand::thread_rng().gen::<f64>()
        < framework.user_data.config.openai.auto.random.trigger_chance
    {
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
