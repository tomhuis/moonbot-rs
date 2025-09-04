use std::vec;

use crate::{Context, Error};
use async_openai::types::{
    ChatCompletionRequestSystemMessageArgs, CreateChatCompletionRequestArgs,
    CreateImageRequestArgs, Image, ImageModel, ImageResponseFormat, ImageSize,
};
use base64::prelude::*;
use serenity::{
    all::{CreateAttachment, EditMessage},
    builder::CreateEmbed,
};

/// Ask a question to OpenAI
#[poise::command(slash_command)]
pub async fn askgpt(
    ctx: Context<'_>,
    #[description = "The prompt to send to OpenAI"] prompt: String,
) -> Result<(), Error> {
    let client = ctx.data().openai_client.as_ref().unwrap();

    let request = CreateChatCompletionRequestArgs::default()
        .model(ctx.data().config.openai.askgpt.model.as_str())
        .messages([ChatCompletionRequestSystemMessageArgs::default()
            .content(prompt)
            .build()?
            .into()])
        .max_tokens(ctx.data().config.openai.askgpt.max_tokens)
        .build()?;

    let resp = client.chat().create(request).await?;
    ctx.reply(
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
#[poise::command(slash_command)]
pub async fn genimage(
    ctx: Context<'_>,
    #[description = "The prompt to send to OpenAI"] prompt: String,
    #[description = "The size of the image to generate"] size: Option<ImageSizeType>,
    #[description = "The number of images to generate"] amount: Option<u8>,
) -> Result<(), Error> {
    let client = ctx.data().openai_client.as_ref().unwrap();

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

    let request = CreateImageRequestArgs::default()
        .model(model)
        .n(n)
        .prompt(prompt.as_str())
        .size(image_size)
        .response_format(ImageResponseFormat::B64Json)
        .build()
        .unwrap();

    let resp = client.images().create(request).await.unwrap();

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
#[poise::command(slash_command)]
pub async fn prompt_show(ctx: Context<'_>) -> Result<(), Error> {
    let db = ctx.data().db;
    let sys_ctx = if let Some(db_ctx) = moonbot_db::get_global_system_context(db).await {
        db_ctx
    } else {
        ctx.data().config.openai.auto.system_context.clone()
    };
    if sys_ctx.is_empty() {
        ctx.say("No global system_context set.").await?;
    } else {
        let joined = sys_ctx
            .iter()
            .enumerate()
            .map(|(i, s)| format!("{}: {}", i + 1, s))
            .collect::<Vec<_>>()
            .join("\n");
        ctx.say(format!("Current global system_context ({} lines):\n{}", sys_ctx.len(), joined))
            .await?;
    }
    Ok(())
}

/// Replace the global system_context with one or more lines (separate by \n)
#[poise::command(slash_command)]
pub async fn prompt_set(
    ctx: Context<'_>,
    #[description = "New system context; use new lines to separate multiple lines"] content: String,
) -> Result<(), Error> {
    let lines: Vec<String> = content.lines().map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();
    if let Err(e) = moonbot_db::set_global_system_context(ctx.data().db, lines).await {
        ctx.say(format!("Failed to set global prompt: {}", e)).await?;
    } else {
        ctx.say("Updated global system_context.").await?;
    }
    Ok(())
}

/// Append a single line to the global system_context
#[poise::command(slash_command)]
pub async fn prompt_add(
    ctx: Context<'_>,
    #[description = "Line to append to system_context"] line: String,
) -> Result<(), Error> {
    if let Err(e) = moonbot_db::add_global_system_context_line(ctx.data().db, line).await {
        ctx.say(format!("Failed to add line: {}", e)).await?;
    } else {
        ctx.say("Appended line to global system_context.").await?;
    }
    Ok(())
}

/// Clear the global system_context (reverts to config fallback)
#[poise::command(slash_command)]
pub async fn prompt_clear(ctx: Context<'_>) -> Result<(), Error> {
    if let Err(e) = moonbot_db::clear_global_system_context(ctx.data().db).await {
        ctx.say(format!("Failed to clear: {}", e)).await?;
    } else {
        ctx.say("Cleared global system_context.").await?;
    }
    Ok(())
}
