use tracing::instrument;

use crate::{
    actions::playback_actions,
    checks::author_in_shared_voice_channel,
    server::{Context, ServerError},
};

/// Stop playback and clear the queue.
#[instrument(skip(ctx))]
#[poise::command(slash_command, check = "author_in_shared_voice_channel")]
pub async fn stop(ctx: Context<'_>) -> Result<(), ServerError> {
    playback_actions::stop(&ctx).await
}
