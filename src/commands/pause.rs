use crate::{
    actions::playback_actions,
    checks::{author_in_shared_voice_channel, author_in_voice_channel},
    models::{DiscordError, RuntimeError},
    server::Context,
};

/// Pause the current track.
#[poise::command(
    slash_command,
    check = "author_in_voice_channel",
    check = "author_in_shared_voice_channel"
)]
pub async fn pause(ctx: Context<'_>) -> Result<(), RuntimeError> {
    ctx.defer().await.map_err(DiscordError::Gateway)?;
    playback_actions::pause(&ctx).await
}
