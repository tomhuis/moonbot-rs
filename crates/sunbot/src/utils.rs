use crate::Context;
use poise::serenity_prelude as serenity;
use tracing::info;

// Check if a message is a reply or a mention for a specific user
pub async fn is_reply_or_mention(
    ctx: &serenity::Context,
    message: &serenity::Message,
    user_id: serenity::UserId,
) -> bool {
    if let Some(ref reply) = message.message_reference {
        if let Some(msg_id) = reply.message_id {
            let msg = ctx
                .http
                .get_message(reply.channel_id, msg_id)
                .await
                .unwrap();

            if msg.author.id == user_id {
                info!("Reply detected: {}", msg.content);
                return true;
            }
        }
    }

    if message.mentions_user_id(user_id) {
        info!("Mention detected: {}", message.content);
        return true;
    }

    false
}

/// Reply with an error message
pub async fn send_err_msg(ctx: Context<'_>, title: &str, description: &str) {
    let embed = serenity::CreateEmbed::default()
        .title(title)
        .color(0xFF0000)
        .description(description);
    let _ = ctx
        .send(poise::CreateReply::default().embed(embed.clone()))
        .await;
}
