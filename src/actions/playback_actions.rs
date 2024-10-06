use crate::{
    event_handlers::queue_handler::QueueHandler,
    server::{Context, ServerError},
};
use poise::serenity_prelude::GuildId;
use songbird::{Event, TrackEvent};
use tracing::{error, instrument, trace};

#[instrument(skip(ctx))]
pub async fn start_queue_playback(
    ctx: &Context<'_>,
    guild_id: &GuildId,
) -> Result<(), ServerError> {
    trace!("Attempting to start queue playback");
    let mut guard = ctx.data().guild_map.write().await;
    let req_client = &ctx.data().request_client;
    trace!("Write lock to guild map obtained.");

    let guild_state = guard.get_mut(&guild_id.to_string()).unwrap();
    if guild_state.playback_state.is_playing() {
        trace!("Playback already in progress.");
        return Ok(());
    }

    guild_state.playback_state.play_next();
    let url = guild_state
        .playback_state
        .get_current_track()
        .as_ref()
        .map(|track| track.url.clone())
        .unwrap();

    let manager = songbird::get(ctx.serenity_context()).await.ok_or_else(|| {
        error!("Failed to get songbird manager from context.");
        ServerError::InternalError("Unable to begin playback.".to_string())
    })?;

    let manager_lock = manager.get_or_insert(*guild_id);
    trace!("Commencing download and audio conversion of video.");
    let t = songbird::input::YoutubeDl::new(req_client.clone(), url);

    let mut guard = manager_lock.lock().await;
    trace!("Attempting to play converted track.");
    let t_handle = guard.play(t.into());

    _ = t_handle
        .add_event(
            Event::Track(TrackEvent::End),
            QueueHandler::new(
                &guild_id,
                ctx.data().guild_map.clone(),
                manager_lock.clone(),
                req_client.clone(),
            ),
        )
        .map_err(|e| {
            error!(err=%e, "Failed to add queue event handler.");
        });

    Ok(())
}
