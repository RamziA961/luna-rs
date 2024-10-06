use async_trait::async_trait;
use songbird::{Event, EventContext, EventHandler};
use tracing::{error, instrument};

pub struct ErrorHandler;

#[async_trait]
impl EventHandler for ErrorHandler {
    #[instrument(skip(self))]
    async fn act(&self, _e: &EventContext<'_>) -> Option<Event> {
        error!("Error detected. Error handler called to action.");
        None
    }
}
