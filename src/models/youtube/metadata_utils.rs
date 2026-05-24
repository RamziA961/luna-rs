use crate::models::{
    PlaylistMetadata, VideoMetadata,
    youtube::{PLAYLIST_URI, SINGLE_URI},
};
use google_youtube3::api::ThumbnailDetails;
use html_escape::decode_html_entities;
use std::collections::VecDeque;

/// Safely evaluates if the live broadcast state implies an active live stream.
pub fn is_live_stream(live_content: Option<&String>) -> bool {
    live_content.is_some_and(|status| status != "none")
}

/// Cascades down thumbnail options from highest resolution to lowest fallback.
pub fn extract_thumbnail(details: Option<&ThumbnailDetails>) -> Option<&str> {
    let d = details?;
    d.maxres
        .as_ref()
        .or(d.high.as_ref())
        .or(d.medium.as_ref())
        .or(d.standard.as_ref())
        .or(d.default.as_ref())
        .and_then(|t| t.url.as_deref())
}

/// Houses the shared core assembly logic for constructing VideoMetadata structures.
pub fn assemble_metadata(
    id: Option<&str>,
    title: Option<&str>,
    channel: Option<&str>,
    thumbnail_url: Option<&str>,
) -> Option<VideoMetadata> {
    let (id, title, channel, thumb) = (id?, title?, channel?, thumbnail_url?);

    Some(VideoMetadata {
        id: id.to_string(),
        title: decode_html_entities(title).to_string(),
        channel: decode_html_entities(channel).to_string(),
        url: format!("{SINGLE_URI}{id}"),
        thumbnail_url: thumb.to_string(),
    })
}

/// Core assembly constructor for PlaylistMetadata structures.
pub fn assemble_playlist_metadata(
    id: Option<&str>,
    title: Option<&str>,
    channel: Option<&str>,
    thumbnail_url: Option<&str>,
) -> Option<PlaylistMetadata> {
    let (id, title, channel, thumb) = (id?, title?, channel?, thumbnail_url?);

    Some(PlaylistMetadata {
        id: id.to_string(),
        title: decode_html_entities(title).to_string(),
        channel: decode_html_entities(channel).to_string(),
        url: format!("{PLAYLIST_URI}{id}"),
        thumbnail_url: thumb.to_string(),
        items: VecDeque::new(),
    })
}
