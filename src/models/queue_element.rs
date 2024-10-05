use std::fmt::Display;

use super::{PlaylistMetadata, VideoMetadata, YoutubeMetadata};

#[derive(Debug, Clone)]
pub enum QueueElement {
    Track(VideoMetadata),
    Playlist(PlaylistMetadata),
}

impl Display for QueueElement {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            QueueElement::Track(t) => {
                write!(f, "Track {} - {}", t.title, t.channel)
            }
            QueueElement::Playlist(p) => {
                write!(
                    f,
                    "Playlist {} - {} with {} tracks",
                    p.title,
                    p.channel,
                    p.items.len()
                )
            }
        }
    }
}

impl From<YoutubeMetadata> for QueueElement {
    fn from(value: YoutubeMetadata) -> Self {
        match value {
            YoutubeMetadata::Track(t) => Self::Track(t),
            YoutubeMetadata::Playlist(p) => Self::Playlist(p),
        }
    }
}
