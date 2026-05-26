use std::time::Duration;

use crate::{
    event_handlers::{
        disconnect_handler::DisconnectHandler, error_handler::ErrorHandler,
        inactivity_handler::InactivityHandler,
    },
    models::{DiscordError, InternalError, RuntimeError},
    server::Context,
};
use songbird::{CoreEvent, Event};
use tracing::{error, instrument};

#[instrument(skip_all)]
pub async fn join_channel(ctx: Context<'_>) -> Result<(), RuntimeError> {
    let guild_id = ctx
        .guild_id()
        .ok_or(InternalError::GuildInformationMissing)?;

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
    .ok_or(InternalError::VoiceChannelMissing)?;

    let manager = songbird::get(ctx.serenity_context())
        .await
        .ok_or_else(|| InternalError::DependencyMissing("Songbird Voice Client".to_string()))?;

    // Perform the connection join handshake
    let handle_lock = manager.join(guild_id, channel_id).await.map_err(|e| {
        error!(err = %e, "Could not join voice channel {channel_id} in guild {guild_id}");
        DiscordError::Join(e)
    })?;

    let mut handle = handle_lock.lock().await;

    // Triggered when the bot is disconnected or kicked from the voice region channel
    handle.add_global_event(
        Event::Core(CoreEvent::DriverDisconnect),
        DisconnectHandler::new(&guild_id, ctx.data().guild_map.clone(), manager.clone()),
    );

    // Run the inactivity check loop every 30s to see if humans left
    handle.add_global_event(
        Event::Periodic(Duration::from_secs(30), None),
        InactivityHandler::new(
            &guild_id,
            manager.clone(),
            ctx.serenity_context().cache.clone(),
        ),
    );

    // Intercept media streaming decoding/io errors
    handle.add_global_event(
        Event::Track(songbird::TrackEvent::Error),
        ErrorHandler::new(ctx.serenity_context().clone(), channel_id),
    );

    Ok(())
}
