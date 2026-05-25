use crate::{
    checks::{author_in_shared_voice_channel, author_in_voice_channel},
    models::{DiscordError, InternalError, RuntimeError},
    server::Context,
};
use tracing::instrument;

/// Seek a position in the track.
#[poise::command(slash_command, subcommands("absolute"))]
pub async fn seek(_: Context<'_>) -> Result<(), RuntimeError> {
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
) -> Result<(), RuntimeError> {
    ctx.defer().await.map_err(DiscordError::Gateway)?;

    let guild_id = ctx
        .guild_id()
        .ok_or(InternalError::GuildInformationMissing)?;

    let sliced = position.split(":").collect::<Vec<&str>>();

    if sliced.len() != 3 {
        return Err(RuntimeError::User(format!(
            "{position} is not a valid timestamp. Please use the HH:MM:SS format"
        )));
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

    return if let Some(timestamp) = timestamp {
        let mut guard = ctx.data().guild_map.write().await;
        let state = guard.get_mut(&guild_id.to_string()).unwrap();
        let track_handle = state.playback_state.get_track_handle_mut();

        return if let Some(handle) = track_handle {
            _ = handle.seek(timestamp);
            Ok(())
        } else {
            Err(RuntimeError::User("Nothing currently playing".to_string()))
        };
    } else {
        Err(RuntimeError::User(format!(
            "{position} is not a valid timestamp. Please use the HH:MM:SS format"
        )))
    };
}
