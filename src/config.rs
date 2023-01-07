use crate::plug;
use serde::Deserialize;
use std::time::Duration;

/// Configuration of this application
#[derive(Deserialize, Debug, Clone)]
pub struct Config {

    // Network timeout in milliseconds
    network_timeout_ms: u64,

    /// Configurations of Shelly Plug (S) devices
    pub shelly_plugs: Vec<plug::Config>,
}

impl Config {

    // Read the config file from 'config.json'
    pub fn read_from_deafult_file() -> Config {
        let config_as_string: String = std::fs::read_to_string("config.json")
            .expect("config file can not be read from 'config.json'");
        let config: Config = serde_json::from_str(&config_as_string)
            .expect("config file could not be parsed as JSON");    
        config
    }

    /// Network connection timeout
    pub fn timeout(&self) -> Duration {
        Duration::from_millis(self.network_timeout_ms)
    }
}
