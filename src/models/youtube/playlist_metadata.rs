use super::{YoutubeError, metadata_utils, video_metadata::VideoMetadata};
use google_youtube3::api::{Playlist, SearchResult};
use std::{collections::VecDeque, fmt::Display};
use tracing::{error, instrument};

#[derive(Debug, Clone)]
pub struct PlaylistMetadata {
    pub id: String,
    pub title: String,
    pub channel: String,
    pub url: String,
    pub thumbnail_url: String,
    pub items: VecDeque<VideoMetadata>,
}

impl Display for PlaylistMetadata {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Playlist: {} - {} with {} remaining tracks",
            self.title,
            self.channel,
            self.items.len()
        )
    }
}

impl TryFrom<&Playlist> for PlaylistMetadata {
    type Error = YoutubeError;

    #[instrument(skip_all)]
    fn try_from(value: &Playlist) -> Result<Self, Self::Error> {
        let snippet = value.snippet.as_ref().ok_or(YoutubeError::Conversion)?;

        metadata_utils::assemble_playlist_metadata(
            value.id.as_deref(),
            snippet.title.as_deref(),
            snippet.channel_title.as_deref(),
            metadata_utils::extract_thumbnail(snippet.thumbnails.as_ref()),
        )
        .ok_or_else(|| {
            error!("Playlist to PlaylistMetadata conversion failed.");
            YoutubeError::Conversion
        })
    }
}

impl TryFrom<&SearchResult> for PlaylistMetadata {
    type Error = YoutubeError;

    #[instrument(skip_all)]
    fn try_from(value: &SearchResult) -> Result<Self, Self::Error> {
        let snippet = value.snippet.as_ref().ok_or(YoutubeError::Conversion)?;

        let playlist_id = value.id.as_ref().and_then(|id| id.playlist_id.as_deref());

        metadata_utils::assemble_playlist_metadata(
            playlist_id,
            snippet.title.as_deref(),
            snippet.channel_title.as_deref(),
            metadata_utils::extract_thumbnail(snippet.thumbnails.as_ref()),
        )
        .ok_or_else(|| {
            error!("SearchResult to PlaylistMetadata conversion failed.");
            YoutubeError::Conversion
        })
    }
}
