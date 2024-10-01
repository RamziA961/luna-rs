use super::{PlaybackState, QueueElement};

#[derive(Debug, Clone, Default)]
pub struct GuildState {
    pub playback_state: PlaybackState,
}
