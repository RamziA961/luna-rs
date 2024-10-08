use super::{YoutubeError, SINGLE_URI};
use google_youtube3::api::{PlaylistItem, SearchResult, Video};
use html_escape::decode_html_entities as decode_html;
use std::fmt::Display;
use tracing::{error, instrument};

#[derive(Debug, Clone)]
pub struct VideoMetadata {
    pub id: String,
    pub title: String,
    pub channel: String,
    pub url: String,
}

impl Display for VideoMetadata {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Track: {} - {}", self.title, self.channel)
    }
}

impl TryFrom<&Video> for VideoMetadata {
    type Error = YoutubeError;

    #[instrument]
    fn try_from(value: &Video) -> Result<Self, Self::Error> {
        let id = &value.id;

        let metadata = value.snippet.as_ref().and_then(|snippet| {
            let title = &snippet.title;
            let channel = &snippet.channel_title;

            match (id, title, channel) {
                (Some(id), Some(t), Some(c)) => Some(VideoMetadata {
                    id: id.clone(),
                    title: decode_html(t).to_string(),
                    channel: decode_html(c).to_string(),
                    url: format!("{SINGLE_URI}{id}"),
                }),
                _ => None,
            }
        });

        metadata.ok_or_else(|| {
            error!("Video to VideoMetadata conversion failed.");
            YoutubeError::ConversionError
        })
    }
}

impl TryFrom<&SearchResult> for VideoMetadata {
    type Error = YoutubeError;

    #[instrument]
    fn try_from(value: &SearchResult) -> Result<Self, Self::Error> {
        let id = &value
            .id
            .as_ref()
            .and_then(|resource_id| resource_id.video_id.clone());

        let metadata = value.snippet.as_ref().and_then(|snippet| {
            let title = &snippet.title;
            let channel = &snippet.channel_title;

            match (id, title, channel) {
                (Some(id), Some(t), Some(c)) => Some(VideoMetadata {
                    id: id.clone(),
                    title: decode_html(t).to_string(),
                    channel: decode_html(c).to_string(),
                    url: format!("{SINGLE_URI}{id}"),
                }),
                _ => None,
            }
        });

        metadata.ok_or_else(|| {
            error!("SearchResult to VideoMetadata conversion failed.");
            YoutubeError::ConversionError
        })
    }
}

impl TryFrom<&PlaylistItem> for VideoMetadata {
    type Error = YoutubeError;

    #[instrument]
    fn try_from(value: &PlaylistItem) -> Result<Self, Self::Error> {
        let metadata = value.snippet.as_ref().and_then(|snippet| {
            let id = &snippet
                .resource_id
                .as_ref()
                .and_then(|resource_id| resource_id.video_id.clone());
            let title = &snippet.title;
            let channel = &snippet.channel_title;

            match (id, title, channel) {
                (Some(id), Some(t), Some(c)) => Some(VideoMetadata {
                    id: id.clone(),
                    title: decode_html(t).to_string(),
                    channel: decode_html(c).to_string(),
                    url: format!("{}{}", SINGLE_URI, id),
                }),
                _ => None,
            }
        });

        metadata.ok_or_else(|| {
            error!("PlaylistItem to VideoMetadata conversion failed.");
            YoutubeError::ConversionError
        })
    }
}
