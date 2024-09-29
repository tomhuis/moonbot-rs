use crate::{Data, Error};
use poise::serenity_prelude as serenity;

mod openai;
mod dad;
pub mod lavalink;

pub async fn handler(
    ctx: &serenity::Context,
    event: &serenity::FullEvent,
    framework: poise::FrameworkContext<'_, Data, Error>,
    _data: &Data,
) -> Result<(), Error> {
    match event {
        serenity::FullEvent::Message { new_message } => {
            dad::handle_message(ctx, framework, new_message).await?;
            openai::handle_random_message(ctx, framework, new_message).await?;
            openai::handle_reply(ctx, framework, new_message).await?;
        }
        _ => {}
    }

    Ok(())
}
