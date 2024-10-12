use tracing::{error, instrument};

use crate::{
    embeds,
    server::{Context, ServerError},
};

/// Prompt the Gemini AI model.
#[instrument(skip(ctx))]
#[poise::command(slash_command)]
pub async fn ask(
    ctx: Context<'_>,
    #[description = "Prompt for Gemini."] prompt: String,
) -> Result<(), ServerError> {
    ctx.defer().await?;

    match ctx.data().gemini_client.text_request(&prompt).await {
        Ok(response) => {
            ctx.send(
                poise::CreateReply::default().embed(embeds::create_ask_embed(
                    &ctx.data().gemini_client.model(),
                    &response,
                )),
            )
            .await?
        }
        Err(e) => {
            error!(err=%e, "An error occurred while prompting Gemini.");
            ctx.reply(e.to_string()).await?
        }
    };

    Ok(())
}
