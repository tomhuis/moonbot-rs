use crate::{Context, Error};
use async_openai::types::{
    ChatCompletionRequestSystemMessageArgs, CreateChatCompletionRequestArgs,
    CreateImageRequestArgs, Image, ImageModel, ImageSize,
};
use serenity::builder::CreateEmbed;

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
) -> Result<(), Error> {
    let client = ctx.data().openai_client.as_ref().unwrap();

    let embed = CreateEmbed::new()
        .title("Please wait generating images")
        .field("Prompt", prompt.as_str(), false)
        .field("Requstor", ctx.author().tag(), false);

    let reply = ctx.send(poise::CreateReply::default().embed(embed)).await?;

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

    let request = CreateImageRequestArgs::default()
        .model(model)
        .prompt(prompt.as_str())
        .size(image_size)
        .build()
        .unwrap();

    let resp = client.images().create(request).await.unwrap();

    let mut builder = poise::CreateReply::default();

    let embeds = resp
        .data
        .iter()
        .map(|image| match image.as_ref() {
            Image::Url {
                url,
                revised_prompt: _,
            } => CreateEmbed::new().image(url),
            _ => CreateEmbed::new().description("Unknown image type"),
        })
        .collect();

    builder.embeds = embeds;
    reply.edit(ctx, builder).await?;
    Ok(())
}
