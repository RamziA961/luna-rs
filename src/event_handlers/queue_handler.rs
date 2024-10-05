use std::{collections::HashMap, sync::Arc};

use async_trait::async_trait;
use poise::serenity_prelude::GuildId;
use songbird::{Event, EventContext, EventHandler};
use tokio::sync::{Mutex, RwLock};
use tracing::{error, instrument};

use crate::models::GuildState;

#[derive(Debug)]
pub struct QueueHandler {
    guild_id: GuildId,
    guild_map: Arc<RwLock<HashMap<String, GuildState>>>,
    handler: Arc<Mutex<songbird::Call>>,
    request_client: reqwest::Client,
}

impl QueueHandler {
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
    #[instrument(skip(_e))]
    async fn act(&self, _e: &EventContext<'_>) -> Option<Event> {
        let mut guard = self.guild_map.write().await;
        let guild_state = guard.get_mut(&self.guild_id.to_string());

        if guild_state.is_none() {
            return None;
        }

        let guild_state = guild_state.unwrap();
        guild_state.playback_state.play_next();

        let next = guild_state.playback_state.get_current_track();

        match next {
            Some(t) => {
                let t_handle = self.handler.lock().await.play(
                    songbird::input::YoutubeDl::new(self.request_client.clone(), t.url.clone())
                        .into(),
                );

                _ = t_handle
                    .add_event(
                        Event::Track(songbird::TrackEvent::End),
                        Self::new(
                            &self.guild_id,
                            self.guild_map.clone(),
                            self.handler.clone(),
                            self.request_client.clone(),
                        ),
                    )
                    .map_err(|e| {
                        error!(err=%e, "Failed to add event handler.");
                    });
                None
            }
            None => None,
        }
    }
}
