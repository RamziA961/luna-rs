use tracing::error;

use crate::server::{Context, ServerError};

pub async fn author_in_voice_channel(ctx: Context<'_>) -> Result<bool, ServerError> {
    let author = ctx.author();
    let is_in_vc = ctx
        .guild()
        .map(|guild| guild.voice_states.contains_key(&author.id));

    match is_in_vc {
        None => {
            error!("Could not validate if author in voice channel.");
            Err(ServerError::InternalError(
                "Could not validate if author in voice channel.".to_string(),
            ))
        }
        Some(false) => {
            ctx.reply("Please join a voice channel to initiate this command.")
                .await?;
            Ok(false)
        }
        Some(true) => Ok(true),
    }
}

pub async fn author_in_shared_voice_channel(ctx: Context<'_>) -> Result<bool, ServerError> {
    let author = ctx.author();

    let author_vc = ctx
        .guild()
        .as_ref()
        .and_then(|guild| guild.voice_states.get(&author.id))
        .and_then(|voice_state| voice_state.channel_id);

    let bot_vc = ctx
        .guild()
        .as_ref()
        .and_then(|guild| guild.voice_states.get(&ctx.framework().bot_id))
        .and_then(|voice_state| voice_state.channel_id);

    match (author_vc, bot_vc) {
        (Some(author_vc), Some(bot_vc)) if author_vc == bot_vc => Ok(true),
        (Some(_), None) => Ok(true),
        (Some(_), Some(_)) | (None, Some(_)) => {
            _ = ctx.reply("Please join a shared voice channel to issue this command.")
                .await;
            Ok(false)
        },
        (None, None) => {
            _ = ctx.reply("Please join a voice channel to initiate this command.")
                .await;
            Ok(false)
        },
        
    }
}
