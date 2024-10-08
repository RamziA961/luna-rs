use async_trait::async_trait;
use poise::serenity_prelude::GuildId;
use songbird::{Event, EventContext, EventHandler};
use std::sync::Arc;
use tracing::{error, trace};

#[derive(Debug)]
pub struct InactivityHandler {
    guild_id: GuildId,
    cache: Arc<poise::serenity_prelude::Cache>,
    handler: Arc<songbird::Songbird>,
}

impl InactivityHandler {
    pub fn new(
        guild_id: &GuildId,
        handler: Arc<songbird::Songbird>,
        cache: Arc<poise::serenity_prelude::Cache>,
    ) -> Self {
        Self {
            guild_id: *guild_id,
            handler,
            cache,
        }
    }
}

#[async_trait]
impl EventHandler for InactivityHandler {
    async fn act(&self, _e: &EventContext<'_>) -> Option<Event> {
        let channel_id = self
            .cache
            .guild(self.guild_id)
            .as_ref()
            .and_then(|guild| guild.voice_states.get(&self.cache.current_user().id))
            .and_then(|voice_state| voice_state.channel_id);

        let member_count = self
            .cache
            .guild(self.guild_id)
            .as_ref()
            .map(|guild| {
                guild
                    .voice_states
                    .values()
                    .filter(|voice_state| {
                        voice_state.channel_id == channel_id
                            && voice_state.member.as_ref().is_some_and(|m| !m.user.bot)
                    })
                    .count()
            })
            .unwrap_or(0);

        if member_count == 0 {
            trace!("Leaving empty channel an empty voice channel. Leaving channel.");
            // We only need to leave and let the disconnect handler deal with
            // cleaning up the resources.
            _ = self.handler.leave(self.guild_id).await
                .map_err(|e| error!(err=%e, "Could not leave voice channel"));
        }

        None
    }
}
