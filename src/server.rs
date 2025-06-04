use crate::{client::Client, types::CheckRunEvent, types::PullRequestEvent};
use axum::{
    Json, Router,
    extract::State,
    http::{HeaderMap, HeaderValue, StatusCode},
    routing::{get, post},
};
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tokio::signal;
use tower_http::trace::TraceLayer;
use tracing::{debug, error, info, warn};

mod hex;
#[cfg(test)]
mod test;
mod tls;

pub const SERVER_STATUS_OK: &str = "ok";
pub const SERVER_STATUS_ERROR: &str = "error";
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

/// HTTP Server for receiving webhook events from GitHub
pub struct Server {
    options: ServerOptions,
}

#[derive(Clone)]
struct ServerState {
    // TODO: Check if this could be a string
    webhook_secret: Option<String>,
    // TODO: This could be a reference with a mutex for token cache
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
        let router = new_router(state);

        let addr = SocketAddr::from(([0, 0, 0, 0, 0, 0, 0, 0], self.options.port));
        info!("Starting server on {}", addr);

        if self.options.ssl.enabled {
            let listener =
                tls::TlsListener::bind(addr, &self.options.ssl.key, &self.options.ssl.cert)
                    .await
                    .map_err(|e| format!("Failed to bind to port with SSL: {e}"))?;

            axum::serve(listener, router)
                .with_graceful_shutdown(shutdown_signal())
                .await
                .map_err(|e| format!("Server error: {e}"))
        } else {
            let listener = TcpListener::bind(addr)
                .await
                .map_err(|e| format!("Failed to bind to port: {e}"))?;

            axum::serve(listener, router)
                .with_graceful_shutdown(shutdown_signal())
                .await
                .map_err(|e| format!("Server error: {e}"))
        }
    }
}

fn new_router(state: ServerState) -> Router {
    let webhook_router: Router = Router::new()
        .route("/webhook", post(webhook_handler))
        .with_state(state)
        .layer(TraceLayer::new_for_http());

    // Do not use tracing for the health check endpoint
    let health_router: Router = Router::new().route("/healthz", get(healthz));

    Router::new().merge(webhook_router).merge(health_router)
}

/// Expose health check endpoint
/// Can be used when running under kubernetes to check if the server is running
/// GET /healthz
async fn healthz() -> (StatusCode, Json<Response>) {
    (StatusCode::OK, Json(Response::new()))
}

/// Handle the webhook events send from GitHub
/// POST /webhook
async fn webhook_handler(
    headers: HeaderMap,
    state: State<ServerState>,
    payload: String,
) -> (StatusCode, Json<Response>) {
    let event = match headers.get("X-GitHub-Event") {
        Some(event) => event
            .to_str()
            .unwrap_or("could not read X-GitHub-Event header"),
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(Response::error("Missing X-GitHub-Event header")),
            );
        }
    };
    debug!("Received webhook event: {}", event);
    if let Err(e) = verify_webhook(
        headers.get("X-Hub-Signature-256"),
        state.webhook_secret.as_deref(),
        &payload,
    ) {
        warn!("Failed to verify webhook signature: {}", e.1.message);
        return e;
    }

    match event {
        "check_run" => handle_check_run_event(&state.github, &payload).await,
        "pull_request" => handle_pull_request_event(&state.github, &payload).await,
        event => {
            let message = format!("Received unsupported event: {}", event);
            info!("{message}");
            (StatusCode::NOT_IMPLEMENTED, Json(Response::error(&message)))
        }
    }
}

/// Verify the webhook request against the shared secret
fn verify_webhook(
    signature: Option<&HeaderValue>,
    secret: Option<&str>,
    payload: &str,
) -> Result<(), (StatusCode, Json<Response>)> {
    let secret = match secret {
        Some(s) => s,
        None => {
            return Ok(());
        }
    };

    let signature = match signature {
        Some(s) => s.to_str().map_err(|e| {
            info!("Failed to read X-Hub-Signature-256 header: {e}");
            (
                StatusCode::FORBIDDEN,
                Json(Response::error("Invalid X-Hub-Signature-256 header")),
            )
        })?,
        None => {
            return Err((
                StatusCode::FORBIDDEN,
                Json(Response::error("Missing X-Hub-Signature-256 header")),
            ));
        }
    };
    let signature = signature.strip_prefix("sha256=").unwrap_or(signature);
    let signature = hex::decode_hex(signature).map_err(|_| {
        (
            StatusCode::FORBIDDEN,
            Json(Response::error("Invalid X-Hub-Signature-256 header")),
        )
    })?;

    let mut mac = Hmac::<sha2::Sha256>::new_from_slice(secret.as_bytes()).map_err(|e| {
        error!("Failed to create HMAC from secret: {e}");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(Response::error("Failed to create HMAC from secret")),
        )
    })?;
    mac.update(payload.as_bytes());

    mac.verify_slice(signature.as_slice()).map_err(|_| {
        (
            StatusCode::FORBIDDEN,
            Json(Response::error("Invalid webhook signature")),
        )
    })?;

    Ok(())
}

/// Handle webhook pull_request events
async fn handle_pull_request_event(client: &Client, payload: &str) -> (StatusCode, Json<Response>) {
    let payload: PullRequestEvent = match serde_json::from_str(payload) {
        Ok(event) => event,
        Err(e) => {
            warn!("Failed to parse pull_request event payload: {e}");
            return (
                StatusCode::BAD_REQUEST,
                Json(Response::error("Invalid pull_request event payload")),
            );
        }
    };

    match payload.action.as_str() {
        "opened" | "synchronize" => {}
        action => {
            debug!("Ignoring pull_request event with action: {action}");
            return (StatusCode::OK, Json(Response::new()));
        }
    }

    let app_id = match payload.installation {
        Some(installation) => installation.id,
        None => {
            warn!("Missing app installation id in pull_request event");
            return (
                StatusCode::BAD_REQUEST,
                Json(Response::error("Missing app installation id")),
            );
        }
    };

    if let Err(e) = client
        .create_check_run(
            app_id,
            &payload.repository.full_name,
            &payload.pull_request.head.sha,
        )
        .await
    {
        error!("Failed to create check run: {e}");
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(Response::error("Failed to create check-run")),
        );
    };
    info!(
        "Created check run for pull request {} - {}",
        payload.repository.full_name, payload.pull_request.number
    );
    (StatusCode::OK, Json(Response::new()))
}

/// Handle webhook check_run events
async fn handle_check_run_event(client: &Client, payload: &str) -> (StatusCode, Json<Response>) {
    let payload: CheckRunEvent = match serde_json::from_str(payload) {
        Ok(event) => event,
        Err(e) => {
            warn!("Failed to parse check_run event payload: {e}");
            return (
                StatusCode::BAD_REQUEST,
                Json(Response::error("Invalid check_run event payload")),
            );
        }
    };

    if payload
        .check_run
        .app
        .is_some_and(|app| app.client_id == client.client_id())
    {
        debug!("Ignoring check_run event from our own app");
        return (StatusCode::OK, Json(Response::new()));
    }

    let app_id = match payload.installation {
        Some(installation) => installation.id,
        None => {
            warn!("Missing app installation id in check_run event");
            return (
                StatusCode::BAD_REQUEST,
                Json(Response::error("Missing app installation id")),
            );
        }
    };

    let (uncompleted, own_run) = match client
        .get_check_run_status(
            app_id,
            &payload.repository.full_name,
            &payload.check_run.head_sha,
        )
        .await
    {
        Ok(check_runs) => check_runs,
        Err(e) => {
            error!("{e}");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(Response::error("Failed to get check-runs")),
            );
        }
    };
    if let Err(e) = client
        .update_check_run(
            app_id,
            &payload.repository.full_name,
            &payload.check_run.head_sha,
            uncompleted == 0,
            own_run,
        )
        .await
    {
        error!("Failed to update check-run: {e}");
    }
    (StatusCode::OK, Json(Response::new()))
}

/// Detailed status of the Webserver
#[derive(Debug, Serialize, Deserialize)]
pub struct Response {
    /// Status of the server.
    /// "ok" if everything is running fine, "error" if something is wrong.
    // TODO: use &str
    pub status: String,
    /// Optional message providing more details about the status.
    pub message: String,
}

impl Response {
    /// Create a new response with ok status.
    pub fn new() -> Self {
        Self {
            status: SERVER_STATUS_OK.to_string(),
            message: SERVER_MESSAGE_OK.to_string(),
        }
    }

    /// Create a new response with the error status.
    pub fn error(message: &str) -> Self {
        Self {
            status: SERVER_STATUS_ERROR.to_string(),
            message: message.to_string(),
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
