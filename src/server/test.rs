use crate::testutils::{ExpectedRequests, MockGithubApiServer, TlsCertificate};
use crate::{client::Client, client::ClientOptions, types::*};
use std::collections::VecDeque;
use tokio::time::Duration;

use super::*;

#[tokio::test]
async fn ignore_own_check_run() {
    let test_body = include_str!("../types/testdata/own-check-run-event.json");

    let github = Client::new_for_testing(
        "test-client-id",
        "test-client-secret",
        "https://noops.example.com",
    );

    let (status, response) =
        handle_check_run_event(ServerState::new(None, github), test_body).await;
    if status != StatusCode::OK {
        panic!("Should have ignored event and returned OK, got: {status}, message={response:?}");
    }
}

macro_rules! verify_webhook_test {
    ($($name:ident: $value:expr,)*) => {
    $(
        #[test]
        fn $name() {
            let (signature, secret, payload, res) = $value;

            let signature: Option<HeaderValue> = match signature {
                Some(sig) => Some(HeaderValue::from_str(sig).unwrap()),
                None => None,
            };

            let output = verify_webhook(signature.as_ref(), secret, payload);

            match res {
                Ok(()) => assert!(output.is_ok(), "Expected Ok, got: {:?}", output),
                Err(res) => {
                    if let Err((status, message)) = output {
                        let (res_status, res_message) = res;
                        assert_eq!(res_status, status, "Status code mismatch");
                        assert_eq!(res_message.message, message.message, "Wrong message");
                    } else {
                        panic!("Expected error, got Ok");
                    }
                },
            };
        }
    )*
    }
}

verify_webhook_test! {
    verify_webhook_valid_signature: (
        Some("sha256=2f94a757d2246073e26781d117ce0183ebd87b4d66c460494376d5c37d71985b"),
        Some("test-secret"),
        "test payload",
        verify_webhook_ok_result(),
    ),
    verify_webhook_invalid_signature: (
        Some("sha256=0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"),
        Some("test-secret"),
        "test payload",
        verify_webhook_error_result("Invalid webhook signature"),
    ),
    verify_webhook_malformed_signature: (
        Some("sha256=invalid-signature"),
        Some("test-secret"),
        "test payload",
        verify_webhook_error_result("Invalid X-Hub-Signature-256 header"),
    ),
    verify_webhook_missing_signature: (
        None,
        Some("test-secret"),
        "test payload",
        verify_webhook_error_result("Missing X-Hub-Signature-256 header"),
    ),
    verify_webhook_no_secret: (
        Some("sha256=invalid-signature"),
        None,
        "test payload",
        verify_webhook_ok_result(),
    ),
    verify_webhook_no_secret_or_signature: (
        None,
        None,
        "test payload",
        verify_webhook_ok_result(),
    ),

}

#[tokio::test]
async fn ignore_webhook_comment_without_command() {
    let payload = include_str!("testdata/issue-comment-event-ignored.json");

    let mut headers = HeaderMap::new();
    headers.insert("X-GitHub-Event", HeaderValue::from_static("issue_comment"));

    let state = ServerState::new(
        None,
        Client::new_for_testing("testid", "testsecret", "https://noops.example.com"),
    );
    let state = State(state);

    let (status, _) = webhook_handler(headers, state, payload.to_string()).await;

    assert_eq!(
        StatusCode::OK,
        status,
        "Should return OK for ignored comment"
    );
}

#[tokio::test]
async fn handle_webhook_comment_refresh_command() {
    // Prepare the payload for the refresh command
    let payload = include_str!("testdata/issue-comment-event-refresh.json");

    // Prepare expected requests for the mock GitHub API
    let token = "test_token";
    let commit = "abc123";
    let client_id = "test-client-id";
    let mut own_run = CheckRun::new(commit);
    own_run.id = 123456;
    // Status should be success, so the server does not attempt to update it.
    own_run.update_status(0);
    own_run.app = Some(App {
        id: 123456,
        client_id: client_id.to_string(),
        slug: "test-app".to_string(),
        name: "test-app".to_string(),
    });
    let expected_requests = VecDeque::from(vec![
        ExpectedRequests::GetInstallationToken(
            StatusCode::OK,
            TokenResponse {
                token: token.to_string(),
                expires_at: chrono::Utc::now() + chrono::Duration::seconds(3600),
            },
        ),
        ExpectedRequests::GetPullRequest(
            StatusCode::OK,
            PullRequestResponse {
                id: 123456,
                number: 42,
                head: BranchRef {
                    label: "feature-branch".to_string(),
                    ref_field: "feature-branch".to_string(),
                    sha: commit.to_string(),
                    repo: Repo {
                        id: 7890,
                        name: "test-repo".to_string(),
                        full_name: "test-org/test-repo".to_string(),
                    },
                },
            },
        ),
        ExpectedRequests::GetCheckRuns(
            StatusCode::OK,
            CheckRunsResponse {
                total_count: 1,
                check_runs: vec![own_run],
            },
        ),
    ]);

    // Start the mock server
    let server = MockGithubApiServer::new(expected_requests);
    let api_addr = server.start().await;

    // Prepare server state and headers
    let certificate = TlsCertificate::create(
        "/tmp/cerberus-mergeguard_handle_webhook_comment_refresh_command_test",
    );
    let client_options = ClientOptions {
        client_id: client_id.to_string(),
        private_key: certificate.key.to_string(),
        api: api_addr.to_string(),
    };
    let github = Client::build(client_options).expect("Failed to build GitHub client");
    let state = ServerState::new(None, github);
    let state = State(state);

    let mut headers = HeaderMap::new();
    headers.insert("X-GitHub-Event", HeaderValue::from_static("issue_comment"));

    // Call the webhook handler
    let (status, response) = webhook_handler(headers, state, payload.to_string()).await;

    // Assert the webhook was handled successfully
    assert_eq!(
        StatusCode::OK,
        status,
        "Should return OK for refresh command, response: {response:?}"
    );
}

fn verify_webhook_ok_result() -> Result<(), (StatusCode, Json<Response>)> {
    Ok(())
}

fn verify_webhook_error_result(message: &str) -> Result<(), (StatusCode, Json<Response>)> {
    Err((StatusCode::FORBIDDEN, Json(Response::error(message))))
}

#[tokio::test]
async fn webhook_check_run_job_queue() {
    // Prepare the payload for the refresh command
    let payload = include_str!("testdata/check-run-event.json");

    // Start the mock server
    let server = MockGithubApiServer::new(VecDeque::new());
    let api_addr = server.start().await;

    // Prepare server state and headers
    let certificate =
        TlsCertificate::create("/tmp/cerberus-mergeguard_webhook_check_run_job_queue");
    let client_options = ClientOptions {
        client_id: "test-client-id".to_string(),
        private_key: certificate.key.to_string(),
        api: api_addr.to_string(),
    };
    let github = Client::build(client_options).expect("Failed to build GitHub client");
    let mut state = ServerState::new(None, github);
    state.use_job_queue = true;
    let state = State(state);

    let mut headers = HeaderMap::new();
    headers.insert("X-GitHub-Event", HeaderValue::from_static("check_run"));

    // Call the webhook handler
    let (status, response) = webhook_handler(headers, state.clone(), payload.to_string()).await;

    // Assert the webhook was handled successfully
    assert_eq!(
        StatusCode::OK,
        status,
        "Should return OK for refresh command, response: {response:?}"
    );

    let job_queue = state.0.job_queue.lock().await;

    assert_eq!(1, job_queue.len(), "Job queue should have one job");
}

#[test]
fn duplicate_jobs() {
    let mut job_queue = Vec::new();

    job_queue.push(Job {
        app_installation_id: 1,
        repo: "test-org/test-repo".to_string(),
        commit: "abc123".to_string(),
    });
    job_queue.push(Job {
        app_installation_id: 1,
        repo: "test-org/new-test-repo".to_string(),
        commit: "abc123".to_string(),
    });
    job_queue.push(Job {
        app_installation_id: 1,
        repo: "test-org/new-test-repo".to_string(),
        commit: "123456".to_string(),
    });
    job_queue.push(Job {
        app_installation_id: 3,
        repo: "test-org/test-repo".to_string(),
        commit: "abc123".to_string(),
    });
    job_queue.push(Job {
        app_installation_id: 2,
        repo: "test-org/test-repo".to_string(),
        commit: "abc123".to_string(),
    });
    job_queue.push(Job {
        app_installation_id: 3,
        repo: "test-org/test-repo".to_string(),
        commit: "abc123".to_string(),
    });
    job_queue.push(Job {
        app_installation_id: 1,
        repo: "test-org/new-test-repo".to_string(),
        commit: "abc123".to_string(),
    });
    job_queue.push(Job {
        app_installation_id: 1,
        repo: "test-org/new-test-repo".to_string(),
        commit: "123456".to_string(),
    });

    deduplicate_jobs(&mut job_queue);

    assert_eq!(5, job_queue.len(), "Job queue should have 5 unique jobs");
}

#[tokio::test]
async fn run_periodic_job_queue() {
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

    let certificate = TlsCertificate::create("/tmp/cerberus-mergeguard_run_periodic_job_queue");
    let client_options = ClientOptions {
        client_id: "test-client".to_string(),
        private_key: certificate.key.to_string(),
        api: api_addr.to_string(),
    };
    let github = Client::build(client_options).expect("Failed to build GitHub client");

    let mut state = ServerState::new(None, github);
    state.new_job(12345, "testorg/testrepo", commit).await;
    state.periodically_run_job_queue(1);

    for i in 0..10 {
        tokio::time::sleep(Duration::from_secs(1)).await;

        if state.job_queue.lock().await.is_empty() {
            break;
        }
        if i == 9 {
            panic!("Job queue did not empty after 10 second");
        }
    }

    let requests = &server.state.lock().await.requests;
    assert_eq!(3, requests.len(), "Should have made 3 requests");
}
