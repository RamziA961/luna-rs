use crate::{
    actions::playback_actions,
    checks::{author_in_shared_voice_channel, author_in_voice_channel},
    server::{Context, ServerError},
};

// Resume a paused track.
#[poise::command(
    slash_command,
    check = "author_in_voice_channel",
    check = "author_in_shared_voice_channel"
)]
pub async fn resume(ctx: Context<'_>) -> Result<(), ServerError> {
    _ = ctx.defer().await;
    playback_actions::resume(&ctx).await
}
