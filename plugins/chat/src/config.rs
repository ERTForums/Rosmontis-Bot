use kovi::log::{error, info};
use kovi::utils::{load_toml_data, save_toml_data};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Serialize, Deserialize)]
pub struct Config {
    pub(crate) api_url: String,
    pub(crate) proxy: Option<String>,
    pub(crate) bearer_token: String,
    pub(crate) model: String,
    pub(crate) system_promote: String,
    pub(crate) temperature: Option<f32>,
    pub(crate) max_output_tokens: Option<u32>,
}

impl Config {
    pub fn from_file(path: PathBuf) -> Self {
        if !path.is_file() {
            error!("Config is not exist");
            info!("Create new config: {}", path.display());
            let config = Config::default();
            save_toml_data(&config, path).expect("Failed to write new config");
            panic!()
        }
        load_toml_data(Config::default(), path).expect("Fail to load config")
    }
}

impl Default for Config {
    fn default() -> Self {
        Config {
            api_url: "Your API URL".to_string(),
            proxy: None,
            bearer_token: "Your API Token".to_string(),
            model: "Your Model".to_string(),
            system_promote: "System Promote".to_string(),
            temperature: None,
            max_output_tokens: None,
        }
    }
}
