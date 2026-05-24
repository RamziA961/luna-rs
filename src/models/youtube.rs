use google_youtube3::hyper_rustls;
use google_youtube3::{
    YouTube,
    common::NoToken,
    hyper_rustls::HttpsConnector,
    hyper_util::{
        client::legacy::{Client, connect::HttpConnector},
        rt::TokioExecutor,
    },
};
use std::fmt::Debug;
use tracing::{error, info, instrument, trace};

mod metadata_utils;
pub mod playlist_metadata;
pub mod video_metadata;

use playlist_metadata::PlaylistMetadata;
use video_metadata::VideoMetadata;

#[derive(Debug)]
pub enum YoutubeMetadata {
    Track(VideoMetadata),
    Playlist(PlaylistMetadata),
}

const SINGLE_URI: &str = "https://youtube.com/watch?v=";
const PLAYLIST_URI: &str = "https://youtube.com/playlist?list=";

const PLAYLIST_CAP: usize = 50;

#[derive(thiserror::Error, Debug)]
pub enum YoutubeError {
    #[error("Failed to extract video information from results.")]
    Conversion,

    #[error("Requested resource not found.")]
    NotFound,

    #[error("{0}")]
    Api(google_youtube3::Error),

    #[error("Invalid Youtube URL provided.")]
    Url,

    #[error("Unsupported Error: {0}")]
    Unsupported(String),
}

#[derive(Clone)]
pub struct YoutubeClient {
    api_key: String,
    client: YouTube<HttpsConnector<HttpConnector>>,
}

impl Debug for YoutubeClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("YoutubeClient")
            .field("client", &"<...>")
            .finish_non_exhaustive()
    }
}

impl YoutubeClient {
    pub async fn new(api_key: &str) -> Self {
        let client = Client::builder(TokioExecutor::new());
        let connector = hyper_rustls::HttpsConnectorBuilder::new()
            .with_native_roots()
            .unwrap()
            .https_or_http()
            .enable_http1()
            .build();

        let client = client.build(connector);
        let hub = YouTube::new(client, NoToken);

        Self {
            api_key: api_key.to_string(),
            client: hub,
        }
    }

    #[instrument]
    pub async fn process_url(&self, url: &str) -> Result<YoutubeMetadata, YoutubeError> {
        trace!("Processing URL.");
        let parsed_url = url::Url::parse(url).map_err(|e| {
            trace!(err=%e, "Could not parse URL.");
            YoutubeError::Url
        })?;

        // Attempt to extract a video ID first
        if let Some(id) = Self::extract_video_id(&parsed_url) {
            trace!(video_id=%id, "URL designated as video.");
            let metadata = self.get_video_metadata(&id).await?;
            trace!(metadata=%metadata, "Video metadata retrieved.");
            return Ok(YoutubeMetadata::Track(metadata));
        }

        // Fallback to attempting to extract a playlist ID
        if let Some(id) = Self::extract_playlist_id(&parsed_url) {
            trace!(playlist_id=%id, "URL designated as playlist.");
            let metadata = self.get_playlist_metadata(&id).await?;
            trace!(metadata=%metadata, "Playlist metadata retrieved.");
            return Ok(YoutubeMetadata::Playlist(metadata));
        }

        // If neither worked, the URL format is unsupported
        trace!(url=?parsed_url, "Reporting URL as error.");
        Err(YoutubeError::Url)
    }

    #[instrument(skip(self))]
    pub async fn search_video(&self, query: &str) -> Result<VideoMetadata, YoutubeError> {
        trace!("Searching for video");
        let (_, list) = self
            .client
            .search()
            .list(&vec!["snippet".to_string()])
            .q(query)
            .param("key", &self.api_key)
            .add_type("video")
            .max_results(1)
            .doit()
            .await
            .map_err(|e| {
                error!(err=%e, "Failed searching for video resource.");
                YoutubeError::Api(e)
            })?;

        let top_result = list
            .items
            .as_ref()
            .and_then(|items| items.first())
            .ok_or_else(|| {
                info!("Failed to find video resource with given search query.");
                YoutubeError::NotFound
            })?;

        VideoMetadata::try_from(top_result)
    }

    #[instrument(skip(self))]
    pub async fn search_playlist(&self, query: &str) -> Result<PlaylistMetadata, YoutubeError> {
        trace!("Searching for playlist");
        let (_, list) = self
            .client
            .search()
            .list(&vec!["snippet".to_string()])
            .q(query)
            .param("key", &self.api_key)
            .add_type("playlist")
            .max_results(1)
            .doit()
            .await
            .map_err(|e| {
                error!(err=%e, "Failed searching for playlist resource.");
                YoutubeError::Api(e)
            })?;

        let top_result = list
            .items
            .as_ref()
            .and_then(|items| items.first())
            .ok_or_else(|| {
                info!("Failed to find playlist matching given query");
                YoutubeError::NotFound
            })?;

        let mut metadata = PlaylistMetadata::try_from(top_result)?;
        let items = self.fetch_playlist_items(&metadata.id).await?;
        metadata.items.extend(items);

        Ok(metadata)
    }

    #[instrument(skip(self))]
    pub async fn get_video_metadata(&self, video_id: &str) -> Result<VideoMetadata, YoutubeError> {
        trace!("Requested video metadata");
        let (_, list) = self
            .client
            .videos()
            .list(&vec!["snippet".to_string()])
            .add_id(video_id)
            .param("key", &self.api_key)
            .doit()
            .await
            .map_err(|e| {
                error!(err=%e, "Error fetching resource.");
                YoutubeError::Api(e)
            })?;

        let video = list
            .items
            .as_ref()
            .and_then(|items| items.first())
            .ok_or_else(|| {
                error!("Failed to requested video resource by ID.");
                YoutubeError::NotFound
            })?;

        VideoMetadata::try_from(video)
    }

    #[instrument(skip(self))]
    pub async fn get_playlist_metadata(
        &self,
        playlist_id: &str,
    ) -> Result<PlaylistMetadata, YoutubeError> {
        trace!("Requested playlist metadata");
        let metadata_request = self
            .client
            .playlists()
            .list(&vec!["snippet".to_string()])
            .add_id(playlist_id)
            .param("key", &self.api_key)
            .max_results(1);

        // Run concurrently
        let (playlist_res, items_res) = tokio::join!(
            metadata_request.doit(),
            self.fetch_playlist_items(playlist_id)
        );

        // Map and clean up
        let (_, list) = playlist_res.map_err(|e| {
            error!(err=%e, "Error fetching playlist resource.");
            YoutubeError::Api(e)
        })?;

        let items = items_res?;

        let playlist = list
            .items
            .as_ref()
            .and_then(|items| items.first())
            .ok_or_else(|| {
                error!("Failed to requested playlist resource by ID.");
                YoutubeError::NotFound
            })?;

        let mut metadata = PlaylistMetadata::try_from(playlist)?;
        metadata.items.extend(items);
        Ok(metadata)
    }

    async fn fetch_playlist_items(
        &self,
        playlist_id: &str,
    ) -> Result<Vec<VideoMetadata>, YoutubeError> {
        let mut page_token: Option<String> = None;
        let mut playlist_items = Vec::new();

        loop {
            let mut request = self
                .client
                .playlist_items()
                .list(&vec!["snippet".to_string()])
                .playlist_id(playlist_id)
                .param("key", &self.api_key)
                .max_results(50);

            if let Some(ref token) = page_token {
                request = request.page_token(token);
            }

            let (_, response) = request.doit().await.map_err(YoutubeError::Api)?;

            if let Some(items) = response.items {
                let valid_metadata = items.into_iter().filter_map(|item| {
                    VideoMetadata::try_from(&item)
                        .map_err(|_| trace!("Skipped playlist item"))
                        .ok()
                });
                playlist_items.extend(valid_metadata);
            }

            let Some(next_token) = response.next_page_token else {
                break;
            };

            if playlist_items.len() >= PLAYLIST_CAP {
                break;
            }

            page_token = Some(next_token);
        }

        Ok(playlist_items)
    }

    /// Robustly extracts a YouTube video ID from various URL formats.
    fn extract_video_id(url: &url::Url) -> Option<String> {
        let domain = url.domain().unwrap_or("");

        if domain == "youtu.be" {
            return url
                .path_segments()?
                .next()
                .filter(|id| id.len() == 11)
                .map(String::from);
        }

        if domain != "youtube.com" && !domain.ends_with(".youtube.com") {
            return None;
        }

        if let Some((_, arg)) = url.query_pairs().find(|(q, _)| q == "v")
            && arg.len() == 11
        {
            return Some(arg.into_owned());
        }

        let mut segments = url.path_segments()?;
        let prefix = segments.next()?;

        if matches!(prefix, "shorts" | "embed" | "v" | "live") {
            return segments
                .next()
                .filter(|id| id.len() == 11)
                .map(String::from);
        }

        None
    }

    /// Extracts a playlist ID, supports Mixes and Albums.
    fn extract_playlist_id(url: &url::Url) -> Option<String> {
        let domain = url.domain().unwrap_or("");

        // Playlists are almost exclusively on the main domain or subdomains
        if domain == "youtube.com" || domain.ends_with(".youtube.com") {
            url.query_pairs()
                .find(|(q, _)| q == "list")
                .map(|(_, arg)| arg.into_owned())
        } else {
            None
        }
    }
}
