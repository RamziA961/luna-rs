use async_trait::async_trait;
use poise::serenity_prelude;
use songbird::{Event, EventContext, EventHandler};
use tracing::{error, instrument};

pub struct ErrorHandler {
    serenity_ctx: serenity_prelude::Context,
    channel_id: serenity_prelude::ChannelId,
}

impl ErrorHandler {
    pub fn new(
        serenity_ctx: serenity_prelude::Context,
        channel_id: serenity_prelude::ChannelId,
    ) -> Self {
        Self {
            channel_id,
            serenity_ctx,
        }
    }
}

#[async_trait]
impl EventHandler for ErrorHandler {
    #[instrument(skip_all)]
    async fn act(&self, ctx: &EventContext<'_>) -> Option<Event> {
        if let EventContext::Track(track_events) = ctx {
            for (state, handle) in *track_events {
                if let songbird::tracks::PlayMode::Errored(err) = &state.playing {
                    error!(err = %err, "Track playback error detected.");
                    let embed = crate::embeds::create_error_embed(&format!(
                        "An error occurred while playing the next track",
                    ));

                    let _ = self
                        .channel_id
                        .send_message(
                            &self.serenity_ctx,
                            poise::serenity_prelude::CreateMessage::default().embed(embed),
                        )
                        .await;

                    let _ = handle.stop();
                }
            }
        } else {
            error!("Generic or driver error event fired.");
        }

        None
    }
}
