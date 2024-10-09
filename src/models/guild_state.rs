use std::fmt::Display;

use super::PlaybackState;

#[derive(Debug, Clone, Default)]
pub struct GuildState {
    pub playback_state: PlaybackState,
}

impl Display for GuildState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "GuildState {{ playback_state: {} }}",
            self.playback_state
        )
    }
}
