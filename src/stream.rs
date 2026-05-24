use songbird::input::{core::io::ReadOnlySource, AudioStream, Input, LiveInput};
use std::process::{Command, Stdio};

#[derive(thiserror::Error, Debug)]
pub enum StreamError {
    #[error("Failed to spawn subprocess {0}: {1}")]
    Spawn(String, std::io::Error),
    #[error("Failed to capture stdout from {0}")]
    Capture(String),
}

/// Spawns yt-dlp and pipes it into ffmpeg to deliver a clean MP3 stream to Songbird.
pub fn create_audio_stream(url: &str) -> Result<Input, StreamError> {
    // Spawn yt-dlp to download the raw audio stream
    let mut ytdl = Command::new("yt-dlp")
        .args([
            "-f",
            "251/bestaudio",
            "-o",
            "-", // Stream to stdout
            url,
        ])
        .stdout(Stdio::piped())
        .spawn()
        .map_err(|e| StreamError::Spawn("yt-dlp".to_string(), e))?;

    let ytdl_stdout = ytdl
        .stdout
        .take()
        .ok_or_else(|| StreamError::Capture("yt-dlp".to_string()))?;

    // Spawn ffmpeg to transcode on-the-fly into raw/probe-friendly MP3 data
    let mut ffmpeg = Command::new("ffmpeg")
        .args([
            "-i", "pipe:0", // Read from yt-dlp's stdout
            "-f", "mp3", // Probe proof format
            "-ac", "2", // Stereo
            "-ar", "48000",  // Discord audio standard frequency
            "-vn",    // Explicitly skip video decoding
            "pipe:1", // Write to stdout
        ])
        .stdin(ytdl_stdout)
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| StreamError::Spawn("ffmpeg".to_string(), e))?;

    let ffmpeg_stdout = ffmpeg
        .stdout
        .take()
        .ok_or_else(|| StreamError::Capture("ffmpeg".to_string()))?;

    let media_source = ReadOnlySource::new(ffmpeg_stdout);
    let boxed_in: Box<dyn symphonia::core::io::MediaSource> = Box::new(media_source);

    let audio_stream = AudioStream { input: boxed_in };
    let raw_src = LiveInput::Raw(audio_stream);

    Ok(Input::Live(raw_src, None))
}
