use std::{collections::HashMap, sync::Arc};

use async_trait::async_trait;
use poise::serenity_prelude::GuildId;
use songbird::{Event, EventContext, EventHandler};
use tokio::sync::RwLock;
use tracing::{error, instrument, trace};

use crate::models::GuildState;

#[derive(Debug)]
pub struct DisconnectHandler {
    guild_id: GuildId,
    guild_map: Arc<RwLock<HashMap<String, GuildState>>>,
    handler: Arc<songbird::Songbird>,
}

impl DisconnectHandler {
    pub fn new(
        guild_id: &GuildId,
        guild_map: Arc<RwLock<HashMap<String, GuildState>>>,
        handler: Arc<songbird::Songbird>,
    ) -> Self {
        Self {
            guild_id: *guild_id,
            guild_map,
            handler,
        }
    }
}

#[async_trait]
impl EventHandler for DisconnectHandler {
    #[instrument(skip_all, fields(guild_id=self.guild_id.to_string()))]
    async fn act(&self, _e: &EventContext<'_>) -> Option<Event> {
        trace!("Disconnected from a voice channel. Cleaning up guild state.");

        let mut guard = self.guild_map.write().await;
        
        // If guild state not present, terminate early.
        // This will likely mean, the inactivity handler,
        // was fired.
        guard.remove(&self.guild_id.to_string())?;
        drop(guard);

        _ = self
            .handler
            .remove(self.guild_id)
            .await
            .map_err(|e| error!(err=%e, "Failed to remove guild songbird state from manager."));

        None
    }
}
