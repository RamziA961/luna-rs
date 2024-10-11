use crate::{
    actions::playback_actions,
    checks::{author_in_shared_voice_channel, author_in_voice_channel},
    server::{Context, ServerError},
};

/// Display the next items in the queue.
#[poise::command(
    slash_command,
    check = "author_in_voice_channel",
    check = "author_in_shared_voice_channel"
)]
pub async fn queue(ctx: Context<'_>) -> Result<(), ServerError> {
    ctx.defer().await?;
    playback_actions::show_queue(&ctx).await?;
    Ok(())
}
