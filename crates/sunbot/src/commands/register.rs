use crate::{Context, Error};


/// Register application commands in this guild or globally
#[poise::command(prefix_command, hide_in_help)]
pub async fn register_commands(
    ctx: Context<'_>
) -> Result<(), Error> {
    poise::builtins::register_application_commands_buttons(ctx).await?;
    Ok(())
}
