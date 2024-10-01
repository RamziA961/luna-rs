mod guild_state;
mod playback_state;
mod youtube;

pub use guild_state::GuildState;

pub use playback_state::PlaybackState;
pub use youtube::{PlaylistMetadata, VideoMetadata, YoutubeClient};

#[derive(Debug, Default, poise::ChoiceParameter)]
pub enum ResourceType {
    #[default]
    Track,
    Playlist,
}

#[derive(Debug, Clone)]
pub enum QueueElement {
    Track(VideoMetadata),
    Playlist(PlaylistMetadata),
}

pub trait CanHandleInput {
    fn url_check(input: &str) -> bool;
}
