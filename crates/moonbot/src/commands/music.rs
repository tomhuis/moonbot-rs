use crate::{utils::send_err_msg, Context, Error};
use futures::{future, StreamExt};
use humantime::format_duration;
use lavalink_rs::prelude::*;
use poise::serenity_prelude as serenity;
use std::ops::Deref;
use std::time::Duration;

async fn _join(
    ctx: &Context<'_>,
    guild_id: serenity::GuildId,
    channel_id: Option<serenity::ChannelId>,
) -> Result<bool, Error> {
    let lava_client = match &ctx.data().lavalink {
        Some(x) => x,
        None => {
            send_err_msg(*ctx, "Error", "Lavalink client is not available.").await;
            return Ok(false);
        }
    };

    let manager = songbird::get(ctx.serenity_context()).await.unwrap().clone();

    if lava_client.get_player_context(guild_id).is_none() {
        let connect_to = match channel_id {
            Some(x) => x,
            None => {
                let guild = ctx.guild().unwrap().deref().clone();
                let user_channel_id = guild
                    .voice_states
                    .get(&ctx.author().id)
                    .and_then(|voice_state| voice_state.channel_id);

                match user_channel_id {
                    Some(channel) => channel,
                    None => {
                        send_err_msg(
                            *ctx,
                            "Error",
                            "You are not in a voice channel, please join one first.",
                        )
                        .await;
                        return Ok(false);
                    }
                }
            }
        };

        let handler = manager.join_gateway(guild_id, connect_to).await;

        match handler {
            Ok((connection_info, _)) => {
                lava_client
                    // The turbofish here is Optional, but it helps to figure out what type to
                    // provide in `PlayerContext::data()`
                    //
                    // While a tuple is used here as an example, you are free to use a custom
                    // public structure with whatever data you wish.
                    // This custom data is also present in the Client if you wish to have the
                    // shared data be more global, rather than centralized to each player.
                    .create_player_context_with_data::<(serenity::ChannelId, std::sync::Arc<serenity::Http>)>(
                        guild_id,
                        connection_info,
                        std::sync::Arc::new((
                            ctx.channel_id(),
                            ctx.serenity_context().http.clone(),
                        )),
                    )
                    .await?;
                return Ok(true);
            }
            Err(why) => {
                send_err_msg(
                    *ctx,
                    "Error",
                    format!("Error joining the channel: {}", why).as_str(),
                )
                .await;
                return Err(why.into());
            }
        }
    }

    Ok(false)
}

/// Play a song in the voice channel you are connected in.
#[poise::command(slash_command, rename = "music-play")]
pub async fn play(
    ctx: Context<'_>,
    #[description = "Search term or URL"]
    #[rest]
    term: String,
) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap();

    let lava_client = match &ctx.data().lavalink {
        Some(x) => x,
        None => {
            send_err_msg(ctx, "Error", "Lavalink client is not available.").await;
            return Ok(());
        }
    };

    _join(&ctx, guild_id, None).await?;
    let Some(player) = lava_client.get_player_context(guild_id) else {
        return Ok(());
    };

    let query = if term.starts_with("http") {
        term
    } else {
        SearchEngines::YouTube.to_query(&term)?
    };

    let loaded_tracks = lava_client.load_tracks(guild_id, &query).await?;
    let mut playlist_info = None;
    let mut tracks: Vec<TrackInQueue> = match loaded_tracks.data {
        Some(TrackLoadData::Track(x)) => vec![x.into()],
        Some(TrackLoadData::Search(x)) => vec![x[0].clone().into()],
        Some(TrackLoadData::Playlist(x)) => {
            playlist_info = Some(x.info);
            x.tracks.iter().map(|x| x.clone().into()).collect()
        }
        Some(TrackLoadData::Error(x)) => {
            send_err_msg(ctx, "Error", x.message.as_str()).await;
            return Ok(());
        }
        _ => {
            ctx.say(format!("{:?}", loaded_tracks)).await?;
            return Ok(());
        }
    };

    let queue = player.get_queue();
    let mut duration = 0;
    let position = queue.get_count().await.unwrap_or(0) + 1;

    for i in &mut tracks {
        i.track.user_data = Some(serde_json::json!({"requester_id": ctx.author().id.get()}));
        duration += i.track.info.length;
    }

    let track = tracks.remove(0).track;
    // If there is no track playing, just play the first track
    if player.get_player().await.unwrap().track.is_none() {
        player.play(&track).await?;
    }

    // Add the rest of the tracks to the queue
    queue.append(tracks.clone().into())?;

    // If the queue is empty, reply with a hidden message to the user
    if queue.get_count().await.unwrap_or(0) == 0 {
        let resp = ctx
            .send(
                poise::CreateReply::default()
                    .content("Added to queue")
                    .ephemeral(true),
            )
            .await?;
        resp.delete(ctx).await?;
        return Ok(());
    }

    let mut embed = serenity::CreateEmbed::default().color(0x2ECC71);

    embed = if let Some(info) = playlist_info {
        embed
            .author(
                serenity::CreateEmbedAuthor::new("Playlist added to queue")
                    .icon_url(ctx.author().avatar_url().unwrap_or_default()),
            )
            .description(format!("Added playlist {}", info.name))
            .field("Tracks", tracks.len().to_string(), false)
            .field(
                "Position",
                format!("#{}-{}", position, 1 + tracks.len()),
                true,
            )
            .field(
                "Duration",
                format_duration(Duration::from_millis(duration)).to_string(),
                true,
            )
    } else {
        embed
            .author(
                serenity::CreateEmbedAuthor::new("Added to queue")
                    .icon_url(ctx.author().avatar_url().unwrap_or_default()),
            )
            .image(track.info.artwork_url.as_ref().unwrap_or(&String::new()))
            .description(format!(
                "[{}](<{}>)",
                track.info.title,
                track.info.uri.as_ref().unwrap_or(&String::new())
            ))
            .field("Position", format!("#{}", position), true)
            .field(
                "Duration",
                format_duration(Duration::from_millis(duration)).to_string(),
                true,
            )
    };

    ctx.send(poise::CreateReply::default().embed(embed)).await?;

    Ok(())
}

/// Join the specified voice channel or the one you are currently in.
#[poise::command(slash_command, rename = "music-join")]
pub async fn join(
    ctx: Context<'_>,
    #[description = "The channel ID to join to."]
    #[channel_types("Voice")]
    channel_id: Option<serenity::ChannelId>,
) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap();
    _join(&ctx, guild_id, channel_id).await?;

    ctx.send(
        poise::CreateReply::default()
            .content("Joined the voice channel.")
            .ephemeral(true),
    )
    .await?;

    Ok(())
}

/// Stop Playing music and Leave the current voice channel.
#[poise::command(slash_command, rename = "music-leave")]
pub async fn leave(ctx: Context<'_>) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap();
    let manager = songbird::get(ctx.serenity_context()).await.unwrap().clone();

    let lava_client = match &ctx.data().lavalink {
        Some(x) => x,
        None => {
            send_err_msg(ctx, "Error", "Lavalink client is not available.").await;
            return Ok(());
        }
    };

    if lava_client.get_player_context(guild_id).is_none() {
        send_err_msg(ctx, "Error", "Im not playing anything! :rage:").await;
        return Ok(());
    }

    let _ = lava_client.delete_player(guild_id).await;

    if manager.get(guild_id).is_some() {
        manager.remove(guild_id).await?;
    }

    let embed = serenity::CreateEmbed::new()
        .author(
            serenity::CreateEmbedAuthor::new("Stopped playing music")
                .icon_url(ctx.author().avatar_url().unwrap_or_default()),
        )
        .color(0x2ECC71);

    ctx.send(poise::CreateReply::default().embed(embed)).await?;
    Ok(())
}

/// Pauses playing music
#[poise::command(slash_command, rename = "music-pause")]
pub async fn pause(ctx: Context<'_>) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap();

    let lava_client = match &ctx.data().lavalink {
        Some(x) => x,
        None => {
            send_err_msg(ctx, "Error", "Lavalink client is not available.").await;
            return Ok(());
        }
    };

    let Some(player) = lava_client.get_player_context(guild_id) else {
        send_err_msg(ctx, "Error", "Join the bot to a voice channel first.").await;
        return Ok(());
    };
    player.set_pause(true).await?;

    let embed = serenity::CreateEmbed::new()
        .author(
            serenity::CreateEmbedAuthor::new("Paused Music")
                .icon_url(ctx.author().avatar_url().unwrap_or_default()),
        )
        .color(0x2ECC71);
    ctx.send(poise::CreateReply::default().embed(embed)).await?;

    Ok(())
}

/// Resumes playing music
#[poise::command(slash_command, rename = "music-resume")]
pub async fn resume(ctx: Context<'_>) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap();

    let lava_client = match &ctx.data().lavalink {
        Some(x) => x,
        None => {
            send_err_msg(ctx, "Error", "Lavalink client is not available.").await;
            return Ok(());
        }
    };

    let Some(player) = lava_client.get_player_context(guild_id) else {
        send_err_msg(ctx, "Error", "Join the bot to a voice channel first.").await;
        return Ok(());
    };

    player.set_pause(false).await?;

    let embed = serenity::CreateEmbed::new()
        .author(
            serenity::CreateEmbedAuthor::new("Resumed Music")
                .icon_url(ctx.author().avatar_url().unwrap_or_default()),
        )
        .color(0x2ECC71);
    ctx.send(poise::CreateReply::default().embed(embed)).await?;

    Ok(())
}

/// Skip the current song
#[poise::command(slash_command, rename = "music-skip")]
pub async fn skip(ctx: Context<'_>) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap();

    let lava_client = match &ctx.data().lavalink {
        Some(x) => x,
        None => {
            send_err_msg(ctx, "Error", "Lavalink client is not available.").await;
            return Ok(());
        }
    };

    let Some(player) = lava_client.get_player_context(guild_id) else {
        send_err_msg(ctx, "Error", "Join the bot to a voice channel first.").await;
        return Ok(());
    };

    player.skip()?;

    // If queue is empty and nothing is playing, send a different message
    if player.get_queue().get_count().await? == 0
        && player.get_player().await.unwrap().track.is_none()
    {
        let embed = serenity::CreateEmbed::new()
            .author(
                serenity::CreateEmbedAuthor::new("Skipped")
                    .icon_url(ctx.author().avatar_url().unwrap_or_default()),
            )
            .color(0x2ECC71)
            .description("Queue is empty");
        ctx.send(poise::CreateReply::default().embed(embed)).await?;
    } else {
        let embed = serenity::CreateEmbed::new()
            .author(
                serenity::CreateEmbedAuthor::new("Skipped")
                    .icon_url(ctx.author().avatar_url().unwrap_or_default()),
            )
            .color(0x2ECC71)
            .description("Skipped the current song");
        ctx.send(poise::CreateReply::default().embed(embed)).await?;
    }

    Ok(())
}

/// Displays the current queue
#[poise::command(slash_command, rename = "music-queue")]
pub async fn queue(ctx: Context<'_>) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap();

    let lava_client = match &ctx.data().lavalink {
        Some(x) => x,
        None => {
            send_err_msg(ctx, "Error", "Lavalink client is not available.").await;
            return Ok(());
        }
    };

    let Some(player) = lava_client.get_player_context(guild_id) else {
        send_err_msg(ctx, "Error", "Join the bot to a voice channel first.").await;
        return Ok(());
    };

    let queue = player.get_queue();
    let queue_count = queue.get_count().await?;
    let player_data = player.get_player().await?;
    let max = queue_count.min(5);
    let mut queue_message = queue
        .enumerate()
        .take_while(|(idx, _)| future::ready(*idx < max))
        .map(|(idx, x)| {
            format!(
                "**{} - **[{} - {}](<{}>)\n*Requested By <@!{}>* | {}\n",
                idx + 1,
                x.track.info.author,
                x.track.info.title,
                x.track.info.uri.as_ref().unwrap_or(&String::new()),
                x.track.user_data.unwrap()["requester_id"],
                format_duration(Duration::from_millis(x.track.info.length)),
            )
        })
        .collect::<Vec<_>>()
        .await
        .join("\n");

    if queue_count > max {
        queue_message.push_str(&format!("\n\nAnd {} more...", queue_count - max));
    }

    let song_position = player.get_player().await.unwrap().state.position;
    let now_playing_message = if let Some(track) = player_data.track {
        format!(
            "[{} - {}](<{}>)\n*Requested by <@!{}>*\n{} Left\n",
            track.info.author,
            track.info.title,
            track.info.uri.as_ref().unwrap_or(&String::new()),
            track.user_data.unwrap()["requester_id"],
            format_duration(Duration::from_millis(
                (track.info.length - song_position) / 1000 * 1000
            ))
        )
    } else {
        "No song is currently playing".to_string()
    };

    let embed = serenity::CreateEmbed::new()
        .title("Queue")
        .color(0x2ECC71)
        .field("Now Playing", now_playing_message, false)
        .field("Queue", queue_message, false);

    ctx.send(poise::CreateReply::default().embed(embed)).await?;

    Ok(())
}
