use super::{YoutubeError, metadata_utils};
use google_youtube3::api::{PlaylistItem, SearchResult, Video};
use std::fmt::Display;
use tracing::{error, instrument, trace};

#[derive(Debug, Clone)]
pub struct VideoMetadata {
    pub id: String,
    pub title: String,
    pub channel: String,
    pub url: String,
    pub thumbnail_url: String,
}

impl Display for VideoMetadata {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Track: {} - {}", self.title, self.channel)
    }
}

impl TryFrom<&Video> for VideoMetadata {
    type Error = YoutubeError;

    #[instrument(skip_all)]
    fn try_from(value: &Video) -> Result<Self, Self::Error> {
        let snippet = value.snippet.as_ref().ok_or(YoutubeError::Conversion)?;

        if metadata_utils::is_live_stream(snippet.live_broadcast_content.as_ref()) {
            trace!("Live stream detected.");
            return Err(YoutubeError::Unsupported(
                "Live streams are not supported yet.".to_string(),
            ));
        }

        metadata_utils::assemble_metadata(
            value.id.as_deref(),
            snippet.title.as_deref(),
            snippet.channel_title.as_deref(),
            metadata_utils::extract_thumbnail(snippet.thumbnails.as_ref()),
        )
        .ok_or_else(|| {
            error!("Video to VideoMetadata conversion failed.");
            YoutubeError::Conversion
        })
    }
}

impl TryFrom<&SearchResult> for VideoMetadata {
    type Error = YoutubeError;

    #[instrument(skip_all)]
    fn try_from(value: &SearchResult) -> Result<Self, Self::Error> {
        let snippet = value.snippet.as_ref().ok_or(YoutubeError::Conversion)?;

        if metadata_utils::is_live_stream(snippet.live_broadcast_content.as_ref()) {
            trace!("Live stream detected.");
            return Err(YoutubeError::Unsupported(
                "Live streams are not supported yet.".to_string(),
            ));
        }

        let video_id = value.id.as_ref().and_then(|id| id.video_id.as_deref());

        metadata_utils::assemble_metadata(
            video_id,
            snippet.title.as_deref(),
            snippet.channel_title.as_deref(),
            metadata_utils::extract_thumbnail(snippet.thumbnails.as_ref()),
        )
        .ok_or_else(|| {
            error!("SearchResult to VideoMetadata conversion failed.");
            YoutubeError::Conversion
        })
    }
}

impl TryFrom<&PlaylistItem> for VideoMetadata {
    type Error = YoutubeError;

    #[instrument(skip_all)]
    fn try_from(value: &PlaylistItem) -> Result<Self, Self::Error> {
        let snippet = value.snippet.as_ref().ok_or(YoutubeError::Conversion)?;

        let video_id = snippet
            .resource_id
            .as_ref()
            .and_then(|res| res.video_id.as_deref());

        metadata_utils::assemble_metadata(
            video_id,
            snippet.title.as_deref(),
            snippet.video_owner_channel_title.as_deref(),
            metadata_utils::extract_thumbnail(snippet.thumbnails.as_ref()),
        )
        .ok_or_else(|| {
            error!("PlaylistItem to VideoMetadata conversion failed.");
            YoutubeError::Conversion
        })
    }
}
