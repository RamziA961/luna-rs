use std::{collections::HashMap, sync::Arc};

use async_trait::async_trait;
use poise::serenity_prelude::{model::guild, GuildId};
use songbird::{Event, EventContext, EventHandler};
use tokio::sync::{Mutex, RwLock};
use tracing::{error, instrument, trace};

use crate::{models::GuildState, server::Context};

#[derive(Debug)]
pub struct QueueHandler {
    guild_id: GuildId,
    guild_map: Arc<RwLock<HashMap<String, GuildState>>>,
    handler: Arc<Mutex<songbird::Call>>,
    request_client: reqwest::Client,
}

impl<'a> QueueHandler {
    pub fn new(
        guild_id: &GuildId,
        guild_map: Arc<RwLock<HashMap<String, GuildState>>>,
        handler: Arc<Mutex<songbird::Call>>,
        request_client: reqwest::Client,
    ) -> Self {
        Self {
            guild_id: guild_id.clone(),
            guild_map,
            handler,
            request_client,
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
        trace!(guild_state=?guild_state, "Modified guild state to play next track.");

        let current_track = guild_state.playback_state.get_current_track().clone();

        if let Some(t) = current_track {
            trace!(track=?t, "Next track found.");
            let mut guard = self.handler.lock().await;

            let t_handle = guard.play(
                songbird::input::YoutubeDl::new(self.request_client.clone(), t.url.clone()).into(),
            );

            //_ = self.ctx.reply(t.to_string()).await;

            _ = t_handle
                .add_event(
                    Event::Track(songbird::TrackEvent::End),
                    Self::new(
                        &self.guild_id,
                        self.guild_map.clone(),
                        self.handler.clone(),
                        self.request_client.clone(),
                        //self.ctx.clone(),
                    ),
                )
                .map_err(|e| {
                    error!(err=%e, "Failed to add event handler.");
                });

            guild_state.playback_state.set_track_handle(Some(t_handle));
        } else {
            trace!("No track queued to play.");
        };

        None
    }
}
