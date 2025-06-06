use crate::{client, error::Error, server};
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
    pub fn load(path: &str) -> Result<Self, Error> {
        // TODO: Replace with supported version
        let contents =
            fs::read_to_string(path).map_err(|e| Error::ReadConfigFile(path.to_string(), e))?;

        let config: Self = serde_yaml::from_str(&contents)
            .map_err(|e| Error::ParseConfigFile(path.to_string(), e))?;

        config.validate().map_err(Error::InvalidConfig)?;
        Ok(config)
    }

    /// Validate the configuration
    pub fn validate(&self) -> Result<(), &'static str> {
        self.server.validate()?;
        self.github.validate()?;
        Ok(())
    }
}
