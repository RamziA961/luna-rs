use async_trait::async_trait;
use poise::serenity_prelude::{self, CacheHttp, GuildChannel, GuildId};
use songbird::{Event, EventContext, EventHandler};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::{Mutex, RwLock};
use tracing::{error, instrument, trace};

use crate::{
    embeds,
    models::{GuildState, VideoMetadata, YoutubeClient},
    stream,
};

#[derive(Debug, Clone)]
pub struct QueueHandler {
    serenity_ctx: serenity_prelude::Context,
    guild_id: GuildId,
    guild_channel: GuildChannel,
    guild_map: Arc<RwLock<HashMap<String, GuildState>>>,
    handler: Arc<Mutex<songbird::Call>>,
    youtube_client: YoutubeClient,
}

impl QueueHandler {
    pub fn new(
        serenity_ctx: serenity_prelude::Context,
        guild_id: &GuildId,
        guild_channel: GuildChannel,
        guild_map: Arc<RwLock<HashMap<String, GuildState>>>,
        handler: Arc<Mutex<songbird::Call>>,
        youtube_client: YoutubeClient,
    ) -> Self {
        Self {
            serenity_ctx,
            guild_id: *guild_id,
            guild_channel,
            guild_map,
            handler,
            youtube_client,
        }
    }
}

#[async_trait]
impl EventHandler for QueueHandler {
    #[instrument(skip_all, fields(guild_id = %self.guild_id))]
    async fn act(&self, _e: &EventContext<'_>) -> Option<Event> {
        trace!("Track has ended. Handler called to action.");
        let guild_key = self.guild_id.to_string();

        let (queued_track, radio_seed) = {
            let mut map_guard = self.guild_map.write().await;
            let guild_state = map_guard.get_mut(&guild_key)?;

            guild_state.playback_state.play_next();
            (
                guild_state.playback_state.get_current_track().clone(),
                guild_state.playback_state.get_radio_seed(),
            )
        };

        let Some((track, embed)) = self
            .resolve_next_track(queued_track, radio_seed, &guild_key)
            .await
        else {
            trace!("No track to play, stopping playback.");
            return None;
        };

        trace!(?track, "Next track resolved for playback.");
        self.play_and_notify(track, embed, &guild_key).await;

        None
    }
}

impl QueueHandler {
    #[instrument(skip(self))]
    async fn resolve_next_track(
        &self,
        queued_track: Option<VideoMetadata>,
        radio_seed: Option<String>,
        guild_key: &str,
    ) -> Option<(VideoMetadata, serenity_prelude::CreateEmbed)> {
        if let Some(track) = queued_track {
            let embed = embeds::create_playing_track_embed(&track);
            return Some((track, embed));
        }

        let seed_url = radio_seed?;
        trace!(%seed_url, "Queue empty. Radio mode active. Fetching related track.");

        let radio_track = match self.youtube_client.get_related_video(&seed_url).await {
            Ok(t) => t,
            Err(e) => {
                error!(err = %e, "Radio mode failed to fetch a related track.");
                return None;
            }
        };

        // Update state with the newly fetched radio track
        let mut map_guard = self.guild_map.write().await;
        if let Some(guild_state) = map_guard.get_mut(guild_key) {
            guild_state
                .playback_state
                .set_current_track(Some(radio_track.clone()));
            guild_state.playback_state.set_playing(true);
        }

        let embed = embeds::create_radio_playing_embed(&radio_track);
        Some((radio_track, embed))
    }

    #[instrument(skip(self, embed))]
    async fn play_and_notify(
        &self,
        track: VideoMetadata,
        embed: serenity_prelude::CreateEmbed,
        guild_key: &str,
    ) {
        let (mut call_guard, message_res) = tokio::join!(
            self.handler.lock(),
            self.guild_channel.send_message(
                self.serenity_ctx.http(),
                poise::serenity_prelude::CreateMessage::default().embed(embed)
            )
        );

        if let Err(e) = message_res {
            error!(err = %e, "Failed to send playback embed notification.");
        }

        match stream::create_audio_stream(&track.url) {
            Ok(track_input) => {
                let track_handle = call_guard.play(track_input.into());

                if let Err(e) =
                    track_handle.add_event(Event::Track(songbird::TrackEvent::End), self.clone())
                {
                    error!(err = %e, "Failed to cascade next track event handler.");
                }

                let mut map_guard = self.guild_map.write().await;
                if let Some(guild_state) = map_guard.get_mut(guild_key) {
                    guild_state
                        .playback_state
                        .set_track_handle(Some(track_handle));
                }
            }
            Err(err_msg) => {
                error!(err = %err_msg, "Failed to transition to the next track stream.");
            }
        }
    }
}
