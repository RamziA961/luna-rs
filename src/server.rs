use std::{collections::HashMap, str::FromStr, sync::Arc};

use crate::{
    commands,
    configuration::ConfigurationVariables,
    metrics::Metric,
    models::{self, DiscordError, RuntimeError},
};
use poise::{
    FrameworkError,
    serenity_prelude::{self, prelude::TypeMapKey},
};
use tokio::sync::RwLock;
use tracing::{error, info};

pub type Context<'a> = poise::Context<'a, ServerState, RuntimeError>;

#[derive(Debug, Clone)]
pub struct ServerState {
    pub configuration_variables: ConfigurationVariables,
    pub request_client: reqwest::Client,
    pub youtube_client: models::YoutubeClient,
    pub guild_map: Arc<RwLock<HashMap<String, models::GuildState>>>,
}

struct GuildMapKey;
impl TypeMapKey for GuildMapKey {
    type Value = Arc<RwLock<HashMap<String, models::GuildState>>>;
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

    pub async fn start(&mut self) -> Result<(), RuntimeError> {
        self.serenity_client.start().await.map_err(|e| {
            error!(err=%e, "Fatal error");
            DiscordError::Gateway(e).into()
        })
    }

    // Graceful shutdown. Leave all channels and close gateway.
    pub async fn stop(&mut self) {
        let (sb_manager, guild_map) = {
            let data = self.serenity_client.data.read().await;
            let sb_manager = data.get::<songbird::SongbirdKey>().cloned();
            let guild_map = data.get::<GuildMapKey>().cloned();
            (sb_manager, guild_map)
        };

        if let (Some(sb), Some(gm)) = (sb_manager, guild_map) {
            let map = gm.read().await;

            for guild_id_str in map.keys() {
                if let Ok(guild_id) = serenity_prelude::GuildId::from_str(guild_id_str)
                    && let Err(e) = sb.remove(guild_id).await
                {
                    error!(guild_id = %guild_id, err = %e, "Failed to leave channel during shutdown");
                }
            }

            info!("Disconnected from all voice channels.");
        }

        self.serenity_client.shard_manager.shutdown_all().await;
        info!("Gateway connection closed.");
    }

    /// Configures the Poise framework, mapping commands and error handlers.
    fn framework_options() -> poise::FrameworkOptions<ServerState, RuntimeError> {
        poise::FrameworkOptions {
            commands: vec![
                commands::pause::pause(),
                commands::play::play(),
                commands::queue::queue(),
                commands::radio::radio(),
                commands::resume::resume(),
                commands::skip::skip(),
                commands::stop::stop(),
            ],
            pre_command: |ctx| {
                Box::pin(async move {
                    let command_name = ctx.command().name.clone();
                    let author_id = ctx.author().id.get();
                    let guild_id = ctx.guild_id().map(|id| id.get()).unwrap_or(0);

                    metrics::counter!(Metric::CommandsInvokedTotal.as_ref(), "command" => command_name.clone())
                        .increment(1);

                    tracing::info!(
                        command = %command_name,
                        user = %author_id,
                        guild = %guild_id,
                        "Command invoked"
                    );
                })
            },
            post_command: |ctx| {
                Box::pin(async move {
                    metrics::counter!(Metric::CommandsCompletedTotal.as_ref(), "command" => ctx.command().name.clone()).increment(1);

                    tracing::info!(
                        command = %ctx.command().name,
                        user_id = %ctx.author().id.get(),
                        "Command completed successfully"
                    );
                })
            },
            on_error: |err| Box::pin(Self::error_handler(err)),
            require_cache_for_guild_check: true,
            ..Default::default()
        }
    }

    /// Handles initialization logic executed once Discord is connected.
    async fn setup_framework(
        ctx: &serenity_prelude::Context,
        fw: &poise::Framework<ServerState, RuntimeError>,
        vars: ConfigurationVariables,
    ) -> Result<ServerState, RuntimeError> {
        // Initialize crypto provider
        rustls::crypto::ring::default_provider()
            .install_default()
            .expect("Failed to install rustls crypto provider");

        // Register Commands
        Self::register_commands(ctx, &fw.options().commands, &vars).await?;

        // Initialize State
        let youtube_client = models::YoutubeClient::new(vars.youtube_api_key()).await;
        let guild_map = Arc::new(RwLock::new(HashMap::new()));

        {
            let mut data = ctx.data.write().await;
            data.insert::<GuildMapKey>(guild_map.clone());
        }

        Ok(ServerState {
            youtube_client,
            request_client: reqwest::Client::new(),
            configuration_variables: vars,
            guild_map,
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
        commands: &[poise::Command<ServerState, RuntimeError>],
        vars: &ConfigurationVariables,
    ) -> Result<(), RuntimeError> {
        #[cfg(debug_assertions)]
        {
            info!("Registering commands to Dev Guild...");
            let guild_id = serenity_prelude::GuildId::new(vars.dev_guild_id() as u64);
            poise::builtins::register_in_guild(&ctx.http, commands, guild_id)
                .await
                .map_err(DiscordError::Gateway)?;
        }
        #[cfg(not(debug_assertions))]
        {
            info!("Registering commands Globally...");
            poise::builtins::register_globally(ctx, commands)
                .await
                .map_err(DiscordError::Gateway)?;
        }
        Ok(())
    }

    /// Global framework error handler.
    async fn error_handler(err: FrameworkError<'_, ServerState, RuntimeError>) {
        // TODO: Explore simplifying with spans?
        match err {
            FrameworkError::Command { error, ctx, .. } => {
                let command_name = ctx.command().name.clone();
                let user_id = ctx.author().id.get();
                let guild_id = ctx.guild_id().map(|id| id.get()).unwrap_or(0);

                let user_response = match error {
                    RuntimeError::User(msg) => {
                        metrics::counter!(Metric::CommandErrorsTotal.as_ref(), "type" => "user_error", "command" => command_name.clone()).increment(1);
                        info!(command=%command_name, user=%user_id, guild=%guild_id, "User error: {msg}");
                        msg
                    }
                    RuntimeError::Unimplemented => {
                        "Sorry, this feature is planned but not implemented yet.".to_string()
                    }
                    // Hide internals
                    internal_fault => {
                        metrics::counter!(Metric::CommandErrorsTotal.as_ref(), "type" => "internal_error", "command" => command_name.clone()).increment(1);
                        error!(
                            command=%command_name,
                            user=%user_id,
                            guild=%guild_id,
                            err=%internal_fault,
                            "System error during command execution."
                        );
                        "An unexpected internal error occurred while processing your request."
                            .to_string()
                    }
                };
                let reply = poise::CreateReply::default()
                    .embed(crate::embeds::create_error_embed(&user_response));

                if let Err(e) = ctx.send(reply).await {
                    error!(err=%e, user=%user_id, guild=%guild_id, "Failed to send error embed to user.");
                }
            }

            FrameworkError::CommandCheckFailed { error, ctx, .. } => {
                let command_name = ctx.command().name.clone();
                let user_id = ctx.author().id.get();
                let guild_id = ctx.guild_id().map(|id| id.get()).unwrap_or(0);

                let user_response = if let Some(err) = error {
                    match err {
                        RuntimeError::User(msg) => {
                            metrics::counter!(Metric::CommandErrorsTotal.as_ref(), "type" => "checks_failed", "command" => command_name.clone()).increment(1);
                            info!(command=%command_name, user=%user_id, guild=%guild_id, "Check failed (User): {msg}");
                            msg
                        }
                        internal_fault => {
                            metrics::counter!(Metric::CommandErrorsTotal.as_ref(), "type" => "internal_error", "command" => command_name.clone()).increment(1);
                            error!(command=%command_name, user=%user_id, guild=%guild_id, err=%internal_fault, "Check failed (Internal).");
                            "An unexpected error occurred during command validation.".to_string()
                        }
                    }
                } else {
                    info!(command=%command_name, user=%user_id, guild=%guild_id, "Check failed silently.");
                    "You do not meet the requirements to run this command.".to_string()
                };

                let reply = poise::CreateReply::default()
                    .embed(crate::embeds::create_error_embed(&user_response));

                let _ = ctx.send(reply).await;
            }

            FrameworkError::EventHandler { error, event, .. } => {
                metrics::counter!(Metric::EventHandlerErrorsTotal.as_ref(), "type" => "background_error", "event" => event.snake_case_name()).increment(1);
                error!(event=?event.snake_case_name(), err=%error, "Background event handler failed.");
            }

            FrameworkError::Setup { error, .. } => {
                metrics::counter!(Metric::SystemErrorsTotal.as_ref(), "type" => "setup")
                    .increment(1);
                error!(err=%error, "Fatal error during bot setup.");
            }
            other => {
                metrics::counter!(Metric::SystemErrorsTotal.as_ref(), "type" => "unhandled_framework")
                    .increment(1);
                error!(err=%other, "Unhandled framework error.");
            }
        }
    }
}
