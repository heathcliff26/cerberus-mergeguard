use axum::http::StatusCode;
use std::collections::{HashMap, VecDeque};
use tokio::sync::Mutex;

use super::*;
use crate::testutils::{ExpectedRequests, MockGithubApiServer, TlsCertificate};

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
    let certificate = TlsCertificate::create("/tmp/get_new_token_when_expired");
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
