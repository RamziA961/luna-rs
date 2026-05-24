use crate::{
    embeds,
    event_handlers::queue_handler::QueueHandler,
    models::QueueElement,
    server::{Context, ServerError},
};
use songbird::{Event, TrackEvent};
use tracing::{error, instrument, trace};

#[instrument(skip_all, fields(guild_id = %ctx.guild_id().unwrap_or_default()))]
pub async fn start_queue_playback(ctx: &Context<'_>) -> Result<(), ServerError> {
    trace!("Attempting to start queue playback");
    let guild_id = ctx
        .guild_id()
        .ok_or_else(|| ServerError::Internal("Could not find guild information.".to_string()))?;

    let guild_key = guild_id.to_string();
    let channel = ctx
        .guild_channel()
        .await
        .ok_or_else(|| ServerError::Internal("Could not obtain guild channel.".to_string()))?;

    // Extract track info and modify queue state
    let url = {
        let mut map_guard = ctx.data().guild_map.write().await;
        let guild_state = map_guard.get_mut(&guild_key).ok_or_else(|| {
            ServerError::Internal("Could not find guild playback information.".to_string())
        })?;

        if guild_state.playback_state.is_playing() {
            trace!("Playback already in progress.");
            return Ok(());
        }

        guild_state.playback_state.play_next();
        guild_state
            .playback_state
            .get_current_track()
            .as_ref()
            .map(|track| track.url.clone())
            .ok_or_else(|| {
                ServerError::Internal("Queue state updated but track is missing.".to_string())
            })?
    };

    let manager = songbird::get(ctx.serenity_context()).await.ok_or_else(|| {
        error!("Failed to get songbird manager from context.");
        ServerError::Internal("Unable to begin playback.".to_string())
    })?;

    let manager_lock = manager.get_or_insert(guild_id);
    trace!("Commencing download and audio conversion of video.");

    let track_input = crate::stream::create_audio_stream(&url).map_err(|e| {
        error!(err = %e, "Failed to instantiate custom audio stream pipeline.");
        ServerError::Internal("Failed to process audio stream pipeline.".to_string())
    })?;

    let mut call_guard = manager_lock.lock().await;
    trace!("Attempting to play converted track.");
    let t_handle = call_guard.play(track_input.into());

    if let Err(e) = t_handle.add_event(
        Event::Track(TrackEvent::End),
        QueueHandler::new(
            ctx.serenity_context().clone(),
            &guild_id,
            channel,
            ctx.data().guild_map.clone(),
            manager_lock.clone(),
        ),
    ) {
        error!(err = %e, "Failed to add queue event handler.");
    }

    // Update the track handle reference back in the map safely
    let mut map_guard = ctx.data().guild_map.write().await;
    if let Some(guild_state) = map_guard.get_mut(&guild_key) {
        guild_state.playback_state.set_track_handle(Some(t_handle));
    }

    Ok(())
}

#[instrument(skip(ctx))]
pub async fn add_element_to_queue(
    ctx: &Context<'_>,
    queue_element: QueueElement,
) -> Result<(), ServerError> {
    let guild_id = ctx
        .guild_id()
        .ok_or_else(|| ServerError::Internal("Could not find guild information".to_string()))?;

    let mut map_guard = ctx.data().guild_map.write().await;
    let guild_state = map_guard.entry(guild_id.to_string()).or_default();
    guild_state.playback_state.enqueue(queue_element.clone());

    let is_playing = guild_state.playback_state.is_playing();

    drop(map_guard);

    let embed = match queue_element {
        QueueElement::Track(t) if is_playing => embeds::create_queued_track_embed(&t),
        QueueElement::Track(t) => embeds::create_playing_track_embed(&t),
        QueueElement::Playlist(p) if is_playing => embeds::create_queued_playlist_embed(&p),
        QueueElement::Playlist(p) => embeds::create_playling_playlist_embed(&p),
    };

    ctx.send(poise::CreateReply::default().embed(embed)).await?;
    Ok(())
}

pub async fn stop(ctx: &Context<'_>) -> Result<(), ServerError> {
    let guild_id = ctx
        .guild_id()
        .ok_or_else(|| ServerError::Internal("Could not find guild information".to_string()))?;

    {
        let mut map_guard = ctx.data().guild_map.write().await;
        trace!(%guild_id, "Resetting guild state.");
        if let Some(state) = map_guard.get_mut(&guild_id.to_string()) {
            state.playback_state.reset();
        }
    }

    trace!("Stopping current track.");
    let handler = songbird::get(ctx.serenity_context())
        .await
        .and_then(|manager| manager.get(guild_id));

    let Some(handle) = handler else {
        trace!("Nothing currently playing.");
        ctx.reply("Nothing is currently playing.").await?;
        return Ok(());
    };

    handle.lock().await.stop();
    ctx.reply("Halted playback and reset the track queue.")
        .await?;
    Ok(())
}

pub async fn pause(ctx: &Context<'_>) -> Result<(), ServerError> {
    let guild_id = ctx
        .guild_id()
        .ok_or_else(|| ServerError::Internal("Could not find guild information".to_string()))?;

    let track_data = {
        let map_guard = ctx.data().guild_map.read().await;
        map_guard.get(&guild_id.to_string()).map(|state| {
            (
                state.playback_state.get_current_track().clone(),
                state.playback_state.get_track_handle().clone(),
            )
        })
    };

    let Some((Some(current_track), Some(track_handle))) = track_data else {
        ctx.reply("Nothing is currently playing.").await?;
        return Ok(());
    };

    let _ = track_handle.pause();
    ctx.send(poise::CreateReply::default().embed(embeds::create_paused_embed(&current_track)))
        .await?;
    Ok(())
}

pub async fn resume(ctx: &Context<'_>) -> Result<(), ServerError> {
    let guild_id = ctx
        .guild_id()
        .ok_or_else(|| ServerError::Internal("Could not find guild information".to_string()))?;

    let track_data = {
        let map_guard = ctx.data().guild_map.read().await;
        map_guard.get(&guild_id.to_string()).map(|state| {
            (
                state.playback_state.get_current_track().clone(),
                state.playback_state.get_track_handle().clone(),
            )
        })
    };

    let Some((Some(current_track), Some(track_handle))) = track_data else {
        ctx.reply("Nothing is currently playing.").await?;
        return Ok(());
    };

    let _ = track_handle.play();
    ctx.send(
        poise::CreateReply::default().embed(embeds::create_resume_track_embed(&current_track)),
    )
    .await?;
    Ok(())
}

pub async fn skip(ctx: &Context<'_>, n: usize) -> Result<(), ServerError> {
    let guild_id = ctx
        .guild_id()
        .ok_or_else(|| ServerError::Internal("Could not find guild information".to_string()))?;

    let mut map_guard = ctx.data().guild_map.write().await;
    let guild_state = map_guard.get_mut(&guild_id.to_string()).ok_or_else(|| {
        ServerError::Internal("Could not find guild playback information".to_string())
    })?;

    let Some(track_handle) = guild_state.playback_state.get_track_handle().clone() else {
        ctx.reply("The queue is empty.").await?;
        return Ok(());
    };

    let mut skipped = 1;
    for _ in 0..(n.saturating_sub(1)) {
        guild_state.playback_state.dequeue();
        skipped += 1;
    }

    let next = guild_state.playback_state.next().cloned();
    let remaining_queued = guild_state.playback_state.number_of_tracks_queued();

    drop(map_guard);

    match next {
        Some(QueueElement::Playlist(p)) => {
            ctx.send(
                poise::CreateReply::default().embed(embeds::create_skip_playlist_embed(
                    &p,
                    skipped,
                    remaining_queued,
                )),
            )
            .await
        }
        Some(QueueElement::Track(t)) => {
            ctx.send(
                poise::CreateReply::default().embed(embeds::create_skip_track_embed(
                    &t,
                    skipped,
                    remaining_queued,
                )),
            )
            .await
        }
        None => {
            ctx.reply(format!(
                "Skipped {skipped} tracks. The queue has been exhausted."
            ))
            .await
        }
    }?;

    let _ = track_handle.stop();
    Ok(())
}

pub async fn show_queue(ctx: &Context<'_>) -> Result<(), ServerError> {
    let guild_id = ctx
        .guild_id()
        .ok_or_else(|| ServerError::Internal("Could not find guild information".to_string()))?;

    let queue_info = {
        let map_guard = ctx.data().guild_map.read().await;
        let guild_state = map_guard.get(&guild_id.to_string()).ok_or_else(|| {
            ServerError::Internal("Could not find guild playback information".to_string())
        })?;

        Some((
            guild_state.playback_state.next_items_queued(5),
            guild_state.playback_state.number_of_tracks_queued(),
            guild_state.playback_state.number_of_items_queued(),
        ))
    };

    let Some((next_tracks, n_tracks, n_items)) = queue_info else {
        ctx.reply("The queue is empty.").await?;
        return Ok(());
    };

    if n_items == 0 {
        ctx.reply("The queue is empty.").await?;
    } else {
        ctx.send(
            poise::CreateReply::default().embed(embeds::create_queue_overview_embed(
                &next_tracks,
                n_tracks,
                n_items,
            )),
        )
        .await?;
    }

    Ok(())
}
