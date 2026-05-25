use tracing::{instrument, trace};

use crate::{
    models::{InternalError, RuntimeError},
    server::Context,
};

#[instrument(skip_all, fields(guild_id = %ctx.guild_id().unwrap_or_default()))]
pub async fn author_in_voice_channel(ctx: Context<'_>) -> Result<bool, RuntimeError> {
    let author_id = ctx.author().id;

    // Scope block to extract data and drop the non-Send guild object
    let is_in_vc = {
        let guild = ctx
            .guild()
            .ok_or(InternalError::GuildInformationMissing)?;

        guild.voice_states.contains_key(&author_id)
    };

    if !is_in_vc {
        trace!("Command dispatched by author not in a voice channel.");
        return Err(RuntimeError::User(
            "Please join a voice channel to initiate this command.".to_string(),
        ));
    }

    Ok(true)
}

#[instrument(skip_all, fields(guild_id = %ctx.guild_id().unwrap_or_default()))]
pub async fn author_in_shared_voice_channel(ctx: Context<'_>) -> Result<bool, RuntimeError> {
    let author_id = ctx.author().id;
    let bot_id = ctx.framework().bot_id;

    let (author_vc, bot_vc) = {
        let guild = ctx
            .guild()
            .ok_or(InternalError::GuildInformationMissing)?;

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
        (Some(_), Some(_)) | (None, Some(_)) => Err(RuntimeError::User(
            "Please join a shared voice channel to issue this command.".to_string(),
        )),
        (None, None) => Err(RuntimeError::User(
            "Please join a voice channel to initiate this command.".to_string(),
        )),
    }
}
