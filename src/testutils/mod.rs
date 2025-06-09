use crate::config::Configuration;
use crate::types::*;
use axum::{
    Router,
    extract::State,
    http::{HeaderMap, Method, StatusCode, Uri},
};
use std::{collections::VecDeque, process::Command};
use std::{net::SocketAddr, sync::Arc};
use tokio::sync::{Mutex, watch};

type SharedState = Arc<Mutex<MockGithubApiServerState>>;

/// State of the mock server.
pub struct MockGithubApiServerState {
    expected_requests: VecDeque<ExpectedRequests>,
    /// The requests that were made to the server.
    pub requests: Vec<RecordedRequests>,
}

/// Recorded requests to the mock server.
pub struct RecordedRequests {
    pub headers: HeaderMap,
    pub method: String,
    pub uri: String,
    pub body: String,
}

/// Mock server for testing the github client.
/// Allows to set a series of expected requests and the associated responses.
/// The server will respond with the expected responses in the order they were set.
/// It will panic when a request is made but no responses are available.
/// Whenever an error occurs, the server will panic as well.
pub struct MockGithubApiServer {
    pub state: SharedState,
    shutdown_tx: watch::Sender<()>,
    shutdown_rx: watch::Receiver<()>,
}

impl MockGithubApiServer {
    /// Create a new mock server with the serving the expected requests.
    pub fn new(expected_requests: VecDeque<ExpectedRequests>) -> Self {
        let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(());
        let requests = Vec::new();
        let state = MockGithubApiServerState {
            expected_requests,
            requests,
        };
        Self {
            state: Arc::new(Mutex::new(state)),
            shutdown_tx,
            shutdown_rx,
        }
    }
    /// Start the mock server and return the address it is listening on.
    /// This will panic if the server fails to start.
    pub async fn start(&self) -> String {
        let router: Router<()> = Router::new()
            .fallback(handle_request)
            .with_state(self.state.clone());
        let addr = SocketAddr::from(([0, 0, 0, 0, 0, 0, 0, 1], 0));
        let listener = tokio::net::TcpListener::bind(addr)
            .await
            .expect("Failed to bind mock server");

        let addr = format!(
            "http://localhost:{}",
            listener
                .local_addr()
                .expect("Listener should have addr")
                .port()
        )
        .to_string();

        let mut shutdown_rx = self.shutdown_rx.clone();
        let shutdown_signal = async move {
            shutdown_rx
                .changed()
                .await
                .expect("Failed to receive shutdown signal")
        };

        tokio::spawn(async move {
            axum::serve(listener, router)
                .with_graceful_shutdown(shutdown_signal)
                .await
                .expect("Failed to run mock server");
        });

        addr
    }
}

impl Drop for MockGithubApiServer {
    fn drop(&mut self) {
        self.shutdown_tx
            .send(())
            .expect("Failed to send shutdown signal");
    }
}

/// An enum for the expected requests and the reponses that should be given.
/// All responses have the status code and the response body that should be returned.
pub enum ExpectedRequests {
    GetInstallationToken(StatusCode, TokenResponse),
    GetCheckRuns(StatusCode, CheckRunsResponse),
    CreateCheckRun(StatusCode, CheckRun),
    UpdateCheckRun(StatusCode, CheckRun),
    GetPullRequest(StatusCode, PullRequestResponse),
}

impl ExpectedRequests {
    /// Returns the provided status code and the response body as a tuple.
    pub fn response(&self) -> (StatusCode, String) {
        match self {
            ExpectedRequests::GetInstallationToken(status, token_response) => (
                *status,
                serde_json::to_string(&token_response).expect("Failed to serialize token response"),
            ),
            ExpectedRequests::GetCheckRuns(status, check_runs_response) => (
                *status,
                serde_json::to_string(&check_runs_response)
                    .expect("Failed to serialize token response"),
            ),
            ExpectedRequests::CreateCheckRun(status, check_run) => (
                *status,
                serde_json::to_string(&check_run).expect("Failed to serialize token response"),
            ),
            ExpectedRequests::UpdateCheckRun(status, check_run) => (
                *status,
                serde_json::to_string(&check_run).expect("Failed to serialize token response"),
            ),
            ExpectedRequests::GetPullRequest(status, pull_request_response) => (
                *status,
                serde_json::to_string(&pull_request_response)
                    .expect("Failed to serialize pull request response"),
            ),
        }
    }
}

async fn handle_request(
    method: Method,
    headers: HeaderMap,
    uri: Uri,
    State(state): State<SharedState>,
    payload: String,
) -> (StatusCode, String) {
    let mut state = state.lock().await;

    let record = RecordedRequests {
        headers,
        method: method.to_string(),
        uri: uri.to_string(),
        body: payload,
    };

    state.requests.push(record);

    if let Some(expected) = state.expected_requests.pop_front() {
        expected.response()
    } else {
        panic!("Unexpected request: {}", uri);
    }
}

/// Temporary configuration file for testing purposes.
/// Will be deleted when it goes out of scope.
pub struct TmpTestConfigFile {
    pub file: String,
}

impl TmpTestConfigFile {
    /// Create a new temporary configuration file with the given content.
    pub fn new(config: Configuration) -> Self {
        let config =
            serde_yaml::to_string(&config).expect("Failed to serialize configuration to YAML");

        let suffix: u64 = rand::random();

        let file = std::env::temp_dir()
            .join(format!("cerberus_test_config_{}.yaml", suffix))
            .to_str()
            .expect("Failed to convert path to string")
            .to_string();
        std::fs::write(&file, config).expect("Failed to write configuration to file");

        Self { file }
    }
}

impl Drop for TmpTestConfigFile {
    fn drop(&mut self) {
        std::fs::remove_file(&self.file).expect("Failed to remove temporary config file");
    }
}

/// Randomly generated self-signed TLS certificate and key pair.
/// Will be cleaned up when it goes out of scope.
pub struct TlsCertificate {
    pub key: String,
    pub crt: String,
}

impl TlsCertificate {
    /// Create a self signed TLS certificate and key pair.
    pub fn create(name: &str) -> Self {
        let key = format!("{name}.key").to_string();
        let crt = format!("{name}.crt").to_string();
        println!("Creating TLS certificate '{crt}' and key '{key}' ");
        let output = Command::new("openssl")
            .args([
                "req",
                "-x509",
                "-nodes",
                "-days",
                "1",
                "-newkey",
                "rsa:2048",
                "-keyout",
                &key,
                "-out",
                &crt,
                "-subj",
                "/CN=localhost",
            ])
            .output()
            .expect("Failed to execute openssl command");

        if !output.status.success() {
            panic!(
                "Failed to create TLS certificate: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
        let output = Command::new("chmod")
            .args(["644", &key])
            .output()
            .expect("Failed to execute chmod command");
        if !output.status.success() {
            panic!(
                "Failed to set permissions for TLS key: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }

        println!("TLS certificate created successfully.");
        TlsCertificate { key, crt }
    }
    /// Returns the certificate as a reqwest::tls::Certificate
    pub fn certificate(&self) -> reqwest::tls::Certificate {
        let cert_data = std::fs::read(&self.crt).expect("Failed to read TLS certificate file");
        reqwest::tls::Certificate::from_pem(&cert_data)
            .expect("Failed to create TLS certificate from PEM data")
    }
}

impl Drop for TlsCertificate {
    fn drop(&mut self) {
        println!("Removing TLS certificate: {}", self.crt);

        let res_key = std::fs::remove_file(&self.key);
        let res_crt = std::fs::remove_file(&self.crt);
        res_key.expect("Failed to remove TLS key file");
        res_crt.expect("Failed to remove TLS certificate file");

        println!("TLS certificate removed successfully.");
    }
}
