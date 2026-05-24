use async_trait::async_trait;
use poise::serenity_prelude::{self, CacheHttp, GuildChannel, GuildId};
use songbird::{Event, EventContext, EventHandler};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::{Mutex, RwLock};
use tracing::{error, instrument, trace};

use crate::{embeds, models::GuildState, stream};

#[derive(Debug, Clone)]
pub struct QueueHandler {
    serenity_ctx: serenity_prelude::Context,
    guild_id: GuildId,
    guild_channel: GuildChannel,
    guild_map: Arc<RwLock<HashMap<String, GuildState>>>,
    handler: Arc<Mutex<songbird::Call>>,
}

impl QueueHandler {
    pub fn new(
        serenity_ctx: serenity_prelude::Context,
        guild_id: &GuildId,
        guild_channel: GuildChannel,
        guild_map: Arc<RwLock<HashMap<String, GuildState>>>,
        handler: Arc<Mutex<songbird::Call>>,
    ) -> Self {
        Self {
            serenity_ctx,
            guild_id: *guild_id,
            guild_channel,
            guild_map,
            handler,
        }
    }
}

#[async_trait]
impl EventHandler for QueueHandler {
    #[instrument(skip_all, fields(guild_id = %self.guild_id))]
    async fn act(&self, _e: &EventContext<'_>) -> Option<Event> {
        trace!("Track has ended. Handler called to action.");

        let guild_key = self.guild_id.to_string();

        let current_track = {
            let mut map_guard = self.guild_map.write().await;
            let guild_state = map_guard.get_mut(&guild_key)?;
            guild_state.playback_state.play_next();

            trace!(%guild_state, "Modified guild state to play next track.");
            guild_state.playback_state.get_current_track().clone()
        };

        let Some(track) = current_track else {
            trace!("No track queued to play.");
            return None;
        };
        trace!(?track, "Next track found.");

        let (mut call_guard, message_res) = tokio::join!(
            self.handler.lock(),
            self.guild_channel.send_message(
                self.serenity_ctx.http(),
                poise::serenity_prelude::CreateMessage::default()
                    .embed(embeds::create_playing_track_embed(&track))
            )
        );

        if let Err(e) = message_res {
            error!(err = %e, "Failed to send next track playback embed notification.");
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

                if let Some(guild_state) = map_guard.get_mut(&guild_key) {
                    guild_state
                        .playback_state
                        .set_track_handle(Some(track_handle));
                }
            }
            Err(err_msg) => {
                error!(err = %err_msg, "Failed to transition to the next track stream.");
            }
        }

        None
    }
}
