use google_youtube3::{
    client::NoToken,
    hyper_rustls::{self, HttpsConnector},
    YouTube,
};
use hyper::client::HttpConnector;
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

#[derive(thiserror::Error, Debug)]
pub enum YoutubeError {
    #[error("Failed to extract video information from results.")]
    ConversionError,

    #[error("Requested resource not found.")]
    NotFoundError,

    #[error("{0}")]
    ApiError(google_youtube3::Error),

    #[error("Invalid Youtube URL provided.")]
    UrlError,
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
    pub fn new(api_key: &str) -> Self {
        let connector = hyper_rustls::HttpsConnectorBuilder::new()
            .with_native_roots()
            .unwrap()
            .https_or_http()
            .enable_http1()
            .build();

        let client = hyper::Client::builder().build(connector);

        Self {
            api_key: api_key.to_string(),
            client: YouTube::new(client, NoToken),
        }
    }

    #[instrument]
    pub async fn process_url(&self, url: &str) -> Result<YoutubeMetadata, YoutubeError> {
        trace!("Processing URL.");
        let path = url::Url::parse(url).map_err(|e| {
            trace!(err=%e, "Could not parse URL.");
            YoutubeError::UrlError
        })?;

        match path.domain() {
            Some("www.youtube.com") if Self::validate_standard_url(&path) => {
                trace!("Standard URL matched.");
                if let Some(id) = Self::extract_track_id_from_standard_url(&path) {
                    trace!("URL designated as video.");
                    let metadata = self.get_video_metadata(&id).await?;
                    trace!(metadata=%metadata, "Video metadata retrieved.");
                    Ok(YoutubeMetadata::Track(metadata))
                } else if let Some(id) = Self::extract_playlist_id_from_standard_url(&path) {
                    trace!("URL designated as playlist.");
                    let metadata = self.get_playlist_metadata(&id).await?;
                    trace!(metadata=%metadata, "Playlist metadata retrieved.");
                    Ok(YoutubeMetadata::Playlist(metadata))
                } else {
                    trace!("Video and Playlist ID could be extracted from URL.");
                    Err(YoutubeError::UrlError)
                }
            }
            Some("www.youtu.be") if Self::validate_shareable_url(&path) => {
                trace!("Shareable URL matched.");
                if let Some(id) = Self::extract_track_id_from_shareable_url(&path) {
                    let metadata = self.get_video_metadata(&id).await?;
                    trace!(metadata=%metadata, "Video metadata retrieved.");
                    Ok(YoutubeMetadata::Track(metadata))
                } else {
                    trace!("Video could be extracted from URL.");
                    Err(YoutubeError::UrlError)
                }
            }
            _ => {
                trace!(url=?path, "Reporting URL as error.");
                Err(YoutubeError::UrlError)
            }
        }
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
                        YoutubeError::NotFoundError
                    })?;

                VideoMetadata::try_from(top_result)
            }
            Err(e) => {
                error!(err=%e, "Failed searching for video resource.");
                Err(YoutubeError::ApiError(e))
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
                        YoutubeError::NotFoundError
                    })?;

                let mut metadata = PlaylistMetadata::try_from(top_result)?;
                let items = self.fetch_playlist_items(&metadata.id).await?;
                metadata.items.extend(items.into_iter());

                Ok(metadata)
            }
            Err(e) => {
                error!(err=%e, "Failed searching for playlist resource.");
                Err(YoutubeError::ApiError(e))
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
                        YoutubeError::NotFoundError
                    })?;

                VideoMetadata::try_from(video)
            }
            Err(e) => {
                error!(err=%e, "Error fetching resource.");
                Err(YoutubeError::ApiError(e))
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
                        YoutubeError::NotFoundError
                    })?;

                let mut metadata = PlaylistMetadata::try_from(playlist)?;
                metadata.items.extend(items.into_iter());
                Ok(metadata)
            }
            (playlist_result, item_result) => {
                if let Err(e) = playlist_result {
                    error!(err=%e, "Error fetching playlist resource.");
                    Err(YoutubeError::ApiError(e))
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

        let (_, mut response_items) = create_request()
            .doit()
            .await
            .map_err(YoutubeError::ApiError)?;

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

            if let Some(pg_token) = next_page_token {
                (_, response_items) = create_request()
                    .page_token(pg_token)
                    .doit()
                    .await
                    .map_err(YoutubeError::ApiError)?
            } else {
                break;
            }
        }

        Ok(playlst_items)
    }

    fn extract_track_id_from_standard_url(url: &url::Url) -> Option<String> {
        url.query_pairs()
            .find(|(q, _)| q == "v")
            .and_then(|(_, arg)| {
                if arg.len() == 11 {
                    Some(arg.to_string())
                } else {
                    None
                }
            })
    }

    fn extract_track_id_from_shareable_url(url: &url::Url) -> Option<String> {
        // skip starting '/'
        let id = url.path().chars().skip(1).collect::<String>();
        if id.len() == 11 {
            Some(id)
        } else {
            None
        }
    }

    fn extract_playlist_id_from_standard_url(url: &url::Url) -> Option<String> {
        url.query_pairs()
            .find(|(q, _)| q == "list")
            .and_then(|(_, arg)| {
                let valid_prefix = arg.chars().take(2).collect::<String>() == "PL";

                if arg.len() == 34 && valid_prefix {
                    Some(arg.to_string())
                } else {
                    None
                }
            })
    }

    // Validate youtube.com style url
    fn validate_standard_url(url: &url::Url) -> bool {
        match url.path() {
            "/watch" => {
                // ignore playlist portion
                Self::extract_track_id_from_standard_url(url).is_some()
            }
            "/playlist" => Self::extract_playlist_id_from_standard_url(url).is_some(),
            _ => false,
        }
    }

    fn validate_shareable_url(url: &url::Url) -> bool {
        Self::extract_track_id_from_shareable_url(url).is_some()
    }
}
