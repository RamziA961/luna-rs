use std::{collections::VecDeque, fmt::Display};
use google_youtube3::api::{Playlist, SearchResult};
use tracing::{error, instrument};
use super::{video_metadata::VideoMetadata, PLAYLIST_URI, YoutubeError};
use html_escape::decode_html_entities as decode_html;

#[derive(Debug, Clone)]
pub struct PlaylistMetadata {
    pub id: String,
    pub title: String,
    pub channel: String,
    pub url: String,
    pub items: VecDeque<VideoMetadata>,
}

impl Display for PlaylistMetadata {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "PlaylistMetadata {{ id: {}, title: {}, channel: {}, url: {}, items: {} elements }}",
            self.id,
            self.title,
            self.channel,
            self.url,
            self.items.len()
        )
    }
}

impl TryFrom<&Playlist> for PlaylistMetadata {
    type Error = YoutubeError;
    
    #[instrument]
    fn try_from(value: &Playlist) -> Result<Self, Self::Error> {
        let ref id = value.id;

        let metadata = value.snippet.as_ref().and_then(|snippet| {
            let ref title = snippet.title;
            let ref channel = snippet.channel_title;

            match (id, title, channel) {
                (Some(id), Some(t), Some(c)) => Some(PlaylistMetadata {
                    id: id.clone(),
                    title: decode_html(t).to_string(),
                    channel: decode_html(c).to_string(),
                    url: format!("{PLAYLIST_URI}{id}"),
                    items: VecDeque::new(),
                }),
                _ => None,
            }
        });

        metadata.ok_or_else(|| {
            error!("Playlist to PlaylistMetadata conversion failed.");
            YoutubeError::ConversionError
        })
    }
}

impl TryFrom<&SearchResult> for PlaylistMetadata {
    type Error = YoutubeError;
    
    #[instrument]
    fn try_from(value: &SearchResult) -> Result<Self, Self::Error> {
        let ref id = value
            .id
            .as_ref()
            .and_then(|resource_id| resource_id.playlist_id.clone());

        let metadata = value.snippet.as_ref().and_then(|snippet| {
            let ref title = snippet.title;
            let ref channel = snippet.channel_title;

            match (id, title, channel) {
                (Some(id), Some(t), Some(c)) => Some(PlaylistMetadata {
                    id: id.clone(),
                    title: decode_html(t).to_string(),
                    channel: decode_html(c).to_string(),
                    url: format!("{PLAYLIST_URI}{id}"),
                    items: VecDeque::new(),
                }),
                _ => None,
            }
        });

        metadata.ok_or_else(|| {
            error!("SearchResult to PlaylistMetadata conversion failed.");
            YoutubeError::ConversionError
        })
    }
}
