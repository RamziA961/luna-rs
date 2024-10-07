use crate::{
    actions::playback_actions,
    checks::{author_in_shared_voice_channel, author_in_voice_channel},
    server::{Context, ServerError},
};

/// Pause the current track.
#[poise::command(
    slash_command,
    check = "author_in_voice_channel",
    check = "author_in_shared_voice_channel"
)]
pub async fn pause(ctx: Context<'_>) -> Result<(), ServerError> {
    playback_actions::pause(&ctx).await
}
