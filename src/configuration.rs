use config::{Config, File};

#[cfg(not(debug_assertions))]
const CONFIG_SORUCE: &str = "Secrets.toml";
#[cfg(debug_assertions)]
const CONFIG_SORUCE: &str = "Secrets.dev.toml";

#[derive(Debug, Clone)]
pub struct ConfigurationVariables {
    discord_token: String,
    youtube_api_key: String,
    #[cfg(debug_assertions)]
    dev_guild_id: usize,
}

impl Default for ConfigurationVariables {
    fn default() -> Self {
        Self::new()
    }
}

impl ConfigurationVariables {
    pub fn new() -> Self {
        let vars = Config::builder()
            .add_source(File::with_name(CONFIG_SORUCE))
            .build()
            .unwrap_or_else(|_| {
                panic!("Configuration file not found. {CONFIG_SORUCE} file expected.")
            });

        let discord_token = vars
            .get_string("DISCORD_TOKEN")
            .expect("Expected DISCORD_TOKEN.");

        let youtube_api_key = vars
            .get_string("YOUTUBE_API_KEY")
            .expect("Expected YOUTUBE_API_KEY.");

        #[cfg(debug_assertions)]
        let dev_guild_id = vars.get::<usize>("GUILD_ID").expect("Expected GUILD_ID.");

        Self {
            discord_token,
            youtube_api_key,
            #[cfg(debug_assertions)]
            dev_guild_id,
        }
    }

    pub fn discord_token(&self) -> &str {
        &self.discord_token
    }

    pub fn youtube_api_key(&self) -> &str {
        &self.youtube_api_key
    }

    #[cfg(debug_assertions)]
    pub fn dev_guild_id(&self) -> usize {
        self.dev_guild_id
    }
}
