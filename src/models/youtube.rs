use google_youtube3::{
    api::{Playlist, SearchResult, Video},
    client::NoToken,
    hyper_rustls::{self, HttpsConnector},
    YouTube,
};
use hyper::client::HttpConnector;
use std::{collections::VecDeque, fmt::Debug};
use tracing::{error, info, instrument, trace};
use super::YoutubeMetadata;

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
        
        trace!(domain=?path.domain());
        match path.domain() {
            Some("www.youtube.com") if Self::validate_standard_url(&path) => {
                trace!("Standard URL matched.");
                if let Some(id) = Self::extract_track_id_from_standard_url(&path) {
                    trace!("URL designated as video.");
                    let metadata = self.get_video_metadata(&id).await?;
                    trace!(metadata=?metadata, "Video metadata retrieved.");
                    Ok(YoutubeMetadata::Track(metadata))
                } else if let Some(id) = Self::extract_playlist_id_from_standard_url(&path) {
                    trace!("URL designated as playlist.");
                    let metadata = self.get_playlist_metadata(&id).await?;
                    trace!(metadata=?metadata, "Playlist metadata retrieved.");
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
                    trace!(metadata=?metadata, "Video metadata retrieved.");
                    Ok(YoutubeMetadata::Track(metadata))
                } else {
                    trace!("Video could be extracted from URL.");
                    Err(YoutubeError::UrlError)
                }
            }
            _ => {
                trace!(url=?path, "Reporting URL as error.");
                Err(YoutubeError::UrlError)
            },
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

                PlaylistMetadata::try_from(top_result)
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
        trace!("Requested video metadata");
        let request = self
            .client
            .playlists()
            .list(&vec!["snippet".to_string()])
            .add_id(&playlist_id)
            .param("key", &self.api_key)
            .max_results(1);

        let response = request.doit().await;

        match response {
            Ok((_, list)) => {
                let playlist = list
                    .items
                    .as_ref()
                    .and_then(|items| items.first())
                    .ok_or_else(|| {
                        error!("Failed to requested playlist resource by ID.");
                        YoutubeError::NotFoundError
                    })?;

                PlaylistMetadata::try_from(playlist)
            }
            Err(e) => {
                error!(err=%e, "Error fetching playlist resource.");
                Err(YoutubeError::ApiError(e))
            }
        }
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
                Self::extract_track_id_from_standard_url(&url).is_some()
            }
            "/playlist" => Self::extract_playlist_id_from_standard_url(&url).is_some(),
            _ => false,
        }
    }

    fn validate_shareable_url(url: &url::Url) -> bool {
        Self::extract_track_id_from_shareable_url(url).is_some()
    }
}

//impl CanHandleInput for YoutubeClient {
//    fn url_check(input: &str) -> bool {
//        if let Ok(url) = url::Url::parse(input) {
//            url.domain().is_some_and(|domain| match domain {
//                "youtube.com" => Self::validate_standard_url(&url),
//                "youtu.be" => Self::validate_shareable_url(&url),
//                _ => false,
//            })
//        } else {
//            false
//        }
//    }
//}

#[derive(Debug, Clone)]
pub struct VideoMetadata {
    pub id: String,
    pub title: String,
    pub channel: String,
    pub url: String,
}

impl TryFrom<&Video> for VideoMetadata {
    type Error = YoutubeError;

    fn try_from(value: &Video) -> Result<Self, Self::Error> {
        let ref id = value.id;

        let metadata = value
            .snippet
            .as_ref()
            .map(|snippet| {
                let ref title = snippet.title;
                let ref channel = snippet.channel_title;

                match (id, title, channel) {
                    (Some(id), Some(t), Some(c)) => Some(VideoMetadata {
                        id: id.clone(),
                        title: t.clone(),
                        channel: c.clone(),
                        url: format!("{SINGLE_URI}{id}"),
                    }),
                    _ => None,
                }
            })
            .flatten();

        metadata.ok_or_else(|| {
            error!("Video to VideoMetadata conversion failed.");
            YoutubeError::ConversionError
        })
    }
}

impl TryFrom<&SearchResult> for VideoMetadata {
    type Error = YoutubeError;

    fn try_from(value: &SearchResult) -> Result<Self, Self::Error> {
        let ref id = value
            .id
            .as_ref()
            .and_then(|resource_id| resource_id.video_id.clone());

        let metadata = value
            .snippet
            .as_ref()
            .map(|snippet| {
                let ref title = snippet.title;
                let ref channel = snippet.channel_title;

                match (id, title, channel) {
                    (Some(id), Some(t), Some(c)) => Some(VideoMetadata {
                        id: id.clone(),
                        title: t.clone(),
                        channel: c.clone(),
                        url: format!("{SINGLE_URI}{id}"),
                    }),
                    _ => None,
                }
            })
            .flatten();

        metadata.ok_or_else(|| {
            error!("SearchResult to VideoMetadata conversion failed.");
            YoutubeError::ConversionError
        })
    }
}

#[derive(Debug, Clone)]
pub struct PlaylistMetadata {
    pub id: String,
    pub title: String,
    pub channel: String,
    pub url: String,
    pub items: VecDeque<VideoMetadata>,
}

impl TryFrom<&Playlist> for PlaylistMetadata {
    type Error = YoutubeError;

    fn try_from(value: &Playlist) -> Result<Self, Self::Error> {
        let ref id = value.id;

        let metadata = value
            .snippet
            .as_ref()
            .map(|snippet| {
                let ref title = snippet.title;
                let ref channel = snippet.channel_title;

                match (id, title, channel) {
                    (Some(id), Some(t), Some(c)) => Some(PlaylistMetadata {
                        id: id.clone(),
                        title: t.clone(),
                        channel: c.clone(),
                        url: format!("{PLAYLIST_URI}{id}"),
                        items: VecDeque::new(),
                    }),
                    _ => None,
                }
            })
            .flatten();

        metadata.ok_or_else(|| {
            error!("Playlist to PlaylistMetadata conversion failed.");
            YoutubeError::ConversionError
        })
    }
}

impl TryFrom<&SearchResult> for PlaylistMetadata {
    type Error = YoutubeError;

    fn try_from(value: &SearchResult) -> Result<Self, Self::Error> {
        let ref id = value
            .id
            .as_ref()
            .and_then(|resource_id| resource_id.playlist_id.clone());

        let metadata = value
            .snippet
            .as_ref()
            .map(|snippet| {
                let ref title = snippet.title;
                let ref channel = snippet.channel_title;

                match (id, title, channel) {
                    (Some(id), Some(t), Some(c)) => Some(PlaylistMetadata {
                        id: id.clone(),
                        title: t.clone(),
                        channel: c.clone(),
                        url: format!("{PLAYLIST_URI}{id}"),
                        items: VecDeque::new(),
                    }),
                    _ => None,
                }
            })
            .flatten();

        metadata.ok_or_else(|| {
            error!("SearchResult to PlaylistMetadata conversion failed.");
            YoutubeError::ConversionError
        })
    }
}
