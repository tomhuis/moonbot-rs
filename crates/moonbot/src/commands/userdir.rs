use crate::{Context, Error};

/// Search users and return mention tags
#[poise::command(slash_command, rename = "userdir", guild_only)]
pub async fn command(
	ctx: Context<'_>,
	#[description = "Query to find users by display name, aliases, or notes"] query: String,
	#[description = "Maximum results (1-10)"] limit: Option<u8>,
) -> Result<(), Error> {
	let k = limit.unwrap_or(5).clamp(1, 10) as i64;
	let list = match moonbot_db::search_users_fts(ctx.data().db, &query, k).await {
		Ok(v) => v,
		Err(_) => moonbot_db::search_users_like(ctx.data().db, &query, k).await.unwrap_or_default(),
	};
	if list.is_empty() {
		ctx.say("No matches.").await?;
	} else {
		let tags = list.into_iter().map(|e| format!("<@{}>", e.user_id)).collect::<Vec<_>>().join(" ");
		ctx.say(tags).await?;
	}
	Ok(())
}

