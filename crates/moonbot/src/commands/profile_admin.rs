use crate::{Context, Error};

/// Set or clear parts of a user's profile
#[poise::command(slash_command, rename = "profile-admin", default_member_permissions = "ADMINISTRATOR", guild_only)]
pub async fn command(
	ctx: Context<'_>,
	#[description = "Action"] action: Action,
	#[description = "User id (default: yourself)"] user_id: Option<String>,
	#[description = "Traits (comma-separated)"] traits: Option<String>,
	#[description = "Summary"] summary: Option<String>,
	#[description = "Trust level -5..5"] trust: Option<i32>,
) -> Result<(), Error> {
	let uid: i64 = user_id.and_then(|s| s.parse().ok()).unwrap_or(ctx.author().id.get() as i64);
	match action {
		Action::Set => {
			let mut profile = moonbot_db::get_user_profile(ctx.data().db, uid).await.unwrap_or_default();
			if let Some(t) = traits { profile.traits = t.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect(); }
			if let Some(s) = summary { profile.summary = s; }
			if let Some(level) = trust { profile.trust_level = level.clamp(-5,5); }
			match moonbot_db::upsert_user_profile(ctx.data().db, uid, profile).await {
				Ok(()) => ctx.send(poise::CreateReply::default().content("Updated").ephemeral(true)).await?,
				Err(e) => ctx.send(poise::CreateReply::default().content(format!("Failed: {}", e)).ephemeral(true)).await?,
			};
		}
		Action::Clear => {
			match moonbot_db::delete_user_profile(ctx.data().db, uid).await {
				Ok(n) => ctx.send(poise::CreateReply::default().content(format!("Deleted {}", n)).ephemeral(true)).await?,
				Err(e) => ctx.send(poise::CreateReply::default().content(format!("Failed: {}", e)).ephemeral(true)).await?,
			};
		}
	}
	Ok(())
}

#[derive(Debug, poise::ChoiceParameter)]
pub enum Action { Set, Clear }

