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
    #[instrument(skip_all, fields(guild_id = self.guild_id.to_string()))]
    async fn act(&self, _e: &EventContext<'_>) -> Option<Event> {
        trace!("Track has ended. Handler called to action.");

        let mut guard = self.guild_map.write().await;
        let guild_state = guard.get_mut(&self.guild_id.to_string())?;
        guild_state.playback_state.play_next();

        trace!(guild_state=%guild_state, "Modified guild state to play next track.");
        let current_track = guild_state.playback_state.get_current_track().clone();

        if let Some(t) = current_track {
            trace!(track=?t, "Next track found.");

            let (mut guard, _) = tokio::join!(
                self.handler.lock(),
                self.guild_channel.send_message(
                    self.serenity_ctx.http(),
                    poise::serenity_prelude::CreateMessage::default()
                        .embed(embeds::create_playing_track_embed(&t))
                )
            );

            match stream::create_audio_stream(&t.url) {
                Ok(track_input) => {
                    let t_handle = guard.play(track_input.into());

                    _ = t_handle
                        .add_event(Event::Track(songbird::TrackEvent::End), self.clone())
                        .map_err(|e| {
                            error!(err=%e, "Failed to add event handler.");
                        });

                    guild_state.playback_state.set_track_handle(Some(t_handle));
                }
                Err(err_msg) => {
                    error!(err = %err_msg, "Failed to transition to the next track stream.");
                }
            }
        } else {
            trace!("No track queued to play.");
        };

        None
    }
}
