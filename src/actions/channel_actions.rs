use crate::{
    event_handlers::disconnect_handler::DisconnectHandler,
    server::{Context, ServerError},
};
use songbird::CoreEvent;
use tracing::{error, instrument};

#[instrument(skip_all)]
pub async fn join_channel(ctx: Context<'_>) -> Result<(), ServerError> {
    let guild_id = ctx.guild_id();

    if guild_id.is_none() {
        error!("Could not locate voice channel. Guild ID is none");
        return Err(ServerError::InternalError(
            "Could not find guild information".to_string(),
        ));
    }

    let guild_id = guild_id.unwrap();

    let channel_id = if let Some(guild) = ctx.guild() {
        guild
            .voice_states
            .get(&ctx.author().id)
            .and_then(|voice_state| voice_state.channel_id)
    } else {
        None
    };

    if channel_id.is_none() {
        error!("Could not locate voice channel for Guild ID: {guild_id}");
        return Err(ServerError::InternalError(
            "Could not locate voice channel.".to_string(),
        ));
    }

    let channel_id = channel_id.unwrap();

    let manager = songbird::get(ctx.serenity_context())
        .await
        .ok_or_else(|| ServerError::InternalError("Could not find Songbird client.".to_string()))?;

    let join_result = manager.join(guild_id, channel_id).await;

    match join_result {
        Ok(handle_lock) => {
            let mut handle = handle_lock.lock().await;

            handle.add_global_event(
                songbird::Event::Core(CoreEvent::DriverDisconnect),
                DisconnectHandler::new(&guild_id, ctx.data().guild_map.clone(), manager),
            );

            Ok(())
        }
        Err(e) => {
            error!(e=%e, "Could not join voice channel {channel_id} in guild {guild_id}");
            Err(ServerError::PermissionsError(format!(
                "Sorry {}. I couldn't join your voice channel.\
                    Please ensure that I have the permissions needed to join.",
                ctx.author().name
            )))
        }
    }
}
