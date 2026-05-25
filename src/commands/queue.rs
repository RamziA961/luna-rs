use crate::{
    actions::playback_actions,
    checks::{author_in_shared_voice_channel, author_in_voice_channel},
    models::{DiscordError, RuntimeError},
    server::Context,
};

/// Display the next items in the queue.
#[poise::command(
    slash_command,
    check = "author_in_voice_channel",
    check = "author_in_shared_voice_channel"
)]
pub async fn queue(ctx: Context<'_>) -> Result<(), RuntimeError> {
    ctx.defer().await.map_err(DiscordError::Gateway)?;
    playback_actions::show_queue(&ctx).await?;
    Ok(())
}
