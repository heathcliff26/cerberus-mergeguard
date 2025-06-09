use crate::testutils::{ExpectedRequests, MockGithubApiServer, TlsCertificate};
use crate::{client::Client, client::ClientOptions, types::*};
use std::collections::VecDeque;

use super::*;

#[tokio::test]
async fn ignore_own_check_run() {
    let test_body = include_str!("../types/testdata/own-check-run-event.json");

    let github = Client::new_for_testing(
        "test-client-id",
        "test-client-secret",
        "https://noops.example.com",
    );

    let (status, response) = handle_check_run_event(&github, test_body).await;
    if status != StatusCode::OK {
        panic!(
            "Should have ignored event and returned OK, got: {}, message={:?}",
            status, response
        );
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
        "Should return OK for refresh command, response: {:?}",
        response
    );
}

fn verify_webhook_ok_result() -> Result<(), (StatusCode, Json<Response>)> {
    Ok(())
}

fn verify_webhook_error_result(message: &str) -> Result<(), (StatusCode, Json<Response>)> {
    Err((StatusCode::FORBIDDEN, Json(Response::error(message))))
}
