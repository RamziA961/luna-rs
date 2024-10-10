use poise::serenity_prelude::{self, Color, Timestamp};

use crate::models::{PlaylistMetadata, QueueElement, VideoMetadata};

fn create_embed_template() -> serenity_prelude::CreateEmbed {
    serenity_prelude::CreateEmbed::new()
        .color(Color::DARK_ORANGE)
        .timestamp(Timestamp::now())
}

fn create_playlist_embed(playlist: &PlaylistMetadata) -> serenity_prelude::CreateEmbed {
    create_embed_template()
        .title(format!("{} - {}", playlist.title, playlist.channel))
        .url(playlist.url.to_string())
        .thumbnail(playlist.thumbnail_url.to_string())
}

fn create_track_embed(track: &VideoMetadata) -> serenity_prelude::CreateEmbed {
    create_embed_template()
        .title(format!("{} - {}", track.title, track.channel))
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
            format!("{} - {}\n{}", first.title, first.channel, first.url,),
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

pub fn create_queue_overview_embed(
    next_tracks: &Vec<&QueueElement>,
    n_tracks: usize,
    n_items: usize,
) -> serenity_prelude::CreateEmbed {
    let mut template = create_embed_template();

    for (i, element) in next_tracks.iter().enumerate() {
        let (name, value) = match element {
            QueueElement::Track(VideoMetadata {
                title,
                channel,
                url,
                thumbnail_url,
                ..
            }) => {
                if i == 0 {
                    template = template.thumbnail(thumbnail_url);
                }

                (format!("{}. {title}", i + 1), format!("{channel}\n{url}"))
            }
            QueueElement::Playlist(PlaylistMetadata {
                title,
                channel,
                url,
                items,
                thumbnail_url,
                ..
            }) => {
                if i == 0 {
                    template = template.thumbnail(thumbnail_url);
                }

                (
                    format!("{}. {title} [{} tracks]", i + 1, items.len()),
                    format!("{channel}\n{url}"),
                )
            }
        };

        template = template.field(name, value, false);
    }

    template
        .title("Song Queue")
        .field("Queued Items", n_items.to_string(), true)
        .field("Queued Tracks", n_tracks.to_string(), true)
}
