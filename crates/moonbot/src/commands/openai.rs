use std::vec;

use crate::{Context, Error};
use crate::context;
use async_openai::types::{
    ChatCompletionRequestUserMessageArgs, CreateChatCompletionRequestArgs,
    CreateImageRequestArgs, Image, ImageModel, ImageResponseFormat, ImageSize,
};
use tokio::time::{sleep, Duration};
use base64::prelude::*;
use serenity::{
    all::{CreateAttachment, EditMessage},
    builder::CreateEmbed,
};
use poise::serenity_prelude::User;

/// Ask a question to OpenAI
#[poise::command(slash_command, rename = "askgpt")]
pub async fn askgpt(
    ctx: Context<'_>,
    #[description = "The prompt to send to OpenAI"] prompt: String,
    #[description = "Use personalization (profile/mood)"] personalize: Option<bool>,
) -> Result<(), Error> {
    let Some(client) = ctx.data().openai_client.as_ref() else {
        ctx.say("OpenAI is not configured.").await?;
        return Ok(());
    };
    // Defer so we don't hit Discord's 3s interaction timeout
    ctx.defer().await?;
    // Build message list; centralized system prompt for consistency
    let mut msgs: Vec<async_openai::types::ChatCompletionRequestMessage> = vec![];
    if personalize.unwrap_or(false) && ctx.data().config.openai.auto.personalize {
        let mut sys = context::build_system_prompt(ctx.data(), ctx.author().id.get() as i64, ctx.author().name.as_str()).await;
        if let Some(guild_id) = ctx.guild_id() {
            if let Some(p) = moonbot_db::get_guild_roleplay(ctx.data().db, guild_id.get() as i64).await {
                sys = format!("### Roleplay persona (guild-wide)\n{}\n\n### Instruction\nStay in the above persona for this conversation. Reflect its style and diction consistently. Avoid generic chatbot greetings.\n\n{}", p, sys);
            } else if let Some(p) = moonbot_db::get_channel_roleplay(ctx.data().db, ctx.channel_id().get() as i64).await {
                sys = format!("### Roleplay persona\n{}\n\n### Instruction\nStay in the above persona for this conversation. Reflect its style and diction consistently. Avoid generic chatbot greetings.\n\n{}", p, sys);
            }
        }
        if !sys.is_empty() {
            msgs.push(async_openai::types::ChatCompletionRequestSystemMessageArgs::default()
                .content(sys).build()?.into());
        }
    }
    let user_msg = ChatCompletionRequestUserMessageArgs::default().content(prompt).build()?;
    msgs.push(user_msg.clone().into());

    // Simple retry with backoff
    let mut last_err: Option<String> = None;
    let resp = {
        let mut out = None;
        for (i, delay_ms) in [200u64, 500, 1000].into_iter().enumerate() {
            // Adaptive params
            let (temp, freq_pen) = context::compute_generation_params(
                ctx.data(),
                ctx.author().id.get() as i64,
                ctx.data().config.openai.askgpt.temperature,
                ctx.data().config.openai.askgpt.frequency_penalty,
            ).await;
            let request = CreateChatCompletionRequestArgs::default()
                .model(ctx.data().config.openai.askgpt.model.as_str())
                .messages(msgs.clone())
                .max_tokens(ctx.data().config.openai.askgpt.max_tokens)
                .temperature(temp)
                .frequency_penalty(freq_pen)
                .user(ctx.author().id.get().to_string())
                .build()?;
        match client.chat().create(request).await {
                Ok(r) => { out = Some(r); break; },
                Err(e) => {
            last_err = Some(format!("{}", e));
                    if i < 2 { sleep(Duration::from_millis(delay_ms)).await; }
                }
            }
        }
    if let Some(r) = out { r } else { return Err(std::io::Error::new(std::io::ErrorKind::Other, last_err.unwrap_or_else(|| "unknown error".into())).into()); }
    };
    ctx.say(
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

#[derive(Debug, poise::ChoiceParameter)]
pub enum ImageSizeType {
    #[name = "256x256"]
    Small,
    #[name = "512x512"]
    Medium,
    #[name = "1024x1024"]
    Large,
}

/// Generates an image using OpenAI
#[poise::command(slash_command, rename = "genimage")]
pub async fn genimage(
    ctx: Context<'_>,
    #[description = "The prompt to send to OpenAI"] prompt: String,
    #[description = "The size of the image to generate"] size: Option<ImageSizeType>,
    #[description = "The number of images to generate"] amount: Option<u8>,
) -> Result<(), Error> {
    let Some(client) = ctx.data().openai_client.as_ref() else {
        ctx.say("OpenAI is not configured.").await?;
        return Ok(());
    };
    // Gate image generation when using custom API bases that likely don't support images
    if !ctx.data().config.openai.api_base.is_empty()
        && !ctx.data().config.openai.api_base.contains("/v1")
    {
        ctx.say("Image generation not supported for this API base.").await?;
        return Ok(());
    }
    // Defer early to avoid interaction timeout
    ctx.defer().await?;

    let image_size = match size.unwrap_or(ImageSizeType::Large) {
        ImageSizeType::Small => ImageSize::S256x256,
        ImageSizeType::Medium => ImageSize::S512x512,
        ImageSizeType::Large => ImageSize::S1024x1024,
    };

    let model = match ctx.data().config.openai.genimage.model.as_str() {
        "dall-e-2" => ImageModel::DallE2,
        "dall-e-3" => ImageModel::DallE3,
        _ => ImageModel::Other(ctx.data().config.openai.genimage.model.clone()),
    };

    let mut embed = CreateEmbed::new()
        .title("Please wait generating images")
        .field("Prompt", prompt.as_str(), false)
        .field("Requstor", format!("<@!{}>", ctx.author().id), false);

    let n = if model == ImageModel::DallE3 {
        if amount.unwrap_or(1) > 1 {
            embed = embed.description("NOTE: Only 1 image can be generated with DALL-E 3");
        }
        1
    } else {
        amount.unwrap_or(1)
    };

    let reply = ctx.send(poise::CreateReply::default().embed(embed)).await?;

    // Retry for image generation as well
    let mut last_err: Option<String> = None;
    let resp = {
        let mut out = None;
        for (i, delay_ms) in [200u64, 500, 1000].into_iter().enumerate() {
            let req = CreateImageRequestArgs::default()
                .model(model.clone())
                .n(n)
                .prompt(prompt.as_str())
                .size(image_size)
                .response_format(ImageResponseFormat::B64Json)
                .build()
                .unwrap();
            match client.images().create(req).await {
                Ok(r) => { out = Some(r); break; },
                Err(e) => {
                    last_err = Some(format!("{}", e));
                    if i < 2 { sleep(Duration::from_millis(delay_ms)).await; }
                }
            }
        }
        if let Some(r) = out { r } else { return Err(std::io::Error::new(std::io::ErrorKind::Other, last_err.unwrap_or_else(|| "unknown error".into())).into()); }
    };

    let mut builder = EditMessage::new();

    let mut embeds = vec![CreateEmbed::new()
        // Setting this so that the embeds are joined together
        .url("https://openai.com")
        .field("Prompt", prompt.as_str(), false)
        .field("Requstor", format!("<@!{}>", ctx.author().id), false)];

    for (pos, image) in resp.data.iter().enumerate() {
        match image.as_ref() {
            Image::Url {
                url,
                revised_prompt: _,
            } => {
                embeds.push(CreateEmbed::new().url("https://openai.com").image(url));
            }
            Image::B64Json {
                b64_json,
                revised_prompt: _,
            } => {
                let filename = format!("image-{}.png", pos);
                let data = BASE64_STANDARD.decode(b64_json.as_bytes()).unwrap();
                builder = builder.new_attachment(CreateAttachment::bytes(data, &filename));
                embeds.push(
                    CreateEmbed::new()
                        .url("https://openai.com")
                        .attachment(filename),
                );
            }
        }
    }

    builder = builder.embeds(embeds);
    // Calling this via the message object as the reply does not send new attachments
    reply
        .into_message()
        .await
        .unwrap()
        .edit(ctx, builder)
        .await?;
    Ok(())
}

/// Show the current global system_context (DB override or config fallback)
#[poise::command(slash_command, rename = "prompt-show", default_member_permissions = "ADMINISTRATOR", guild_only)]
pub async fn prompt_show(ctx: Context<'_>) -> Result<(), Error> {
    let db = ctx.data().db;
    let sys_ctx = if let Some(db_ctx) = moonbot_db::get_global_system_context(db).await {
        db_ctx
    } else {
        ctx.data().config.openai.auto.system_context.clone()
    };
    if sys_ctx.is_empty() {
        ctx.send(poise::CreateReply::default().content("No global system_context set.").ephemeral(true)).await?;
    } else {
        let joined = sys_ctx
            .iter()
            .enumerate()
            .map(|(i, s)| format!("{}: {}", i + 1, s))
            .collect::<Vec<_>>()
            .join("\n");
        ctx.send(poise::CreateReply::default().content(format!("Current global system_context ({} lines):\n{}", sys_ctx.len(), joined)).ephemeral(true)).await?;
    }
    Ok(())
}

/// Replace the global system_context with one or more lines (separate by \n)
#[poise::command(slash_command, rename = "prompt-set", default_member_permissions = "ADMINISTRATOR", guild_only)]
pub async fn prompt_set(
    ctx: Context<'_>,
    #[description = "New system context; use new lines to separate multiple lines"] content: String,
) -> Result<(), Error> {
    let lines: Vec<String> = content.lines().map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();
    if let Err(e) = moonbot_db::set_global_system_context(ctx.data().db, lines).await {
    ctx.send(poise::CreateReply::default().content(format!("Failed to set global prompt: {}", e)).ephemeral(true)).await?;
    } else {
    context::invalidate_global_context().await;
    ctx.send(poise::CreateReply::default().content("Updated global system_context.").ephemeral(true)).await?;
    }
    Ok(())
}

/// Append a single line to the global system_context
#[poise::command(slash_command, rename = "prompt-add", default_member_permissions = "ADMINISTRATOR", guild_only)]
pub async fn prompt_add(
    ctx: Context<'_>,
    #[description = "Line to append to system_context"] line: String,
) -> Result<(), Error> {
    if let Err(e) = moonbot_db::add_global_system_context_line(ctx.data().db, line).await {
    ctx.send(poise::CreateReply::default().content(format!("Failed to add line: {}", e)).ephemeral(true)).await?;
    } else {
    context::invalidate_global_context().await;
    ctx.send(poise::CreateReply::default().content("Appended line to global system_context.").ephemeral(true)).await?;
    }
    Ok(())
}

/// Clear the global system_context (reverts to config fallback)
#[poise::command(slash_command, rename = "prompt-clear", default_member_permissions = "ADMINISTRATOR", guild_only)]
pub async fn prompt_clear(ctx: Context<'_>) -> Result<(), Error> {
    if let Err(e) = moonbot_db::clear_global_system_context(ctx.data().db).await {
    ctx.send(poise::CreateReply::default().content(format!("Failed to clear: {}", e)).ephemeral(true)).await?;
    } else {
    context::invalidate_global_context().await;
    ctx.send(poise::CreateReply::default().content("Cleared global system_context.").ephemeral(true)).await?;
    }
    Ok(())
}

/// Show basic bot status
#[poise::command(slash_command, rename = "status")]
pub async fn status(ctx: Context<'_>) -> Result<(), Error> {
    let cfg = &ctx.data().config;
    let openai_cfg = if cfg.openai.api_key.is_empty() && cfg.openai.api_base.is_empty() { "disabled" } else { "enabled" };
    let personalize = cfg.openai.auto.personalize;
    let db_url = &cfg.database.url;
    let db_ok = "ok"; // migrations already checked at startup
    let embed = CreateEmbed::new()
        .title("Moonbot Status")
        .field("OpenAI", openai_cfg, true)
        .field("Personalize", personalize.to_string(), true)
        .field("Model", cfg.openai.auto.model.as_str(), true)
        .field("DB", format!("{} ({})", db_ok, db_url), false)
        .field("Repo", "https://github.com/tomhuis/moonbot-rs", false);
    ctx.send(poise::CreateReply::default().embed(embed)).await?;
    Ok(())
}

/// Show or set bot mood
#[poise::command(slash_command, rename = "mood", default_member_permissions = "ADMINISTRATOR", guild_only)]
pub async fn mood(
    ctx: Context<'_>,
    #[description = "Action"] action: MoodAction,
    #[description = "Mood name (for set)"] mood: Option<String>,
    #[description = "Intensity -5..5 (for set)"] level: Option<i32>,
    #[description = "Notes (for set)"] notes: Option<String>,
) -> Result<(), Error> {
    match action {
        MoodAction::Show => {
            if let Some(d) = moonbot_db::get_bot_disposition(ctx.data().db).await {
                ctx.send(poise::CreateReply::default().content(format!("mood='{}' level={} notes={}", d.mood, d.mood_level, d.notes)).ephemeral(true)).await?;
            } else {
                ctx.send(poise::CreateReply::default().content("No mood set").ephemeral(true)).await?;
            }
        }
        MoodAction::Set => {
            let d = moonbot_db::Disposition { mood: mood.unwrap_or_else(|| "neutral".into()), mood_level: level.unwrap_or(0).clamp(-5,5), notes: notes.unwrap_or_default() };
            match moonbot_db::set_bot_disposition(ctx.data().db, d).await {
                Ok(()) => ctx.send(poise::CreateReply::default().content("Updated mood").ephemeral(true)).await?,
                Err(e) => ctx.send(poise::CreateReply::default().content(format!("Failed: {}", e)).ephemeral(true)).await?,
            };
            crate::context::invalidate_disposition().await;
        }
    }
    Ok(())
}

#[derive(Debug, poise::ChoiceParameter)]
pub enum MoodAction { Show, Set }

/// User profile privacy utilities
#[poise::command(slash_command, rename = "profile")]
pub async fn profile(
    ctx: Context<'_>,
    #[description = "Action"] action: ProfileAction,
    #[description = "User (omit for yourself)"] user: Option<User>,
) -> Result<(), Error> {
    let target_id = user.as_ref().map(|u| u.id).unwrap_or(ctx.author().id).get() as i64;
    match action {
        ProfileAction::Show => {
            if let Some(p) = moonbot_db::get_user_profile(ctx.data().db, target_id).await {
                ctx.send(poise::CreateReply::default().content(format!("traits={:?}\ntrust={}\nprefs={}\nsummary={}", p.traits, p.trust_level, p.preferences, p.summary)).ephemeral(true)).await?;
            } else {
                ctx.send(poise::CreateReply::default().content("No profile").ephemeral(true)).await?;
            }
        }
        ProfileAction::Clear => {
            match moonbot_db::delete_user_profile(ctx.data().db, target_id).await {
                Ok(n) => ctx.send(poise::CreateReply::default().content(format!("Deleted {} row(s)", n)).ephemeral(true)).await?,
                Err(e) => ctx.send(poise::CreateReply::default().content(format!("Failed: {}", e)).ephemeral(true)).await?,
            };
        }
    }
    Ok(())
}

#[derive(Debug, poise::ChoiceParameter)]
pub enum ProfileAction { Show, Clear }
