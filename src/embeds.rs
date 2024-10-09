use poise::serenity_prelude::{self, Color, CreateEmbed, Timestamp};

use crate::models::{PlaylistMetadata, VideoMetadata};

fn create_embed_template() -> serenity_prelude::CreateEmbed {
    serenity_prelude::CreateEmbed::new()
        .color(Color::DARK_ORANGE)
        .timestamp(Timestamp::now())
}

fn create_playlist_embed(playlist: &PlaylistMetadata) -> serenity_prelude::CreateEmbed {
    create_embed_template()
        .title(format!(
            "{} - {}",
            playlist.title.to_string(),
            playlist.channel.to_string()
        ))
        .url(playlist.url.to_string())
        .thumbnail(playlist.thumbnail_url.to_string())
}

fn create_track_embed(track: &VideoMetadata) -> serenity_prelude::CreateEmbed {
    create_embed_template()
        .title(format!(
            "{} - {}",
            track.title.to_string(),
            track.channel.to_string()
        ))
        .url(track.url.clone())
        .thumbnail(track.thumbnail_url.to_string())
}

pub fn create_queued_playlist_embed(playlist: &PlaylistMetadata) -> serenity_prelude::CreateEmbed {
    create_playlist_embed(playlist)
        .description("Queued the playlist.")
        .field("Tracks:", playlist.items.len().to_string(), true)
}

pub fn create_playling_playlist_embed(
    playlist: &PlaylistMetadata,
) -> serenity_prelude::CreateEmbed {
    let mut embed =
        create_playlist_embed(playlist).description("Playing the next track off the playlist.");

    if let Some(first) = playlist.items.front() {
        embed = embed.field(
            "Up next:",
            format!(
                "{} - {}\n{}",
                first.title.to_string(),
                first.channel.to_string(),
                first.url.to_string(),
            ),
            false,
        )
    }

    embed.field("Tracks:", playlist.items.len().to_string(), true)
}

pub fn create_queued_track_embed(track: &VideoMetadata) -> serenity_prelude::CreateEmbed {
    create_track_embed(track).description("Queued the track.")
}

pub fn create_playing_track_embed(track: &VideoMetadata) -> serenity_prelude::CreateEmbed {
    create_track_embed(track).description("Playing the next track off of the queue.")
}

pub fn create_resume_track_embed(track: &VideoMetadata) -> serenity_prelude::CreateEmbed {
    create_track_embed(track).description("Resumed track.")
}

pub fn create_skip_track_embed(
    track: &VideoMetadata,
    skipped: usize,
    remaining: usize,
) -> serenity_prelude::CreateEmbed {
    create_track_embed(track).description(format!(
        "Skipped {skipped} track(s). Tracks in the queue: {remaining}"
    ))
}

pub fn create_skip_playlist_embed(
    playlist: &PlaylistMetadata,
    skipped: usize,
    remaining: usize,
) -> serenity_prelude::CreateEmbed {
    create_playling_playlist_embed(playlist).description(format!(
        "Skipped {skipped} track(s). Tracks in the queue: {remaining}"
    ))
}

pub fn create_paused_embed(track: &VideoMetadata) -> serenity_prelude::CreateEmbed {
    create_track_embed(track).description("Paused playback.")
}
