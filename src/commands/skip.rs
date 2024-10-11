use crate::{
    actions::playback_actions,
    checks::{author_in_shared_voice_channel, author_in_voice_channel},
    server::{Context, ServerError},
};

/// Skip a number of tracks.
#[poise::command(
    slash_command,
    check = "author_in_voice_channel",
    check = "author_in_shared_voice_channel"
)]
pub async fn skip(
    ctx: Context<'_>,

    #[description = "Number of tracks to skip"]
    #[min = 1_usize]
    n: Option<usize>,
) -> Result<(), ServerError> {
    ctx.defer().await?;
    playback_actions::skip(&ctx, n.unwrap_or(1)).await
}
