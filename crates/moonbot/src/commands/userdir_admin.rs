use crate::{Context, Error};

/// Upsert a user directory entry
#[poise::command(slash_command, rename = "userdir-admin", default_member_permissions = "ADMINISTRATOR", guild_only)]
pub async fn command(
	ctx: Context<'_>,
	#[description = "Action"] action: Action,
	#[description = "User id"] user_id: Option<String>,
	#[description = "Display name"] display_name: Option<String>,
	#[description = "Aliases (comma-separated)"] aliases: Option<String>,
	#[description = "Notes"] notes: Option<String>,
) -> Result<(), Error> {
	match action {
		Action::Upsert => {
			let uid: i64 = user_id.and_then(|s| s.parse().ok()).unwrap_or(ctx.author().id.get() as i64);
			let entry = moonbot_db::UserDirectoryEntry {
				user_id: uid,
				display_name: display_name.unwrap_or_else(|| ctx.author().name.clone()),
				aliases: aliases.map(|s| s.split(',').map(|t| t.trim().to_string()).filter(|t| !t.is_empty()).collect()).unwrap_or_default(),
				notes: notes.unwrap_or_default(),
			};
			match moonbot_db::upsert_user_directory(ctx.data().db, entry).await {
				Ok(()) => ctx.send(poise::CreateReply::default().content("Upserted").ephemeral(true)).await?,
				Err(e) => ctx.send(poise::CreateReply::default().content(format!("Failed: {}", e)).ephemeral(true)).await?,
			};
		}
		Action::Delete => {
			if let Some(uid) = user_id.and_then(|s| s.parse::<i64>().ok()) {
				match moonbot_db::delete_user_directory(ctx.data().db, uid).await {
					Ok(n) => ctx.send(poise::CreateReply::default().content(format!("Deleted {}", n)).ephemeral(true)).await?,
					Err(e) => ctx.send(poise::CreateReply::default().content(format!("Failed: {}", e)).ephemeral(true)).await?,
				};
			} else {
				ctx.send(poise::CreateReply::default().content("Missing user_id").ephemeral(true)).await?;
			}
		}
	}
	Ok(())
}

#[derive(Debug, poise::ChoiceParameter)]
pub enum Action { Upsert, Delete }

