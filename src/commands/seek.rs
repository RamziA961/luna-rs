use crate::{
    checks::{author_in_shared_voice_channel, author_in_voice_channel},
    server::{Context, ServerError},
};
use tracing::instrument;

/// Seek a position in the track.
#[poise::command(slash_command, subcommands("absolute"))]
pub async fn seek(_: Context<'_>) -> Result<(), ServerError> {
    Ok(())
}

/// Seek an absolute position in the track using a timestamp with the format HH:MM:SS.
#[instrument(skip(ctx))]
#[poise::command(
    slash_command,
    check = "author_in_voice_channel",
    check = "author_in_shared_voice_channel"
)]
pub async fn absolute(
    ctx: Context<'_>,
    #[description = "The desired timestamp (HH:MM:SS) to seek to."] position: String,
) -> Result<(), ServerError> {
    let guild_id = ctx
        .guild_id()
        .ok_or_else(|| ServerError::Internal("Could not find guild information".to_string()))?;

    ctx.defer().await?;

    let sliced = position.split(":").collect::<Vec<&str>>();

    if sliced.len() != 3 {
        ctx.reply(format!(
            "{position} is not a valid timestamp. Please use the HH:MM:SS format"
        ))
        .await?;
        return Ok(());
    }

    let parsed = sliced
        .iter()
        .map(|v| v.parse::<u64>())
        .try_fold(vec![], |mut accum, curr| match curr {
            Ok(curr) => {
                accum.push(curr);
                Ok(accum)
            }
            Err(e) => Err(e),
        });

    let timestamp = parsed
        .ok()
        .map(|args| std::time::Duration::from_secs(args[0] * 3600 + args[1] * 60 + args[2]));

    if let Some(timestamp) = timestamp {
        let mut guard = ctx.data().guild_map.write().await;
        let state = guard.get_mut(&guild_id.to_string()).unwrap();
        let track_handle = state.playback_state.get_track_handle_mut();

        if let Some(handle) = track_handle {
            _ = handle.seek(timestamp);
        } else {
            ctx.reply("Nothing currently playing").await?;
        }
    } else {
        ctx.reply(format!(
            "{position} is not a valid timestamp. Please use the HH:MM:SS format"
        ))
        .await?;
    }

    Ok(())
}
