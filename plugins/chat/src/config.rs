use kovi::log::{error, info};
use kovi::utils::{load_toml_data, save_toml_data};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Read;
use std::path::PathBuf;

#[derive(Serialize, Deserialize)]
pub struct Config {
    pub(crate) api_url: String,
    pub(crate) bearer_token: String,
    pub(crate) model: String,
    pub(crate) system_promote: String,
    pub(crate) temperature: Option<f32>,
    pub(crate) max_tokens: Option<u32>,
    pub(crate) user_repository: PathBuf,
}

impl Config {
    pub fn from_file() -> Self {
        let path = std::env::current_dir()
            .expect("Failed to get current directory")
            .join("config.toml");
        if !path.is_file() {
            error!("Config is not exist");
            info!("Create new config: {}", path.display());
            let config = Config::default();
            save_toml_data(
                &kovi::toml::to_string(&config).expect("Failed to write new config"),
                path,
            )
            .expect("Failed to write new config");
            panic!()
        }
        load_toml_data(Config::default(), path).expect("Fail to load config")
    }
}

impl Default for Config {
    fn default() -> Self {
        Config {
            api_url: "Your API URL".to_string(),
            bearer_token: "Your API Token".to_string(),
            model: "Your Model".to_string(),
            system_promote: "System Promote".to_string(),
            temperature: None,
            max_tokens: None,
            user_repository: PathBuf::from("user.json"),
        }
    }
}
