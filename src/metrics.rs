pub mod instruments;

#[derive(Debug, Clone, Copy, strum::AsRefStr, strum::Display)]
#[strum(prefix = "luna_", serialize_all = "snake_case")]
pub enum Metric {
    CommandsInvokedTotal,
    CommandsCompletedTotal,
    CommandErrorsTotal,
    EventHandlerErrorsTotal,
    SystemErrorsTotal,

    // Stream related metrics (ffmpeg, yt-dlp)
    StreamCreationTotal,
    #[strum(serialize = "stream_startup_duration_seconds")]
    StreamStartupDuration,
    AudioBytesStreamedTotal,
}
