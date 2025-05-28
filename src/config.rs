use crate::{client, server};
use serde::{Deserialize, Serialize};
use std::fs;

/// Configuration options for the bot
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "kebab-case")]
pub struct Configuration {
    /// Set the log level to use.
    /// Accepted values are "error", "warn", "info" and "debug".
    #[serde(skip_serializing_if = "str::is_empty", default = "default_log_level")]
    pub log_level: String,
    /// Server configuration
    #[serde(default)]
    pub server: server::ServerOptions,
    /// Client configuration
    pub github: client::ClientOptions,
}

fn default_log_level() -> String {
    "info".to_string()
}

impl Configuration {
    /// Load the configuration from a file
    pub fn load(path: &str) -> Result<Self, String> {
        // TODO: Replace with supported version
        let contents = fs::read_to_string(path)
            .map_err(|e| format!("Failed to read config file '{}': {}", path, e))?;

        let config: Self = serde_yaml::from_str(&contents)
            .map_err(|e| format!("Failed to parse config file '{}': {}", path, e))?;

        config.validate()?;
        Ok(config)
    }

    /// Validate the configuration
    pub fn validate(&self) -> Result<(), &'static str> {
        self.server.validate()?;
        self.github.validate()?;
        Ok(())
    }
}
