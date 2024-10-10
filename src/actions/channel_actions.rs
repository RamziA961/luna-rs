use crate::{
    event_handlers::{
        disconnect_handler::DisconnectHandler, error_handler::ErrorHandler,
        inactivity_handler::InactivityHandler,
    },
    server::{Context, ServerError},
};
use songbird::{CoreEvent, Event};
use tracing::{error, instrument};

#[instrument(skip_all)]
pub async fn join_channel(ctx: Context<'_>) -> Result<(), ServerError> {
    let guild_id = ctx.guild_id().ok_or_else(|| {
        error!("Could not locate voice channel. Guild ID is none");
        ServerError::InternalError("Could not find guild information".to_string())
    })?;

    if ctx
        .data()
        .guild_map
        .read()
        .await
        .get(&guild_id.to_string())
        .is_some()
    {
        return Ok(());
    }

    let channel_id = ctx
        .guild()
        .and_then(|guild| {
            guild
                .voice_states
                .get(&ctx.author().id)
                .and_then(|voice_state| voice_state.channel_id)
        })
        .ok_or_else(|| {
            error!("Could not locate voice channel for Guild ID: {guild_id}");
            ServerError::InternalError("Could not locate voice channel.".to_string())
        })?;

    let manager = songbird::get(ctx.serenity_context())
        .await
        .ok_or_else(|| ServerError::InternalError("Could not find Songbird client.".to_string()))?;

    let join_result = manager.join(guild_id, channel_id).await;

    match join_result {
        Ok(handle_lock) => {
            let mut handle = handle_lock.lock().await;

            handle.add_global_event(
                Event::Core(CoreEvent::DriverDisconnect),
                DisconnectHandler::new(&guild_id, ctx.data().guild_map.clone(), manager.clone()),
            );

            handle.add_global_event(
                Event::Core(CoreEvent::ClientDisconnect),
                InactivityHandler::new(
                    &guild_id,
                    manager.clone(),
                    ctx.serenity_context().cache.clone(),
                ),
            );
            handle.add_global_event(Event::Track(songbird::TrackEvent::Error), ErrorHandler);

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
