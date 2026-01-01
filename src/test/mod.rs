use crate::client::ClientOptions;
use crate::config::Configuration;
use crate::server::ServerOptions;
use crate::testutils::*;
use crate::types::*;
use crate::{Command, GlobalOpts};
use axum::http::{HeaderMap, StatusCode};
use reqwest::header;
use std::collections::VecDeque;

#[tokio::test]
async fn pull_request_event() {
    let commit = "test_commit";
    let mut check_run = CheckRun::new(commit);
    check_run.id = 12345;
    let token = "test_token";

    let expected_requests = VecDeque::from(vec![
        ExpectedRequests::GetInstallationToken(
            StatusCode::OK,
            TokenResponse {
                token: token.to_string(),
                expires_at: chrono::Utc::now() + chrono::Duration::seconds(3600),
            },
        ),
        ExpectedRequests::CreateCheckRun(StatusCode::OK, check_run),
    ]);

    let server = MockGithubApiServer::new(expected_requests);
    let api_addr = server.start().await;

    let client_id = "test_client_id";
    let certificate = TlsCertificate::create();
    let mut server_options = ServerOptions::default();
    server_options.port = 8900;
    let config = Configuration {
        log_level: "debug".to_string(),
        github: ClientOptions {
            api: api_addr.clone(),
            client_id: client_id.to_string(),
            private_key: certificate.key.clone(),
        },
        server: server_options,
    };
    let config = TmpTestConfigFile::new(config);

    let app = crate::App {
        global_opts: GlobalOpts {
            log: None,
            config: config.file.clone(),
        },
        command: Command::Server,
    };

    tokio::spawn(async move {
        app.run().await.expect("Failed to run the server");
    });

    let pull_request_event = PullRequestEvent {
        action: "opened".to_string(),
        number: 1,
        pull_request: PullRequest {
            title: "Test Pull Request".to_string(),
            head: BranchRef {
                label: "base_label".to_string(),
                sha: "base_sha".to_string(),
                ref_field: "base_ref".to_string(),
                repo: Repo {
                    id: 12345678,
                    name: "test_repo".to_string(),
                    full_name: "test_user/test_repo".to_string(),
                },
            },
            number: 1,
        },
        installation: Some(Installation { id: 123456 }),
        repository: Repo {
            id: 12345678,
            name: "test_repo".to_string(),
            full_name: "test_user/test_repo".to_string(),
        },
    };
    let response = reqwest::Client::new()
        .post("http://localhost:8900/webhook")
        .header("X-GitHub-Event", "pull_request")
        .json(&pull_request_event)
        .send()
        .await
        .expect("Failed to send pull request event");

    assert_eq!(
        response.status(),
        StatusCode::OK,
        "Webhook call should succeed"
    );

    let state = server.state.lock().await;

    // Check that the token request was made
    let request = state.requests.first().expect("Should have token request");
    assert_eq!("POST", request.method.as_str(), "Method should be POST");
    assert_eq!(
        "/app/installations/123456/access_tokens",
        request.uri.as_str(),
        "URI should match"
    );
    should_have_common_headers(request.headers.clone());

    // Check that the check-run creation request was made
    let request = state
        .requests
        .get(1)
        .expect("Should have check-run request");
    assert_eq!("POST", request.method.as_str(), "Method should be POST");
    assert_eq!(
        "/repos/test_user/test_repo/check-runs",
        request.uri.as_str(),
        "URI should match"
    );
    should_have_common_headers(request.headers.clone());
}

#[tokio::test]
async fn check_run_event_incomplete() {
    let commit = "test_commit";
    let mut check_run = CheckRun::new(commit);
    check_run.id = 12345;
    let token = "test_token";

    let mut expected_run = CheckRun::new(commit);
    expected_run.id = 98765;
    expected_run.status = "queued".to_string();

    let check_runs_response = CheckRunsResponse {
        total_count: 1,
        check_runs: vec![expected_run],
    };

    let expected_requests = VecDeque::from(vec![
        ExpectedRequests::GetInstallationToken(
            StatusCode::OK,
            TokenResponse {
                token: token.to_string(),
                expires_at: chrono::Utc::now() + chrono::Duration::seconds(3600),
            },
        ),
        ExpectedRequests::GetCheckRuns(StatusCode::OK, check_runs_response),
        ExpectedRequests::UpdateCheckRun(StatusCode::OK, check_run.clone()),
    ]);

    let server = MockGithubApiServer::new(expected_requests);
    let api_addr = server.start().await;

    let client_id = "test_client_id";
    let certificate = TlsCertificate::create();
    let mut server_options = ServerOptions::default();
    server_options.port = 8901;
    let config = Configuration {
        log_level: "debug".to_string(),
        github: ClientOptions {
            api: api_addr.clone(),
            client_id: client_id.to_string(),
            private_key: certificate.key.clone(),
        },
        server: server_options,
    };
    let config = TmpTestConfigFile::new(config);

    let app = crate::App {
        global_opts: GlobalOpts {
            log: None,
            config: config.file.clone(),
        },
        command: Command::Server,
    };

    tokio::spawn(async move {
        app.run().await.expect("Failed to run the server");
    });

    let check_run_event = CheckRunEvent {
        action: "created".to_string(),
        check_run,
        installation: Some(Installation { id: 123456 }),
        repository: Repo {
            id: 12345678,
            name: "test_repo".to_string(),
            full_name: "test_user/test_repo".to_string(),
        },
    };

    let response = reqwest::Client::new()
        .post("http://localhost:8901/webhook")
        .header("X-GitHub-Event", "check_run")
        .json(&check_run_event)
        .send()
        .await
        .expect("Failed to send pull request event");

    assert_eq!(
        response.status(),
        StatusCode::OK,
        "Webhook call should succeed"
    );

    let state = server.state.lock().await;

    // Check that the token request was made
    let request = state.requests.first().expect("Should have token request");
    assert_eq!("POST", request.method.as_str(), "Method should be POST");
    assert_eq!(
        "/app/installations/123456/access_tokens",
        request.uri.as_str(),
        "URI should match"
    );
    should_have_common_headers(request.headers.clone());

    let request = state
        .requests
        .get(1)
        .expect("Should have get check-runs request");
    assert_eq!("GET", request.method.as_str(), "Method should be GET");
    assert_eq!(
        "/repos/test_user/test_repo/commits/test_commit/check-runs",
        request.uri.as_str(),
        "URI should match"
    );
    should_have_common_headers(request.headers.clone());
}

#[tokio::test]
async fn check_run_event_ignore_own() {
    let expected_requests = VecDeque::from(Vec::new());

    let server = MockGithubApiServer::new(expected_requests);
    let api_addr = server.start().await;

    let client_id = "test_client_id";
    let certificate = TlsCertificate::create();
    let mut server_options = ServerOptions::default();
    server_options.port = 8902;
    let config = Configuration {
        log_level: "debug".to_string(),
        github: ClientOptions {
            api: api_addr.clone(),
            client_id: client_id.to_string(),
            private_key: certificate.key.clone(),
        },
        server: server_options,
    };
    let config = TmpTestConfigFile::new(config);

    let app = crate::App {
        global_opts: GlobalOpts {
            log: None,
            config: config.file.clone(),
        },
        command: Command::Server,
    };

    tokio::spawn(async move {
        app.run().await.expect("Failed to run the server");
    });

    let commit = "test_commit";
    let mut check_run = CheckRun::new(commit);
    check_run.id = 12345;
    check_run.app = Some(App {
        id: 123456,
        client_id: client_id.to_string(),
        slug: "cerberus-mergeguard".to_string(),
        name: "Cerberus MergeGuard".to_string(),
    });

    let check_run_event = CheckRunEvent {
        action: "created".to_string(),
        check_run,
        installation: Some(Installation { id: 123456 }),
        repository: Repo {
            id: 12345678,
            name: "test_repo".to_string(),
            full_name: "test_user/test_repo".to_string(),
        },
    };

    let response = reqwest::Client::new()
        .post("http://localhost:8902/webhook")
        .header("X-GitHub-Event", "check_run")
        .json(&check_run_event)
        .send()
        .await
        .expect("Failed to send pull request event");

    assert_eq!(
        response.status(),
        StatusCode::OK,
        "Webhook call should succeed"
    );

    // No requests should be made and the server panics anyway on unexpected requests
}

/// Asserts that the common headers, including the token, are set.
fn should_have_common_headers(headers: HeaderMap) {
    assert!(
        headers.contains_key(header::ACCEPT),
        "Missing Accept header"
    );
    assert!(
        headers.contains_key("x-github-api-version"),
        "Missing x-github-api-version header"
    );
    assert!(
        headers.contains_key(header::USER_AGENT),
        "Missing User-Agent header"
    );
    assert!(
        headers.contains_key(header::AUTHORIZATION),
        "Missing Authorization header"
    );
}
