use tracing::instrument;

use crate::{
    actions::playback_actions,
    checks::{author_in_shared_voice_channel, author_in_voice_channel, track_is_playing},
    models::{DiscordError, RuntimeError},
    server::Context,
};

/// Toggle radio mode. Automatically play similar tracks when the queue is empty.
#[instrument(skip(ctx))]
#[poise::command(
    slash_command,
    check = "author_in_voice_channel",
    check = "author_in_shared_voice_channel",
    check = "track_is_playing"
)]
pub async fn radio(ctx: Context<'_>) -> Result<(), RuntimeError> {
    ctx.defer().await.map_err(DiscordError::Gateway)?;
    playback_actions::toggle_radio_mode(&ctx).await?;
    Ok(())
}

