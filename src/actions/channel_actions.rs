use std::time::Duration;

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
        ServerError::Internal("Could not find guild information".to_string())
    })?;

    let guild_key = guild_id.to_string();

    // If the bot is already tracked as active in this guild, step out.
    if ctx.data().guild_map.read().await.contains_key(&guild_key) {
        return Ok(());
    }

    // Isolate the cache lookup inside a temporary scope block to avoid Send/Sync lifetime leaks.
    let channel_id = {
        ctx.guild()
            .as_deref()
            .and_then(|guild| guild.voice_states.get(&ctx.author().id))
            .and_then(|voice_state| voice_state.channel_id)
    }
    .ok_or_else(|| {
        error!("Could not locate voice channel for Guild ID: {guild_id}");
        ServerError::Internal("Could not locate your voice channel. Are you connected?".to_string())
    })?;

    let manager = songbird::get(ctx.serenity_context())
        .await
        .ok_or_else(|| ServerError::Internal("Could not find Songbird client.".to_string()))?;

    // Perform the connection join handshake
    let handle_lock = manager.join(guild_id, channel_id).await.map_err(|e| {
        error!(err = %e, "Could not join voice channel {channel_id} in guild {guild_id}");
        ServerError::Permissions(format!(
            "Sorry {}. I couldn't join your voice channel. Please ensure I have connect/speak permissions.",
            ctx.author().name
        ))
    })?;

    let mut handle = handle_lock.lock().await;

    // Triggered when the bot is disconnected or kicked from the voice region channel
    handle.add_global_event(
        Event::Core(CoreEvent::DriverDisconnect),
        DisconnectHandler::new(&guild_id, ctx.data().guild_map.clone(), manager.clone()),
    );

    // Run the inactivity check loop every 15 seconds to see if humans left
    handle.add_global_event(
        Event::Periodic(Duration::from_secs(15), None),
        InactivityHandler::new(
            &guild_id,
            manager.clone(),
            ctx.serenity_context().cache.clone(),
        ),
    );

    // Intercept media streaming decoding/io errors
    handle.add_global_event(Event::Track(songbird::TrackEvent::Error), ErrorHandler);

    Ok(())
}
