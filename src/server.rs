use crate::client::Client;
use axum::{Json, Router, http::StatusCode, routing::get};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use tokio::signal;
use tower_http::trace::TraceLayer;
use tracing::info;

pub const SERVER_STATUS_OK: &str = "ok";
pub const SERVER_MESSAGE_OK: &str = "Server is running fine";

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
    pub webhook_secret: Option<String>,
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
            webhook_secret: std::env::var("CERBERUS_WEBHOOK_SECRET").ok(),
            ssl: SSLOptions::default(),
        }
    }
}

/// SSL configuration for the server
#[derive(Serialize, Deserialize, Debug, Default)]
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

// HTTP Server for receiving webhook events from GitHub
pub struct Server {
    options: ServerOptions,
}

#[derive(Clone)]
struct ServerState {
    webhook_secret: Option<String>,
    github: Client,
}

impl ServerState {
    /// Create a new server state with the given webhook secret and GitHub client
    pub fn new(webhook_secret: Option<String>, github: Client) -> Self {
        Self {
            webhook_secret,
            github,
        }
    }
}

impl Server {
    /// Create a new server with the given options and GitHub client
    pub fn new(options: ServerOptions) -> Self {
        Self { options }
    }

    /// Run the server
    /// Server will shutdown gracefully on Ctrl+C or SIGTERM
    pub async fn run(&self, github: Client) -> Result<(), String> {
        // TODO: Convert strings to &str where possible to avoid unnecessary allocations
        let state = ServerState::new(self.options.webhook_secret.clone(), github);

        let router: Router = Router::new()
            .route("/healthz", get(healthz))
            .with_state(state)
            .layer(TraceLayer::new_for_http());

        let addr = SocketAddr::from(([0, 0, 0, 0, 0, 0, 0, 0], self.options.port));
        info!("Starting server on {}", addr);

        let listener = tokio::net::TcpListener::bind(addr)
            .await
            .map_err(|e| format!("Failed to bind to port: {e}"))?;
        // TODO: Add SSL support if enabled
        axum::serve(listener, router)
            .with_graceful_shutdown(shutdown_signal())
            .await
            .map_err(|e| format!("Server error: {e}"))
    }
}

/// Expose health check endpoint
/// Can be used when running under kubernetes to check if the server is running
async fn healthz() -> (StatusCode, Json<HealthCheckResponse>) {
    (StatusCode::OK, Json(HealthCheckResponse::new()))
}

/// Detailed status of the Webserver
#[derive(Debug, Serialize, Deserialize)]
pub struct HealthCheckResponse {
    /// Status of the server.
    /// "ok" if everything is running fine, "error" if something is wrong.
    pub status: String,
    /// Optional message providing more details about the status.
    pub message: String,
}

impl HealthCheckResponse {
    /// Create a new health check response with "ok" status.
    pub fn new() -> Self {
        Self {
            status: SERVER_STATUS_OK.to_string(),
            message: SERVER_MESSAGE_OK.to_string(),
        }
    }
}

/// Asynchronously wait for a shutdown signal (Ctrl+C or SIGTERM).
async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}
