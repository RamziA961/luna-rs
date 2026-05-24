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
        write!(f, "YoutubeClient {{ client: <...> }}")
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

    #[instrument]
    pub async fn search_video(&self, query: &str) -> Result<VideoMetadata, YoutubeError> {
        trace!("Searching for video");
        let request = self
            .client
            .search()
            .list(&vec!["snippet".to_string()])
            .q(query)
            .param("key", &self.api_key)
            .add_type("video")
            .max_results(1);

        let response = request.doit().await;

        match response {
            Ok((_, list)) => {
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
            Err(e) => {
                error!(err=%e, "Failed searching for video resource.");
                Err(YoutubeError::Api(e))
            }
        }
    }

    #[instrument]
    pub async fn search_playlist(&self, query: &str) -> Result<PlaylistMetadata, YoutubeError> {
        trace!("Searching for playlist");
        let request = self
            .client
            .search()
            .list(&vec!["snippet".to_string()])
            .q(query)
            .param("key", &self.api_key)
            .add_type("playlist")
            .max_results(1);

        let response = request.doit().await;

        match response {
            Ok((_, list)) => {
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
                metadata.items.extend(items.into_iter());

                Ok(metadata)
            }
            Err(e) => {
                error!(err=%e, "Failed searching for playlist resource.");
                Err(YoutubeError::Api(e))
            }
        }
    }

    #[instrument]
    pub async fn get_video_metadata(&self, video_id: &str) -> Result<VideoMetadata, YoutubeError> {
        trace!("Requested video metadata");
        let request = self
            .client
            .videos()
            .list(&vec!["snippet".to_string()])
            .add_id(video_id)
            .param("key", &self.api_key);
        let response = request.doit().await;

        match response {
            Ok((_, list)) => {
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
            Err(e) => {
                error!(err=%e, "Error fetching resource.");
                Err(YoutubeError::Api(e))
            }
        }
    }

    #[instrument]
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

        let (playlist_response, items_response) = tokio::join!(
            metadata_request.doit(),
            self.fetch_playlist_items(playlist_id)
        );

        match (playlist_response, items_response) {
            (Ok((_, list)), Ok(items)) => {
                let playlist = list
                    .items
                    .as_ref()
                    .and_then(|items| items.first())
                    .ok_or_else(|| {
                        error!("Failed to requested playlist resource by ID.");
                        YoutubeError::NotFound
                    })?;

                let mut metadata = PlaylistMetadata::try_from(playlist)?;
                metadata.items.extend(items.into_iter());
                Ok(metadata)
            }
            (playlist_result, item_result) => {
                if let Err(e) = playlist_result {
                    error!(err=%e, "Error fetching playlist resource.");
                    Err(YoutubeError::Api(e))
                } else if let Err(e) = item_result {
                    Err(e)
                } else {
                    unreachable!("One or both of the above is an error.")
                }
            }
        }
    }

    async fn fetch_playlist_items(
        &self,
        playlist_id: &str,
    ) -> Result<Vec<VideoMetadata>, YoutubeError> {
        let create_request = || {
            self.client
                .playlist_items()
                .list(&vec!["snippet".to_string()])
                .playlist_id(playlist_id)
                .param("key", &self.api_key)
                .max_results(50)
        };

        let (_, mut response_items) = create_request().doit().await.map_err(YoutubeError::Api)?;

        let mut playlst_items = vec![];

        loop {
            let next_page_token = &response_items.next_page_token;

            if let Some(items) = &response_items.items {
                for item in items {
                    let metadata = VideoMetadata::try_from(item);

                    if let Ok(metadata) = metadata {
                        playlst_items.push(metadata);
                    } else {
                        trace!("Skipped playlist item");
                    }
                }
            }

            match next_page_token {
                Some(pg_token) if playlst_items.len() < PLAYLIST_CAP => {
                    (_, response_items) = create_request()
                        .page_token(pg_token)
                        .doit()
                        .await
                        .map_err(YoutubeError::Api)?;
                }
                _ => break,
            }
        }

        Ok(playlst_items)
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
