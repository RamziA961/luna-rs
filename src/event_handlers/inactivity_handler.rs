use async_trait::async_trait;
use poise::serenity_prelude::GuildId;
use songbird::{Event, EventContext, EventHandler};
use std::sync::Arc;
use tracing::{error, instrument, trace};

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
    #[instrument(skip_all, fields(guild_id = %self.guild_id))]
    async fn act(&self, _e: &EventContext<'_>) -> Option<Event> {
        // Evaluate everything cleanly inside a dedicated synchronous scope block.
        // This ensures the non-Send cache types drop *before* any .await boundary.
        let should_leave = {
            let guild = self.cache.guild(self.guild_id)?;

            let bot_id = self.cache.current_user().id;
            let target_channel = guild.voice_states.get(&bot_id)?.channel_id?;

            // Filter and count active human members inside our current channel
            let human_count = guild
                .voice_states
                .values()
                .filter(|vs| {
                    vs.channel_id == Some(target_channel)
                        && vs.member.as_ref().is_some_and(|m| !m.user.bot)
                })
                .count();

            Some(human_count == 0)
        }
        .unwrap_or(false);

        if should_leave {
            trace!("Voice channel is empty of humans. Leaving channel.");

            if let Err(e) = self.handler.leave(self.guild_id).await {
                error!(err = %e, "Could not leave voice channel");
            }
        }

        None
    }
}
