use std::{collections::HashMap, sync::Arc};

use crate::{commands, configuration::ConfigurationVariables, models};
use poise::{serenity_prelude, FrameworkError};
use tokio::sync::RwLock;
use tracing::error;

pub type Context<'a> = poise::Context<'a, ServerState, ServerError>;

#[derive(Debug, Clone)]
pub struct ServerState {
    pub configuration_variables: ConfigurationVariables,
    pub request_client: reqwest::Client,
    pub youtube_client: models::YoutubeClient,
    pub guild_map: Arc<RwLock<HashMap<String, models::GuildState>>>,
}

#[derive(Debug, thiserror::Error)]
pub enum ServerError {
    #[error("Whoops, an internal error occurred: {0}")]
    InternalError(String),

    #[error("Permissions Error: {0}")]
    PermissionsError(String),

    #[error("Unimplemented Error: Sorry this feature is planned but not implemented yet.")]
    UnimplementedError,
}

impl From<poise::serenity_prelude::Error> for ServerError {
    fn from(value: poise::serenity_prelude::Error) -> Self {
        if cfg!(debug_assertions) {
            ServerError::InternalError(value.to_string())
        } else {
            ServerError::InternalError("Fatal error".to_string())
        }
    }
}

pub struct Server {
    serenity_client: poise::serenity_prelude::Client,
}

impl Server {
    pub async fn new(vars: ConfigurationVariables) -> Self {
        use poise::serenity_prelude::GatewayIntents;
        use songbird::SerenityInit;

        let intents = GatewayIntents::non_privileged()
            | GatewayIntents::GUILD_VOICE_STATES
            | GatewayIntents::GUILD_MEMBERS
            | GatewayIntents::GUILD_PRESENCES;

        #[cfg(debug_assertions)]
        let guild_id = vars.dev_guild_id() as u64;
        let discord_token = vars.discord_token().to_string();

        let framework = poise::Framework::<ServerState, ServerError>::builder()
            .options(poise::FrameworkOptions {
                on_error: |err| Box::pin(Self::error_handler(err)),
                commands: vec![
                    commands::pause::pause(),
                    commands::play::play(),
                    commands::resume::resume(),
                    commands::skip::skip(),
                    commands::stop::stop(),
                ],
                require_cache_for_guild_check: true,
                ..Default::default()
            })
            .setup(move |ctx, _, fw| {
                Box::pin(async move {
                    let registration_res = if cfg!(debug_assertions) {
                        poise::builtins::register_in_guild(
                            &ctx.http,
                            &fw.options().commands,
                            serenity_prelude::GuildId::new(guild_id),
                        )
                        .await
                    } else {
                        poise::builtins::register_globally(ctx, &fw.options().commands).await
                    };

                    registration_res.map_err(|e| {
                        error!(e=%e, "Command registration failed.");
                        ServerError::InternalError(e.to_string())
                    })?;

                    Ok(ServerState {
                        youtube_client: models::YoutubeClient::new(vars.youtube_api_key()),
                        request_client: reqwest::Client::new(),
                        configuration_variables: vars,
                        guild_map: Arc::new(RwLock::new(HashMap::new())),
                    })
                })
            })
            .build();

        let serenity_client = poise::serenity_prelude::Client::builder(discord_token, intents)
            .register_songbird()
            .framework(framework)
            .await
            .expect("Failed to build serenity client.");

        Self { serenity_client }
    }

    pub async fn start(&mut self) -> Result<(), ServerError> {
        self.serenity_client.start().await.map_err(|e| {
            error!(err=%e, "Failed to start server.");
            ServerError::InternalError(e.to_string())
        })
    }

    async fn error_handler(err: FrameworkError<'_, ServerState, ServerError>) {
        error!(err=%err, "An error occurred.");
        match err {
            FrameworkError::Command { error, ctx, .. } => {
                _ = ctx.reply(error.to_string()).await;
            }
            FrameworkError::EventHandler { ref error, .. } => {
                _ = err
                    .ctx()
                    .map(|ctx| async move { ctx.say(error.to_string()).await })
            }
            FrameworkError::CommandCheckFailed { error, ctx, .. } => {
                if let Some(error) = error {
                    _ = ctx.reply(error.to_string()).await
                }
            }
            e => {
                error!(err=%e);
            }
        }
    }
}
