use crate::server::{Context, ServerError};
use poise::serenity_prelude::GuildId;
use tracing::{error, info, instrument, trace};

#[instrument(skip(ctx))]
pub async fn start_queue_playback(ctx: &Context<'_>, guild_id: &GuildId) -> Result<(), ServerError> {
    let mut guard = ctx.data().guild_map.write().await;

    let guild_state = guard.get_mut(&guild_id.to_string()).unwrap();
    if guild_state.playback_state.is_playing() {
        return Ok(());
    }

    guild_state.playback_state.play_next();
    let url = guild_state
        .playback_state
        .get_current_track()
        .as_ref()
        .map(|track| track.url.clone());

    if url.is_none() {
        trace!(guild_state=?guild_state, "No tracks queued");
        return Ok(());
    }

    let url = url.unwrap();

    let client = reqwest::Client::new();
    let manager = songbird::get(ctx.serenity_context()).await.ok_or_else(|| {
        error!("Failed to get songbird manager from context.");
        ServerError::InternalError("Unable to begin playback.".to_string())
    })?;

    let manager_lock = manager.get_or_insert(*guild_id);
    let t = songbird::input::YoutubeDl::new(client, url);

    let mut guard = manager_lock.lock().await;
    let handle = guard.play(t.into());

    Ok(())
}
