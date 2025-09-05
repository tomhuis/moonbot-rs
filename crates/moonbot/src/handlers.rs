use crate::{Data, Error};
use poise::builtins::on_error as poise_on_error;
use poise::serenity_prelude as serenity;
use poise::FrameworkError;
use sea_orm::*;
use moonbot_db::entities::prelude::*;

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
            // Always analyze to update personalization state
            openai::handle_analysis_only(ctx, framework, new_message).await?;
            openai::handle_random_message(ctx, framework, new_message).await?;
            openai::handle_reply(ctx, framework, new_message).await?;
        }
        serenity::FullEvent::ReactionAdd { add_reaction } => {
            // Adjust trust when users react to the bot's messages
            if let Some(user_id) = add_reaction.user_id {
                if user_id == framework.bot_id { return Ok(()); }
                // Fetch the message to verify it's authored by the bot
                if let Ok(msg) = ctx.http.get_message(add_reaction.channel_id, add_reaction.message_id).await {
                    if msg.author.id == framework.bot_id {
                        // Log engagement into corpus (best-effort)
                        let kind = match &add_reaction.emoji { serenity::ReactionType::Unicode(s) => format!("reaction:{}", s), _ => "reaction".into() };
                        let _ = moonbot_db::upsert_corpus_entry(
                            framework.user_data.db,
                            msg.guild_id.map(|g| g.get() as i64),
                            Some(add_reaction.channel_id.get() as i64),
                            add_reaction.user_id.map(|u| u.get() as i64),
                            &kind,
                            &format!("reacted to message {}", add_reaction.message_id.get()),
                        ).await;
                        // Determine trust delta by emoji
                        let delta = match &add_reaction.emoji {
                            serenity::ReactionType::Unicode(s) => {
                                match s.as_str() {
                                    "👍" | "😀" | "❤️" | "🥰" | "😄" | "😉" | "🎉" | "✅" | "👏" => 1,
                                    "👎" | "😠" | "😡" | "❌" | "💩" => -1,
                                    _ => 0,
                                }
                            }
                            _ => 0,
                        };
                        if delta != 0 {
                            let uid = user_id.get() as i64;
                            if let Some(mut profile) = moonbot_db::get_user_profile(framework.user_data.db, uid).await {
                                profile.trust_level = (profile.trust_level + delta).clamp(-5, 5);
                                let _ = moonbot_db::upsert_user_profile(framework.user_data.db, uid, profile).await;
                            } else {
                                let profile = moonbot_db::UserProfile { trust_level: delta.clamp(-5,5), ..Default::default() };
                                let _ = moonbot_db::upsert_user_profile(framework.user_data.db, uid, profile).await;
                            }
                        }
                    }
                }
            }
        }
        serenity::FullEvent::GuildCreate { guild, .. } => {
            info!("Joined Guild {}: {}", guild.id, guild.name);
            let guild = moonbot_db::entities::guild::ActiveModel {
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
