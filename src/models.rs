mod guild_state;
mod playback_state;
mod queue_element;
mod youtube;

pub use guild_state::GuildState;
pub use playback_state::PlaybackState;
pub use queue_element::QueueElement;
pub use youtube::{PlaylistMetadata, VideoMetadata, YoutubeClient};

#[derive(Debug)]
pub enum YoutubeMetadata {
    Track(VideoMetadata),
    Playlist(PlaylistMetadata),
}
