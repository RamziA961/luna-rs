mod guild_state;
mod playback_state;
mod queue_element;

mod youtube;

pub use guild_state::GuildState;
pub use playback_state::PlaybackState;
pub use queue_element::QueueElement;

pub use youtube::{
    YoutubeClient, YoutubeMetadata, playlist_metadata::PlaylistMetadata,
    video_metadata::VideoMetadata,
};

use crate::{models::youtube::YoutubeError, stream::StreamError};

#[derive(thiserror::Error, Debug)]
pub enum LunaError {
    #[error("Initialization Failure. {0}")]
    Initialization(#[from] std::io::Error),
    #[error("Runtime Error. {0}")]
    Runtime(#[from] Box<RuntimeError>),
}

#[derive(thiserror::Error, Debug)]
pub enum RuntimeError {
    #[error("Internal error occurred. {0}")]
    Internal(#[from] InternalError),

    #[error("Discord error occurred. {0}")]
    Discord(#[from] DiscordError),

    #[error("Youtube error occurred. {0}")]
    Youtube(#[from] YoutubeError),

    #[error("{0}")]
    User(String),

    #[error("Sorry this feature is planned but not implemented yet.")]
    Unimplemented,
}

#[derive(thiserror::Error, Debug)]
pub enum DiscordError {
    #[error("Gateway error: {0}")]
    Gateway(#[from] poise::serenity_prelude::Error),

    #[error("Voice Channel Join Error: {0}")]
    Join(#[from] songbird::error::JoinError),
}

#[derive(thiserror::Error, Debug)]
pub enum InternalError {
    #[error("Guild information missing.")]
    GuildInformationMissing,

    #[error("Unable to locate voice channel.")]
    VoiceChannelMissing,

    #[error("Bad guild state.")]
    BadGuildState,

    #[error("Missing dependency: {0}")]
    DependencyMissing(String),

    #[error("Streaming error: {0}")]
    Stream(#[from] StreamError),
}
