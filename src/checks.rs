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
