use std::{collections::HashMap, sync::Arc};

use crate::{commands, configuration::ConfigurationVariables, models};
use poise::{serenity_prelude, FrameworkError};
use tokio::sync::RwLock;
use tracing::{error, info};

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
    Internal(String),

    #[error("Permissions Error: {0}")]
    Permissions(String),

    #[error("Unimplemented Error: Sorry this feature is planned but not implemented yet.")]
    Unimplemented,
}

impl From<poise::serenity_prelude::Error> for ServerError {
    fn from(value: poise::serenity_prelude::Error) -> Self {
        if cfg!(debug_assertions) {
            ServerError::Internal(value.to_string())
        } else {
            ServerError::Internal("Fatal error".to_string())
        }
    }
}

pub struct Server {
    serenity_client: poise::serenity_prelude::Client,
}

impl Server {
    /// Builds and configures the Discord bot client.
    pub async fn new(vars: ConfigurationVariables) -> Self {
        use songbird::SerenityInit;

        let discord_token = vars.discord_token().to_string();

        let framework = poise::Framework::builder()
            .options(Self::framework_options())
            .setup(move |ctx, _ready, fw| Box::pin(Self::setup_framework(ctx, fw, vars)))
            .build();

        let serenity_client = serenity_prelude::Client::builder(discord_token, Self::intents())
            .register_songbird()
            .framework(framework)
            .await
            .expect("Failed to build serenity client.");

        Self { serenity_client }
    }

    pub async fn start(&mut self) -> Result<(), ServerError> {
        self.serenity_client.start().await.map_err(|e| {
            error!(err=%e, "Failed to start server.");
            ServerError::Internal(e.to_string())
        })
    }

    /// Configures the Poise framework, mapping commands and error handlers.
    fn framework_options() -> poise::FrameworkOptions<ServerState, ServerError> {
        poise::FrameworkOptions {
            on_error: |err| Box::pin(Self::error_handler(err)),
            commands: vec![
                commands::pause::pause(),
                commands::play::play(),
                commands::queue::queue(),
                commands::resume::resume(),
                commands::skip::skip(),
                commands::stop::stop(),
            ],
            require_cache_for_guild_check: true,
            ..Default::default()
        }
    }

    /// Handles initialization logic executed once Discord is connected.
    async fn setup_framework(
        ctx: &serenity_prelude::Context,
        fw: &poise::Framework<ServerState, ServerError>,
        vars: ConfigurationVariables,
    ) -> Result<ServerState, ServerError> {
        // Initialize crypto provider
        rustls::crypto::ring::default_provider()
            .install_default()
            .expect("Failed to install rustls crypto provider");

        // Register Commands
        Self::register_commands(ctx, &fw.options().commands, &vars).await?;

        // Initialize State
        let youtube_client = models::YoutubeClient::new(vars.youtube_api_key()).await;

        Ok(ServerState {
            youtube_client,
            request_client: reqwest::Client::new(),
            configuration_variables: vars,
            guild_map: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    /// Defines the required Discord Gateway intents.
    fn intents() -> serenity_prelude::GatewayIntents {
        use serenity_prelude::GatewayIntents;
        GatewayIntents::non_privileged()
            | GatewayIntents::GUILD_VOICE_STATES
            | GatewayIntents::GUILD_MEMBERS
            | GatewayIntents::GUILD_PRESENCES
    }

    /// Handles registering commands globally or to a specific dev guild based on build profile.
    async fn register_commands(
        ctx: &serenity_prelude::Context,
        commands: &[poise::Command<ServerState, ServerError>],
        vars: &ConfigurationVariables,
    ) -> Result<(), ServerError> {
        let registration_res = if cfg!(debug_assertions) {
            info!("Registering commands to Dev Guild...");
            let guild_id = serenity_prelude::GuildId::new(vars.dev_guild_id() as u64);
            poise::builtins::register_in_guild(&ctx.http, commands, guild_id).await
        } else {
            info!("Registering commands Globally...");
            poise::builtins::register_globally(ctx, commands).await
        };

        registration_res.map_err(|e| {
            error!(e=%e, "Command registration failed.");
            ServerError::Internal(e.to_string())
        })?;

        Ok(())
    }

    /// Global framework error handler.
    async fn error_handler(err: FrameworkError<'_, ServerState, ServerError>) {
        error!(err=%err, "An error occurred in the framework.");
        match err {
            FrameworkError::Command { error, ctx, .. } => {
                _ = ctx.reply(error.to_string()).await;
            }
            FrameworkError::EventHandler { error, event, .. } => {
                error!(event=?event.snake_case_name(), "Event handler failed: {}", error);
            }
            FrameworkError::CommandCheckFailed { error, ctx, .. } => {
                if let Some(error) = error {
                    _ = ctx.reply(error.to_string()).await;
                }
            }
            _ => {}
        }
    }
}
