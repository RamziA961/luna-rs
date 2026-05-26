use metrics::{counter, histogram};
use songbird::input::{AudioStream, Input, LiveInput, core::io::ReadOnlySource};
use std::process::{Command, Stdio};
use tokio::time::Instant;

use crate::metrics::{Metric, instruments::instrumented_reader::InstrumentedReader};

#[derive(thiserror::Error, Debug)]
pub enum StreamError {
    #[error("Failed to spawn subprocess {0}: {1}")]
    Spawn(String, std::io::Error),
    #[error("Failed to capture stdout from {0}")]
    Capture(String),
}

/// Spawns yt-dlp and pipes it into ffmpeg to deliver a stream to Songbird.
pub fn create_audio_stream(url: &str) -> Result<Input, StreamError> {
    let start_time = Instant::now();
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
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| {
            counter!(Metric::StreamCreationTotal.as_ref(), "status" => "error", "process" => "yt-dlp")
                .increment(1);
            StreamError::Spawn("yt-dlp".to_string(), e)
        })?;

    let ytdl_stdout = ytdl.stdout.take().ok_or_else(|| {
        counter!(Metric::StreamCreationTotal.as_ref(), "status" => "error", "process" => "yt-dlp_stdout")
            .increment(1);
        StreamError::Capture("yt-dlp".to_string())
    })?;

    // Spawn ffmpeg to transcode on-the-fly into raw/probe-friendly MP3 data
    let mut ffmpeg = Command::new("ffmpeg")
        .args(["-i", "pipe:0", "-c:a", "copy", "-f", "ogg", "-vn", "pipe:1"])
        .stdin(ytdl_stdout)
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| {
            counter!(Metric::StreamCreationTotal.as_ref(), "status" => "error", "process" => "ffmpeg")
                .increment(1);
            StreamError::Spawn("ffmpeg".to_string(), e)
        })?;

    let ffmpeg_stdout = ffmpeg.stdout.take().ok_or_else(|| {
        counter!(Metric::StreamCreationTotal.as_ref(), "status" => "error", "process" => "ffmpeg_stdout")
            .increment(1);
        StreamError::Capture("ffmpeg".to_string())
    })?;

    let instrumented_stdout = InstrumentedReader::new(ffmpeg_stdout, {
        let mut first_byte_recorded = false;
        move |bytes_read| {
            if !first_byte_recorded {
                histogram!(Metric::StreamStartupDuration.as_ref())
                    .record(start_time.elapsed().as_secs_f64());
                first_byte_recorded = true;
            }

            metrics::counter!(Metric::AudioBytesStreamedTotal.as_ref())
                .increment(bytes_read as u64);
        }
    });

    let media_source = ReadOnlySource::new(instrumented_stdout);
    let boxed_in: Box<dyn symphonia::core::io::MediaSource> = Box::new(media_source);

    let audio_stream = AudioStream { input: boxed_in };
    let raw_src = LiveInput::Raw(audio_stream);

    counter!(Metric::StreamCreationTotal.as_ref(), "status" => "success").increment(1);

    Ok(Input::Live(raw_src, None))
}
