use axum::http::StatusCode;
use std::collections::{HashMap, VecDeque};
use tokio::sync::Mutex;

use super::*;
use crate::testutils::{ExpectedRequests, MockGithubApiServer, TlsCertificate};
use crate::types::App;

#[tokio::test]
async fn get_token_from_cache() {
    let expected_requests = VecDeque::new();
    let app_id = 12345;
    let mut cache = HashMap::new();
    cache.insert(
        app_id,
        TokenResponse {
            token: "test_token".to_string(),
            expires_at: chrono::Utc::now() + chrono::Duration::seconds(3600),
        },
    );

    let api_server = MockGithubApiServer::new(expected_requests);
    let addr = api_server.start().await;
    let mut client = Client::new_for_testing("testid", "testsecret", &addr);
    client.token_cache = Mutex::new(cache);

    let token = client.get_token(app_id).await;
    match token {
        Ok(token) => {
            assert_eq!("test_token", token, "Token should match the cached value");
        }
        Err(e) => panic!("Failed to get token from cache: {e}"),
    }
}

#[tokio::test]
async fn get_new_token() {
    let app_id = 12345;
    let expected_requests = VecDeque::from(vec![ExpectedRequests::GetInstallationToken(
        StatusCode::OK,
        TokenResponse {
            token: "test_token".to_string(),
            expires_at: chrono::Utc::now() + chrono::Duration::seconds(3600),
        },
    )]);

    let api_server = MockGithubApiServer::new(expected_requests);
    let addr = api_server.start().await;
    let certificate = TlsCertificate::create("/tmp/cerberus-mergeguard_new_token");
    let client = ClientOptions {
        client_id: "testid".to_string(),
        private_key: certificate.key.clone(),
        api: addr.clone(),
    };
    let client = Client::build(client).expect("Failed to build client for testing");

    let token = client.get_token(app_id).await;
    match token {
        Ok(token) => {
            assert_eq!("test_token", token, "Token should match the cached value");
        }
        Err(e) => panic!("Failed to get token from cache: {e}"),
    }
    let cache = client.token_cache.lock().await;
    assert_eq!(1, cache.len(), "Cache should contain one token");
}

#[tokio::test]
async fn get_new_token_when_expired() {
    let app_id = 12345;
    let expected_requests = VecDeque::from(vec![ExpectedRequests::GetInstallationToken(
        StatusCode::OK,
        TokenResponse {
            token: "test_token".to_string(),
            expires_at: chrono::Utc::now() + chrono::Duration::seconds(3600),
        },
    )]);

    let api_server = MockGithubApiServer::new(expected_requests);
    let addr = api_server.start().await;
    let certificate = TlsCertificate::create("/tmp/get_new_token_when_expired");
    let client = ClientOptions {
        client_id: "testid".to_string(),
        private_key: certificate.key.clone(),
        api: addr.clone(),
    };
    let mut client = Client::build(client).expect("Failed to build client for testing");

    let mut cache = HashMap::new();
    cache.insert(
        app_id,
        TokenResponse {
            token: "expired_token".to_string(),
            expires_at: chrono::Utc::now() - chrono::Duration::seconds(1),
        },
    );
    client.token_cache = Mutex::new(cache);

    let token = client.get_token(app_id).await;
    match token {
        Ok(token) => {
            assert_eq!("test_token", token, "Token should match the cached value");
        }
        Err(e) => panic!("Failed to get token from cache: {e}"),
    }
    let cache = client.token_cache.lock().await;
    assert_eq!(1, cache.len(), "Cache should contain one token");
    let cached_token = cache.get(&app_id).expect("Token should be in cache");
    assert_eq!(
        "test_token", cached_token.token,
        "Cached token should match the new token"
    );
}

#[tokio::test]
async fn failed_to_get_token() {
    let app_id = 12345;
    let expected_requests = VecDeque::from(vec![ExpectedRequests::GetInstallationToken(
        StatusCode::INTERNAL_SERVER_ERROR,
        TokenResponse {
            token: "invalid_token".to_string(),
            expires_at: chrono::Utc::now() + chrono::Duration::seconds(3600),
        },
    )]);

    let api_server = MockGithubApiServer::new(expected_requests);
    let addr = api_server.start().await;
    let certificate = TlsCertificate::create("/tmp/failed_to_get_token");
    let client = ClientOptions {
        client_id: "testid".to_string(),
        private_key: certificate.key.clone(),
        api: addr.clone(),
    };
    let client = Client::build(client).expect("Failed to build client for testing");

    if let Ok(token) = client.get_token(app_id).await {
        panic!("Expected an error, but got token: {token}");
    }
}

#[test]
fn test_overall_check_status_empty_list() {
    let client = Client::new_for_testing("own-app-id", "some-secret", "some-addr");

    let (count, own_check_run) = client.overall_check_status(&Vec::new());
    assert_eq!(0, count, "Should not count any check runs");
    assert!(own_check_run.is_none(), "Should not have any own check run");
}

#[test]
fn test_overall_check_status() {
    let client = Client::new_for_testing("own-app-id", "some-secret", "some-addr");
    let check_runs = vec![
        create_test_check_run(
            "commit1",
            "check-1",
            "completed",
            Some(CHECK_RUN_CONCLUSION.to_string()),
            "other-app-id",
        ),
        create_test_check_run(
            "commit1",
            "check-2",
            "completed",
            Some(CHECK_RUN_SKIPPED.to_string()),
            "other-app-id",
        ),
        create_test_check_run("commit1", "check-3", "pending", None, "other-app-id"),
        create_test_check_run("commit1", "check-4", "completed", None, "other-app-id"),
        create_test_check_run(
            "commit1",
            "check-5",
            "completed",
            Some("other-conclusion".to_string()),
            "other-app-id",
        ),
    ];

    let (count, own_check_run) = client.overall_check_status(&check_runs);
    assert_eq!(3, count, "Should count unfinished and failed check runs");
    assert!(own_check_run.is_none(), "Should not have any own check run");
}

#[test]
fn test_overall_check_status_multiple_own_check_runs() {
    let client = Client::new_for_testing("own-app-id", "some-secret", "some-addr");
    let check_runs = vec![
        create_test_check_run(
            "commit1",
            "own-check-1",
            "completed",
            Some("success".to_string()),
            &client.client_id,
        ),
        create_test_check_run(
            "commit1",
            "own-check-2",
            "completed",
            Some("failure".to_string()),
            &client.client_id,
        ),
        create_test_check_run(
            "commit1",
            "other-check",
            "completed",
            Some("failure".to_string()),
            "other-app-id",
        ),
        create_test_check_run(
            "commit1",
            "own-check-3",
            "completed",
            Some("failure".to_string()),
            &client.client_id,
        ),
    ];

    let (count, own_check_run) = client.overall_check_status(&check_runs);
    assert_eq!(1, count, "Should count only other apps check runs");
    let own_check_run = own_check_run.expect("Should have own check run");
    assert_eq!(
        "own-check-1", own_check_run.name,
        "Should pick the first own check run"
    );
}

fn create_test_check_run(
    commit: &str,
    name: &str,
    status: &str,
    conclusion: Option<String>,
    client_id: &str,
) -> CheckRun {
    let mut check_run = CheckRun::new(commit);
    check_run.name = name.to_string();
    check_run.status = status.to_string();
    check_run.conclusion = conclusion;
    check_run.app = Some(App {
        id: 0,
        client_id: client_id.to_string(),
        name: "Test App".to_string(),
        slug: "".to_string(),
    });
    check_run
}
