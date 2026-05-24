use async_trait::async_trait;
use songbird::{Event, EventContext, EventHandler};
use tracing::{error, instrument};

pub struct ErrorHandler;

#[async_trait]
impl EventHandler for ErrorHandler {
    #[instrument(skip_all)]
    async fn act(&self, ctx: &EventContext<'_>) -> Option<Event> {
        if let EventContext::Track(track_events) = ctx {
            for (state, _handle) in *track_events {
                if let songbird::tracks::PlayMode::Errored(err) = &state.playing {
                    error!(err = %err, "Track playback error detected.");
                }
            }
        } else {
            error!("Generic or driver error event fired.");
        }

        None
    }
}
