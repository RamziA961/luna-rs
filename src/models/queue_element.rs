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
                write!(f, "Track: {} - {}\nURL: {}", t.title, t.channel, t.url)
            }
            QueueElement::Playlist(p) => {
                let head = 
                    p.items
                        .front()
                        .map(|t| format!("Up next: {} - {}\n", t.title, t.channel))
                        .unwrap_or("".to_string());

                write!(
                    f,
                    "Playlist: {} - {} with {} tracks remaining\n{head}URL: {}",
                    p.title,
                    p.channel,
                    p.items.len(),
                    p.url
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
