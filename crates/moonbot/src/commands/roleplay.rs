use crate::{Context, Error};

/// Manage the guild-wide roleplay persona
#[poise::command(slash_command, rename = "roleplay", default_member_permissions = "ADMINISTRATOR", guild_only)]
pub async fn command(
	ctx: Context<'_>,
	#[description = "Action"] action: RoleplayAction,
	#[description = "Persona text (for set)"] persona: Option<String>,
) -> Result<(), Error> {
	let Some(gid) = ctx.guild_id() else { return Ok(()); };
	match action {
		RoleplayAction::Show => {
			if let Some(p) = moonbot_db::get_guild_roleplay(ctx.data().db, gid.get() as i64).await {
				ctx.send(poise::CreateReply::default().content(format!("Persona:\n{}", p)).ephemeral(true)).await?;
			} else {
				ctx.send(poise::CreateReply::default().content("No guild persona set").ephemeral(true)).await?;
			}
		}
		RoleplayAction::Set => {
			let Some(txt) = persona else { ctx.send(poise::CreateReply::default().content("Missing persona").ephemeral(true)).await?; return Ok(()); };
			match moonbot_db::set_guild_roleplay(ctx.data().db, gid.get() as i64, txt).await {
				Ok(()) => ctx.send(poise::CreateReply::default().content("Updated guild persona").ephemeral(true)).await?,
				Err(e) => ctx.send(poise::CreateReply::default().content(format!("Failed: {}", e)).ephemeral(true)).await?,
			};
		}
		RoleplayAction::Clear => {
			match moonbot_db::clear_guild_roleplay(ctx.data().db, gid.get() as i64).await {
				Ok(_) => ctx.send(poise::CreateReply::default().content("Cleared guild persona").ephemeral(true)).await?,
				Err(e) => ctx.send(poise::CreateReply::default().content(format!("Failed: {}", e)).ephemeral(true)).await?,
			};
		}
	}
	Ok(())
}

#[derive(Debug, poise::ChoiceParameter)]
pub enum RoleplayAction { Show, Set, Clear }

