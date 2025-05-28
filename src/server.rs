use serde::{Deserialize, Serialize};

/// Options for the http server
#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct ServerOptions {
    /// Port to bind to, defaults to 8080
    #[serde(default = "default_port")]
    pub port: u16,

    /// Optional ssl configuration for the server
    pub ssl: SSLOptions,

    /// Shared webhook secret for verifying the webhook sender
    #[serde(skip_serializing_if = "str::is_empty")]
    pub webhook_secret: String,
}

fn default_port() -> u16 {
    8080
}

impl ServerOptions {
    /// Validate the server options
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.port == 0 {
            return Err("Port can't be 0");
        }
        self.ssl.validate()
    }
}

impl Default for ServerOptions {
    fn default() -> Self {
        Self {
            port: default_port(),
            webhook_secret: std::env::var("CERBERUS_WEBHOOK_SECRET").unwrap_or_default(),
            ssl: SSLOptions::default(),
        }
    }
}

/// SSL configuration for the server
#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct SSLOptions {
    /// Whether to enable SSL, defaults to false
    pub enabled: bool,
    /// Path to the SSL private key file
    pub key: String,
    /// Path to the SSL certificate file
    pub cert: String,
}

impl SSLOptions {
    /// Validate the SSL options
    pub fn validate(&self) -> Result<(), &'static str> {
        if !self.enabled {
            return Ok(());
        }
        if self.key.is_empty() || self.cert.is_empty() {
            return Err("Incomplete SSL configuration: cert and key must be set if SSL is enabled");
        }
        Ok(())
    }
}

impl Default for SSLOptions {
    fn default() -> Self {
        Self {
            enabled: bool::default(),
            key: String::default(),
            cert: String::default(),
        }
    }
}
