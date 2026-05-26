use poise::serenity_prelude::{self, Color, Timestamp};

use crate::models::{PlaylistMetadata, QueueElement, VideoMetadata};

/// Base template with color and timestamp
fn create_embed_template() -> serenity_prelude::CreateEmbed {
    serenity_prelude::CreateEmbed::new()
        .color(Color::DARK_ORANGE)
        .timestamp(Timestamp::now())
}

/// Helper to consistently format track data across all embeds
fn populate_track_info(
    embed: serenity_prelude::CreateEmbed,
    track: &VideoMetadata,
) -> serenity_prelude::CreateEmbed {
    embed
        .field("Track", format!("[{}]({})", track.title, track.url), false)
        .field("Channel", &track.channel, true)
        .thumbnail(track.thumbnail_url.to_string())
}

/// Helper to consistently format playlist data across all embeds
fn populate_playlist_info(
    embed: serenity_prelude::CreateEmbed,
    playlist: &PlaylistMetadata,
) -> serenity_prelude::CreateEmbed {
    embed
        .field(
            "Playlist",
            format!("[{}]({})", playlist.title, playlist.url),
            false,
        )
        .field("Channel", &playlist.channel, true)
        .thumbnail(playlist.thumbnail_url.to_string())
}

/// Use for general info or status updates
pub fn create_info_embed(title: &str, message: &str) -> serenity_prelude::CreateEmbed {
    create_embed_template().title(title).description(message)
}

/// Use for success confirmations
pub fn create_success_embed(title: &str, message: &str) -> serenity_prelude::CreateEmbed {
    create_embed_template()
        .color(Color::DARK_GREEN)
        .title(title)
        .description(message)
}

/// Use for errors or failed operations
pub fn create_error_embed(message: &str) -> serenity_prelude::CreateEmbed {
    create_embed_template()
        .color(Color::RED) // Immediate visual cue for an issue
        .title("Error")
        .description(message)
}

// --- Playlist Embeds ---

pub fn create_queued_playlist_embed(playlist: &PlaylistMetadata) -> serenity_prelude::CreateEmbed {
    let embed = create_embed_template().title("Playlist Queued");
    populate_playlist_info(embed, playlist).field(
        "Total Tracks",
        playlist.items.len().to_string(),
        true,
    )
}

pub fn create_playing_playlist_embed(playlist: &PlaylistMetadata) -> serenity_prelude::CreateEmbed {
    let mut embed = create_embed_template().title("Now Playing Playlist");
    embed = populate_playlist_info(embed, playlist).field(
        "Total Tracks",
        playlist.items.len().to_string(),
        true,
    );

    if let Some(first) = playlist.items.front() {
        embed = embed.field(
            "Up Next",
            format!("[{}]({})", first.title, first.url),
            false,
        );
    }
    embed
}

// --- Track Embeds ---

pub fn create_queued_track_embed(track: &VideoMetadata) -> serenity_prelude::CreateEmbed {
    let embed = create_embed_template().title("Track Queued");
    populate_track_info(embed, track)
}

pub fn create_playing_track_embed(track: &VideoMetadata) -> serenity_prelude::CreateEmbed {
    let embed = create_embed_template().title("Now Playing");
    populate_track_info(embed, track)
}

pub fn create_resume_track_embed(track: &VideoMetadata) -> serenity_prelude::CreateEmbed {
    let embed = create_embed_template().title("Playback Resumed");
    populate_track_info(embed, track)
}

pub fn create_paused_embed(track: &VideoMetadata) -> serenity_prelude::CreateEmbed {
    let embed = create_embed_template().title("Playback Paused");
    populate_track_info(embed, track)
}

// --- Skip Embeds ---

pub fn create_skip_track_embed(
    track: &VideoMetadata,
    skipped: usize,
    remaining: usize,
) -> serenity_prelude::CreateEmbed {
    let embed = create_embed_template()
        .title("Track Skipped")
        .description(format!(
            "Skipped {skipped} track(s). Tracks remaining: {remaining}"
        ));
    populate_track_info(embed, track)
}

pub fn create_skip_playlist_embed(
    playlist: &PlaylistMetadata,
    skipped: usize,
    remaining: usize,
) -> serenity_prelude::CreateEmbed {
    let embed = create_embed_template()
        .title("Playlist Skipped")
        .description(format!(
            "Skipped {skipped} track(s). Tracks remaining: {remaining}"
        ));
    populate_playlist_info(embed, playlist)
}

// --- Queue Overview ---

pub fn create_queue_overview_embed(
    next_tracks: &[QueueElement],
    n_tracks: usize,
    n_items: usize,
) -> serenity_prelude::CreateEmbed {
    let mut embed = create_embed_template()
        .title("Queue Overview")
        .field("Queued Items", n_items.to_string(), true)
        .field("Queued Tracks", n_tracks.to_string(), true);

    for (i, element) in next_tracks.iter().enumerate() {
        match element {
            QueueElement::Track(track) => {
                if i == 0 {
                    embed = embed.thumbnail(&track.thumbnail_url);
                }
                embed = embed.field(
                    format!("{}. {}", i + 1, track.title),
                    format!("{} | [Link]({})", track.channel, track.url),
                    false,
                );
            }
            QueueElement::Playlist(playlist) => {
                if i == 0 {
                    embed = embed.thumbnail(&playlist.thumbnail_url);
                }
                embed = embed.field(
                    format!("{}. {} [Playlist]", i + 1, playlist.title),
                    format!(
                        "{} | {} tracks | [Link]({})",
                        playlist.channel,
                        playlist.items.len(),
                        playlist.url
                    ),
                    false,
                );
            }
        };
    }

    embed
}

// --- Radio Mode Embeds ---

pub fn create_radio_embed(
    is_enabled: bool,
    seed_track: Option<&VideoMetadata>,
) -> serenity_prelude::CreateEmbed {
    let mut embed = create_embed_template()
        .title("Radio Mode")
        .description(format!(
            "Radio mode is now **{}**.",
            if is_enabled { "ON" } else { "OFF" }
        ));

    if is_enabled {
        if let Some(track) = seed_track {
            embed = embed
                .field(
                    "Anchored To",
                    format!("[{}]({})", track.title, track.url),
                    false,
                )
                .field("Channel", &track.channel, true)
                .thumbnail(track.thumbnail_url.to_string());
        } else {
            embed = embed.description(
                "Radio mode is **ON**.\nIt will start automatically queueing tracks once playback begins.",
            );
        }
    }

    embed
}

pub fn create_radio_playing_embed(track: &VideoMetadata) -> serenity_prelude::CreateEmbed {
    let embed = create_embed_template()
        .title("Radio Auto-Play")
        .description("Automatically queued based on your listening history.");
    populate_track_info(embed, track)
}
