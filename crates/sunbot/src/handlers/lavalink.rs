use lavalink_rs::{hook, model::events, prelude::*};
use std::time::Duration;
use poise::serenity_prelude as serenity;
use humantime::format_duration;
use tracing::debug;

#[hook]
pub async fn raw_event(_: LavalinkClient, session_id: String, event: &serde_json::Value) {
    if event["op"].as_str() == Some("event") || event["op"].as_str() == Some("playerUpdate") {
        debug!("{:?} -> {:?}", session_id, event);
    }
}

#[hook]
pub async fn ready_event(client: LavalinkClient, session_id: String, event: &events::Ready) {
    client.delete_all_player_contexts().await.unwrap();
    debug!("{:?} -> {:?}", session_id, event);
}

#[hook]
pub async fn track_start(client: LavalinkClient, _session_id: String, event: &events::TrackStart) {
    let player_context = client.get_player_context(event.guild_id).unwrap();
    let data = player_context
        .data::<(serenity::ChannelId, std::sync::Arc<serenity::Http>)>()
        .unwrap();
    let (channel_id, http) = (&data.0, &data.1);

    // If the queue is empty, we don't want to send a message
    if player_context.get_queue().get_count().await.unwrap_or(0) == 0 {
        return;
    }

    let track = &event.track;

    let requester_id = track
            .user_data
            .as_ref()
            .and_then(|data| data.get("requester_id"))
            .unwrap_or(&serde_json::Value::Null);

    let embed = serenity::CreateEmbed::default()
        .color(0x2ECC71)
        .author(serenity::CreateEmbedAuthor::new("Now Playing"))
        .description(format!(
            "[{}](<{}>)",
            track.info.title, track.info.uri.as_ref().unwrap_or(&String::new())
        ))
        .field("Requested By", format!("<@!{}>", requester_id), false)
        .field("Author", track.info.author.to_string(), false)
        .field("Duration", format_duration(Duration::from_millis(track.info.length)).to_string(), false)
        .image(track.info.artwork_url.as_ref().unwrap_or(&String::new()));

    let _ = channel_id
        .send_message(http, serenity::CreateMessage::new().embed(embed))
        .await;
}
