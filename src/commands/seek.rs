use crate::{
    checks::{author_in_shared_voice_channel, author_in_voice_channel},
    server::{Context, ServerError},
};
use chrono::NaiveTime;
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
    let guild_id = ctx.guild_id().ok_or_else(|| {
        ServerError::InternalError("Could not find guild information".to_string())
    })?;

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
        .map(|v| v.parse::<u32>())
        .try_fold(vec![], |mut accum, curr| match curr {
            Ok(curr) => {
                accum.push(curr);
                Ok(accum)
            }
            Err(e) => Err(e),
        });

    if let Ok(args) = parsed {
        //let timestamp = NaiveTime::from_hms_opt(args[0], args[1], args[2]);
        ////let manager = songbird::get(ctx.serenity_context())
        ////    .await
        ////    .and_then(|m| m.get(guild_id))
        ////    .ok_or_else(|| {
        ////        ServerError::InternalError("Could not find music manager".to_string())
        ////    })?;

        //let mut guard = ctx.data().guild_map.write().await;
        //let state = guard.get(&guild_id).unwrap();
        //state
        //    .playback_state
        //    .get_track_handle_mut()
        //    .unwrap()
        //    .seek(timestamp);
    } else {
        ctx.reply(format!(
            "{position} is not a valid timestamp. Please use the HH:MM:SS format"
        ))
        .await?;
        return Ok(());
    }

    Ok(())
}
