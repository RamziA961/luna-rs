use tracing::trace;

use crate::server::{Context, ServerError};
pub async fn author_in_voice_channel(ctx: Context<'_>) -> Result<bool, ServerError> {
    let author_id = ctx.author().id;

    // Scope block to extract data and drop the non-Send guild object
    let is_in_vc = {
        let guild = ctx
            .guild()
            .ok_or_else(|| ServerError::Internal("Could not find guild information".to_string()))?;
        guild.voice_states.contains_key(&author_id)
    };

    if !is_in_vc {
        trace!("Command dispatched by author not in a voice channel.");
        ctx.reply("Please join a voice channel to initiate this command.")
            .await?;
        return Ok(false);
    }

    Ok(true)
}

pub async fn author_in_shared_voice_channel(ctx: Context<'_>) -> Result<bool, ServerError> {
    let author_id = ctx.author().id;
    let bot_id = ctx.framework().bot_id;

    let (author_vc, bot_vc) = {
        let guild = ctx
            .guild()
            .ok_or_else(|| ServerError::Internal("Could not find guild information".to_string()))?;

        (
            guild
                .voice_states
                .get(&author_id)
                .and_then(|vs| vs.channel_id),
            guild.voice_states.get(&bot_id).and_then(|vs| vs.channel_id),
        )
    };

    match (author_vc, bot_vc) {
        (Some(a), Some(b)) if a == b => Ok(true),
        (Some(_), None) => Ok(true),
        (Some(_), Some(_)) | (None, Some(_)) => {
            ctx.reply("Please join a shared voice channel to issue this command.")
                .await?;
            Ok(false)
        }
        (None, None) => {
            ctx.reply("Please join a voice channel to initiate this command.")
                .await?;
            Ok(false)
        }
    }
}
