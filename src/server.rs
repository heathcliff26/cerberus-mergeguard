use crate::{
    client::Client,
    error::Error,
    types::{CheckRunEvent, IssueCommentEvent, PullRequestEvent},
};
use axum::{
    Json, Router,
    extract::State,
    http::{HeaderMap, HeaderValue, StatusCode},
    routing::{get, post},
};
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::{net::TcpListener, signal, sync::Mutex, time::Duration};
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
#[serde(default, rename_all = "kebab-case")]
pub struct ServerOptions {
    /// Port to bind to, defaults to 8080
    #[serde(default = "default_port")]
    pub port: u16,

    /// Optional ssl configuration for the server
    pub ssl: SSLOptions,

    /// Shared webhook secret for verifying the webhook sender
    pub webhook_secret: Option<String>,

    /// Refresh check runs periodically instead of on every webhook event
    /// This is useful for reducing the number of API calls to GitHub.
    /// When set to zero, periodic refresh is disabled.
    /// Unit is in seconds.
    #[serde(default = "Default::default")]
    pub periodic_refresh: u64,
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
            periodic_refresh: 0,
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

/// Job for refreshing check runs
#[derive(Debug, Ord, PartialEq, PartialOrd, Eq)]
struct Job {
    app_installation_id: u64,
    repo: String,
    commit: String,
}

/// HTTP Server for receiving webhook events from GitHub
pub struct Server {
    options: ServerOptions,
}

#[derive(Clone)]
struct ServerState {
    webhook_secret: Option<String>,
    github: Arc<Client>,
    job_queue: Arc<Mutex<Vec<Job>>>,
    use_job_queue: bool,
}

impl ServerState {
    /// Create a new server state with the given webhook secret and GitHub client
    fn new(webhook_secret: Option<String>, github: Client) -> Self {
        let github = Arc::new(github);
        Self {
            webhook_secret,
            github,
            job_queue: Arc::new(Mutex::new(Vec::new())),
            use_job_queue: false,
        }
    }

    /// Create a new pending job and add it to the job queue
    async fn new_job(&self, app_installation_id: u64, repo: &str, commit: &str) {
        let job = Job {
            app_installation_id,
            repo: repo.to_string(),
            commit: commit.to_string(),
        };
        let mut job_queue = self.job_queue.lock().await;
        job_queue.push(job);
    }

    /// Start a background task that periodically runs all jobs in the queue
    fn periodically_run_job_queue(&mut self, period: u64) {
        let job_queue = self.job_queue.clone();
        let github = self.github.clone();

        info!(
            "Periodic refresh of check runs enabled with a period of {} seconds",
            period,
        );

        self.use_job_queue = true;
        tokio::spawn(async move {
            let period = Duration::from_secs(period);
            loop {
                tokio::time::sleep(period).await;

                let mut job_queue = job_queue.lock().await;
                if job_queue.is_empty() {
                    continue;
                }

                deduplicate_jobs(job_queue.as_mut());

                info!("Running {} jobs in the queue", job_queue.len());

                for job in job_queue.drain(..) {
                    if let Err(e) = github
                        .refresh_check_run_status(job.app_installation_id, &job.repo, &job.commit)
                        .await
                    {
                        error!(
                            "Failed to refresh check run status for job: '{}' - '{}': {}",
                            job.repo, job.commit, e
                        );
                    }
                }
            }
        });
    }
}

impl Server {
    /// Create a new server with the given options and GitHub client
    pub fn new(options: ServerOptions) -> Self {
        Self { options }
    }

    /// Run the server
    /// Server will shutdown gracefully on Ctrl+C or SIGTERM
    pub async fn run(&self, github: Client) -> Result<(), Error> {
        let mut state = ServerState::new(self.options.webhook_secret.clone(), github);
        if self.options.periodic_refresh > 0 {
            state.periodically_run_job_queue(self.options.periodic_refresh);
        }
        let router = new_router(state);

        let addr = SocketAddr::from(([0, 0, 0, 0, 0, 0, 0, 0], self.options.port));
        info!("Starting server on {}", addr);

        if self.options.ssl.enabled {
            let listener =
                tls::TlsListener::bind(addr, &self.options.ssl.key, &self.options.ssl.cert)
                    .await
                    .map_err(|e| Error::BindPort(Box::new(e)))?;

            axum::serve(listener, router)
                .with_graceful_shutdown(shutdown_signal())
                .await
                .map_err(Error::Serve)
        } else {
            let listener = TcpListener::bind(addr)
                .await
                .map_err(|e| Error::BindPort(Box::new(e)))?;

            axum::serve(listener, router)
                .with_graceful_shutdown(shutdown_signal())
                .await
                .map_err(Error::Serve)
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
        "check_run" => handle_check_run_event(state.0, &payload).await,
        "pull_request" => handle_pull_request_event(&state.github, &payload).await,
        "issue_comment" => handle_issue_comment_event(&state.github, &payload).await,
        event => {
            let message = format!("Received unsupported event: {event}");
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
async fn handle_check_run_event(state: ServerState, payload: &str) -> (StatusCode, Json<Response>) {
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
        .is_some_and(|app| app.client_id == state.github.client_id())
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

    if state.use_job_queue {
        state
            .new_job(
                app_id,
                &payload.repository.full_name,
                &payload.check_run.head_sha,
            )
            .await;
        return (StatusCode::OK, Json(Response::new()));
    }

    match state
        .github
        .refresh_check_run_status(
            app_id,
            &payload.repository.full_name,
            &payload.check_run.head_sha,
        )
        .await
    {
        Ok(_) => (StatusCode::OK, Json(Response::new())),
        Err(e) => {
            error!("Failed to refresh check-run status: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(Response::error("Failed to refresh check-run status")),
            )
        }
    }
}

/// Handle webhook issue_comment events
async fn handle_issue_comment_event(
    client: &Client,
    payload: &str,
) -> (StatusCode, Json<Response>) {
    let payload: IssueCommentEvent = match serde_json::from_str(payload) {
        Ok(event) => event,
        Err(e) => {
            warn!("Failed to parse issue_comment event payload: {e}");
            return (
                StatusCode::BAD_REQUEST,
                Json(Response::error("Invalid issue_comment event payload")),
            );
        }
    };

    let app_id = match payload.installation {
        Some(installation) => installation.id,
        None => {
            warn!("Missing app installation id in issue_comment event");
            return (
                StatusCode::BAD_REQUEST,
                Json(Response::error("Missing app installation id")),
            );
        }
    };

    if payload.action != "created" {
        debug!(
            "Ignoring issue_comment event with action: {}",
            payload.action
        );
        return (StatusCode::OK, Json(Response::new()));
    }

    if !payload.comment.body.contains("/cerberus refresh") {
        debug!("Ignoring issue comment without '/cerberus' command");
        return (StatusCode::OK, Json(Response::new()));
    }
    info!(
        "Received issue_comment event for issue {}: {}",
        payload.issue.number, payload.comment.body
    );

    let commit = match client
        .get_pull_request_head_commit(app_id, &payload.repository.full_name, payload.issue.number)
        .await
    {
        Ok(commit) => commit,
        Err(e) => {
            error!("Failed to get pull request head commit: {e}");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(Response::error("Failed to get pull request head commit")),
            );
        }
    };

    if let Err(e) = client
        .refresh_check_run_status(app_id, &payload.repository.full_name, &commit)
        .await
    {
        error!("Failed to refresh check-run status: {e}");
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(Response::error("Failed to refresh check-run status")),
        );
    }

    (StatusCode::OK, Json(Response::new()))
}

/// Detailed status of the Webserver
#[derive(Debug, Serialize, Deserialize)]
pub struct Response {
    /// Status of the server.
    /// "ok" if everything is running fine, "error" if something is wrong.
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

/// Remove duplicates from job queue
fn deduplicate_jobs(job_queue: &mut Vec<Job>) {
    job_queue.sort();
    job_queue.dedup();
}
