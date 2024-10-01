use crate::{
    actions::{channel_actions, playback_actions},
    checks::author_in_voice_channel,
    models::{QueueElement, ResourceType},
    server::{Context, ServerError},
};
use poise::serenity_prelude::model::guild;
use tracing::instrument;

/// Play a track or playlist using a URL or search query.
#[poise::command(slash_command, subcommands("url", "search"))]
pub async fn play(_: Context<'_>) -> Result<(), ServerError> {
    Ok(())
}

#[instrument(skip(_ctx))]
#[poise::command(slash_command)]
pub async fn url(
    _ctx: Context<'_>,
    #[description = "URL of the desired resource."] path: String,
) -> Result<(), ServerError> {
    Err(ServerError::UnimplementedError)
}

/// Search for a resource to play using a query. The default resource type is a video/track.
#[instrument(skip(ctx))]
#[poise::command(slash_command, check = "author_in_voice_channel")]
pub async fn search(
    ctx: Context<'_>,

    //#[rename = "resource type"]
    #[description = "The type of the desired resource for a search query."] resource_type: Option<
        ResourceType,
    >,

    #[description = "Search query to the requested track or playlist."] query: String,
) -> Result<(), ServerError> {
    channel_actions::join_channel(ctx).await?;

    let guild_id = ctx.guild_id().ok_or_else(|| {
        ServerError::InternalError("Could not find guild information".to_string())
    })?;

    let queue_element = match resource_type {
        Some(ResourceType::Track) | None => {
            let metadata = ctx
                .data()
                .youtube_client
                .search_video(&query)
                .await
                .map(|video| QueueElement::Track(video));

            metadata
        }
        Some(ResourceType::Playlist) => {
            let metadata = ctx
                .data()
                .youtube_client
                .search_playlist(&query)
                .await
                .map(|playlist| QueueElement::Playlist(playlist));

            metadata
        }
    };

    match queue_element {
        Ok(element) => {
            _ = ctx.defer().await;

            let mut guard = ctx.data().guild_map.write().await;

            let guild_state = guard.entry(guild_id.to_string()).or_default();

            guild_state.playback_state.enqueue(element);
            //let curr = guild_state
            //    .playback_state
            //    .get_current_track()
            //    .as_ref()
            //    .unwrap();

            //_ = ctx.reply(format!("Queued: {} - {}", curr.title, curr.channel))
            //    .await;
        }
        Err(e) => {
            _ = ctx.reply(e.to_string()).await;
        }
    };

    playback_actions::play_song(&ctx, &guild_id).await?;

    _ = ctx.say("OK").await;
    Ok(())
}
