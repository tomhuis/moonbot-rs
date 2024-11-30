use crate::{Data, Error};
use poise::builtins::on_error as poise_on_error;
use poise::serenity_prelude as serenity;
use poise::FrameworkError;
use sea_orm::*;
use sunbot_db::entities::prelude::*;

use tracing::info;

mod dad;
pub mod lavalink;
mod openai;

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
        serenity::FullEvent::GuildCreate { guild, .. } => {
            info!("Joined Guild {}: {}", guild.id, guild.name);
            let guild = sunbot_db::entities::guild::ActiveModel {
                id: ActiveValue::Set(guild.id.get() as i64),
                ..Default::default()
            };
            Guild::insert(guild)
                .on_conflict_do_nothing()
                .exec(framework.user_data.db)
                .await?;
        }
        _ => {}
    }

    Ok(())
}

pub async fn error_handler<U, E: std::fmt::Display + std::fmt::Debug>(
    error: FrameworkError<'_, U, E>,
) {
    if let Err(e) = poise_on_error(error).await {
        tracing::error!("Error while handling error: {}", e);
    }
}
