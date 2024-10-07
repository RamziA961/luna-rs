use crate::{
    event_handlers::queue_handler::QueueHandler,
    models::QueueElement,
    server::{Context, ServerError},
};
use songbird::{Event, TrackEvent};
use tracing::{error, instrument, trace};

#[instrument(skip(ctx))]
pub async fn start_queue_playback(ctx: &Context<'_>) -> Result<(), ServerError> {
    trace!("Attempting to start queue playback");
    let guild_id = ctx.guild_id().ok_or_else(|| {
        ServerError::InternalError("Could not find guild information".to_string())
    })?;

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

    let manager_lock = manager.get_or_insert(guild_id);
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
                //ctx.clone()
            ),
        )
        .map_err(|e| {
            error!(err=%e, "Failed to add queue event handler.");
        });

    guild_state.playback_state.set_track_handle(Some(t_handle));

    Ok(())
}

#[instrument(skip(ctx))]
pub async fn add_element_to_queue(
    ctx: &Context<'_>,
    queue_element: QueueElement,
) -> Result<(), ServerError> {
    let guild_id = ctx.guild_id().ok_or_else(|| {
        ServerError::InternalError("Could not find guild information".to_string())
    })?;

    let mut guard = ctx.data().guild_map.write().await;
    let guild_state = guard.entry(guild_id.to_string()).or_default();
    guild_state.playback_state.enqueue(queue_element.clone());

    _ = ctx
        .reply(format!(
            "{} {}",
            if guild_state.playback_state.is_playing() {
                "Queued"
            } else {
                "Playing"
            },
            queue_element
        ))
        .await;

    Ok(())
}

pub async fn stop(ctx: &Context<'_>) -> Result<(), ServerError> {
    let guild_id = ctx.guild_id().ok_or_else(|| {
        ServerError::InternalError("Could not find guild information".to_string())
    })?;

    let mut guard = ctx.data().guild_map.write().await;
    trace!(guild_id=?guild_id, "Resetting guild state.");

    guard
        .get_mut(&guild_id.to_string())
        .map(|state| state.playback_state.reset());
    drop(guard);

    trace!("Stopping current track.");
    let handler = songbird::get(ctx.serenity_context())
        .await
        .and_then(|manager| manager.get(guild_id));

    if let Some(handle) = handler {
        handle.lock().await.stop();
        _ = ctx
            .reply("Halted playback and reset the track queue.")
            .await;
    } else {
        trace!("Nothing currently playing.");
        _ = ctx.reply("Nothing is currently playing").await;
    }

    Ok(())
}

pub async fn pause(ctx: &Context<'_>) -> Result<(), ServerError> {
    let guild_id = ctx.guild_id().ok_or_else(|| {
        ServerError::InternalError("Could not find guild information".to_string())
    })?;

    let mut guard = ctx.data().guild_map.write().await;
    let guild_state = guard.get_mut(&guild_id.to_string());
    let track_data = guild_state.and_then(|guild_state| {
        Some((
            guild_state.playback_state.get_current_track().clone(),
            guild_state.playback_state.get_track_handle().clone(),
        ))
    });

    if let Some((Some(current_track), Some(track_handle))) = track_data {
        _ = track_handle.pause();
        _ = ctx
            .reply(format!("Paused {}", current_track.to_string()))
            .await;
    } else {
        _ = ctx.reply("Nothing is currently playing.")
    }

    Ok(())
}

pub async fn resume(ctx: &Context<'_>) -> Result<(), ServerError> {
    let guild_id = ctx.guild_id().ok_or_else(|| {
        ServerError::InternalError("Could not find guild information".to_string())
    })?;

    let mut guard = ctx.data().guild_map.write().await;
    let guild_state = guard.get_mut(&guild_id.to_string());
    let track_data = guild_state.and_then(|guild_state| {
        Some((
            guild_state.playback_state.get_current_track().clone(),
            guild_state.playback_state.get_track_handle().clone(),
        ))
    });

    if let Some((Some(current_track), Some(track_handle))) = track_data {
        _ = track_handle.play();
        _ = ctx
            .reply(format!("Resumed {}", current_track.to_string()))
            .await;
    } else {
        _ = ctx.reply("Nothing is currently playing.")
    }

    Ok(())
}

pub async fn skip(ctx: &Context<'_>, n: usize) -> Result<(), ServerError> {
    let guild_id = ctx.guild_id().ok_or_else(|| {
        ServerError::InternalError("Could not find guild information".to_string())
    })?;

    let mut guard = ctx.data().guild_map.write().await;
    let guild_state = guard.get_mut(&guild_id.to_string()).ok_or_else(|| {
        ServerError::InternalError("Could not find guild playback information".to_string())
    })?;

    let track_handle = if let Some(handle) = guild_state.playback_state.get_track_handle().clone() {
        handle
    } else {
        _ = ctx.reply("The queue is empty.").await;
        return Ok(());
    };

    for _ in 0..n - 1 {
        guild_state.playback_state.dequeue();
    }

    let next = guild_state
        .playback_state
        .next()
        .as_ref()
        .map_or_else(|| "".to_string(), |t| format!("\n{}", t.to_string()));

    _ = ctx.reply(format!("Skipped {n} tracks.{next}")).await;
    _ = track_handle.stop();

    Ok(())
}
