use crate::{
    actions::{channel_actions, playback_actions},
    checks::{author_in_shared_voice_channel, author_in_voice_channel},
    models::{DiscordError, QueueElement, RuntimeError},
    server::Context,
};
use tracing::{instrument, trace};

#[derive(Debug, Default, poise::ChoiceParameter)]
pub enum ResourceType {
    #[default]
    Track,
    Playlist,
}

#[poise::command(slash_command, subcommands("url", "search"))]
pub async fn play(_: Context<'_>) -> Result<(), RuntimeError> {
    Ok(())
}

/// Play a track or playlist using a URL or search query.
#[instrument(skip(ctx))]
#[poise::command(
    slash_command,
    check = "author_in_voice_channel",
    check = "author_in_shared_voice_channel"
)]
pub async fn url(
    ctx: Context<'_>,
    #[description = "URL of the desired resource."] path: String,
) -> Result<(), RuntimeError> {
    channel_actions::join_channel(ctx).await?;
    ctx.defer().await.map_err(DiscordError::Gateway)?;

    let metadata = ctx.data().youtube_client.process_url(&path).await;

    let queue_element = metadata.map(QueueElement::from)?;

    trace!(queue_element=%queue_element, "Adding queue element to queue.");
    playback_actions::add_element_to_queue(&ctx, queue_element).await?;
    playback_actions::start_queue_playback(&ctx).await?;
    Ok(())
}

/// Search for a resource to play using a query. The default resource type is a video/track.
#[instrument(skip(ctx))]
#[poise::command(
    slash_command,
    check = "author_in_voice_channel",
    check = "author_in_shared_voice_channel"
)]
pub async fn search(
    ctx: Context<'_>,

    #[description = "The type of the desired resource for a search query."] resource_type: Option<
        ResourceType,
    >,

    #[description = "Search query to the requested track or playlist."] query: String,
) -> Result<(), RuntimeError> {
    channel_actions::join_channel(ctx).await?;
    ctx.defer().await.map_err(DiscordError::Gateway)?;

    let queue_element = match resource_type {
        Some(ResourceType::Track) | None => ctx
            .data()
            .youtube_client
            .search_video(&query)
            .await
            .map(QueueElement::Track),
        Some(ResourceType::Playlist) => ctx
            .data()
            .youtube_client
            .search_playlist(&query, 50)
            .await
            .map(QueueElement::Playlist),
    }
    .map_err(RuntimeError::Youtube)?;

    trace!(queue_element=%queue_element, "Adding queue element to queue.");
    playback_actions::add_element_to_queue(&ctx, queue_element).await?;
    playback_actions::start_queue_playback(&ctx).await?;
    Ok(())
}
