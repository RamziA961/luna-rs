use super::{video_metadata::VideoMetadata, YoutubeError, PLAYLIST_URI};
use google_youtube3::api::{Playlist, SearchResult};
use html_escape::decode_html_entities as decode_html;
use std::{collections::VecDeque, fmt::Display};
use tracing::{error, instrument, trace};

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

    #[instrument]
    fn try_from(value: &Playlist) -> Result<Self, Self::Error> {
        let id = &value.id;

        let metadata = value.snippet.as_ref().and_then(|snippet| {
            let title = &snippet.title;
            let channel = &snippet.channel_title;
            let thumbnail_url = &snippet
                .thumbnails
                .as_ref()
                .and_then(|details| {
                    details
                        .maxres
                        .as_ref()
                        .or(details.high.as_ref())
                        .or(details.medium.as_ref())
                        .or(details.standard.as_ref())
                        .or(details.default.as_ref())
                })
                .and_then(|thumbnail| thumbnail.url.as_ref());

            trace!(title = title, channel = channel, thumbnail = thumbnail_url);
            match (id, title, channel, thumbnail_url) {
                (Some(id), Some(t), Some(c), Some(thumb)) => Some(PlaylistMetadata {
                    id: id.clone(),
                    title: decode_html(t).to_string(),
                    channel: decode_html(c).to_string(),
                    url: format!("{PLAYLIST_URI}{id}"),
                    thumbnail_url: thumb.to_string(),
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
        let id = &value
            .id
            .as_ref()
            .and_then(|resource_id| resource_id.playlist_id.clone());

        let metadata = value.snippet.as_ref().and_then(|snippet| {
            let title = &snippet.title;
            let channel = &snippet.channel_title;
            let thumbnail_url = &snippet
                .thumbnails
                .as_ref()
                .and_then(|details| {
                    details
                        .maxres
                        .as_ref()
                        .or(details.high.as_ref())
                        .or(details.medium.as_ref())
                        .or(details.standard.as_ref())
                        .or(details.default.as_ref())
                })
                .and_then(|thumbnail| thumbnail.url.as_ref());

            trace!(title = title, channel = channel, thumbnail = thumbnail_url);
            match (id, title, channel, thumbnail_url) {
                (Some(id), Some(t), Some(c), Some(thumb)) => Some(PlaylistMetadata {
                    id: id.clone(),
                    title: decode_html(t).to_string(),
                    channel: decode_html(c).to_string(),
                    url: format!("{PLAYLIST_URI}{id}"),
                    thumbnail_url: thumb.to_string(),
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
