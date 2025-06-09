use crate::error::Error;
use crate::{types::*, version};
use reqwest::{Client, header, header::HeaderMap, header::HeaderName, header::HeaderValue};
use tracing::{debug, info};

/// Get an installation token for the GitHub App.
/// API endpoint: POST /app/installations/{installation_id}/access_tokens
pub async fn get_installation_token(
    endpoint: &str,
    token: &str,
    installation_id: u64,
) -> Result<TokenResponse, Error> {
    let url = format!("{endpoint}/app/installations/{installation_id}/access_tokens");
    info!("Fetching installation token from '{url}'");

    let client = new_client_with_common_headers(token)?;
    let response = send_request(client.post(&url)).await?;

    let token: TokenResponse = response
        .json()
        .await
        .map_err(|e| Error::Parse("get_installation_token", Box::new(e)))?;

    Ok(token)
}

/// Fetch all check runs for a commit.
/// API endpoint: GET /repos/{owner}/{repo}/commits/{ref}/check-runs
pub async fn get_check_runs(
    endpoint: &str,
    token: &str,
    repo: &str,
    commit: &str,
) -> Result<Vec<CheckRun>, Error> {
    let url = format!("{endpoint}/repos/{repo}/commits/{commit}/check-runs");
    info!("Fetching check runs from '{url}'");

    let client = new_client_with_common_headers(token)?;
    let response = send_request(client.get(&url)).await?;
    let response = receive_body(response).await?;

    let check_runs: CheckRunsResponse = match serde_json::from_str(&response) {
        Ok(check_runs) => check_runs,
        Err(e) => {
            debug!("Response body: '{}'", response);
            return Err(Error::Parse("get_check_runs", Box::new(e)));
        }
    };

    Ok(check_runs.check_runs)
}

/// Create a check run for a specific commit.
/// API endpoint: POST /repos/{owner}/{repo}/check-runs
pub async fn create_check_run(
    endpoint: &str,
    token: &str,
    repo: &str,
    payload: &CheckRun,
) -> Result<(), Error> {
    let url = format!("{endpoint}/repos/{repo}/check-runs");
    info!("Creating check-run for '{}' at '{url}'", payload.head_sha);

    let client = new_client_with_common_headers(token)?;
    let response = send_request(client.post(&url).json(payload)).await?;
    let response = receive_body(response).await?;

    match serde_json::from_str::<CheckRun>(&response) {
        Ok(check_run) => {
            info!(
                "Created check-run '{}' for commit '{}'",
                check_run.id, check_run.head_sha,
            );
            Ok(())
        }
        Err(e) => {
            debug!("Response body: '{}'", response);
            Err(Error::Parse("create_check_run", Box::new(e)))
        }
    }
}

/// Update a check run for a specific commit.
/// API endpoint: PATCH /repos/{owner}/{repo}/check-runs/{check_run_id}
pub async fn update_check_run(
    endpoint: &str,
    token: &str,
    repo: &str,
    payload: &CheckRun,
) -> Result<(), Error> {
    let url = format!("{endpoint}/repos/{repo}/check-runs/{}", payload.id);
    info!("Updating check-run for '{}' at '{url}'", payload.head_sha);

    let client = new_client_with_common_headers(token)?;
    let response = send_request(client.patch(&url).json(payload)).await?;
    let response = receive_body(response).await?;

    match serde_json::from_str::<CheckRun>(&response) {
        Ok(check_run) => {
            info!(
                "Updated check-run '{}' for commit '{}'",
                check_run.id, check_run.head_sha,
            );
            Ok(())
        }
        Err(e) => {
            debug!("Response body: '{}'", response);
            Err(Error::Parse("update_check_run", Box::new(e)))
        }
    }
}

/// Get the current status of a pull request.
/// API endpoint: GET /repos/{owner}/{repo}/pulls/{pull_number}
pub async fn get_pull_request(
    endpoint: &str,
    token: &str,
    repo: &str,
    pull_number: u64,
) -> Result<PullRequestResponse, Error> {
    let url = format!("{endpoint}/repos/{repo}/pulls/{pull_number}");
    info!("Fetching pull request from '{url}'");

    let client = new_client_with_common_headers(token)?;
    let response = send_request(client.get(&url)).await?;
    let response = receive_body(response).await?;

    match serde_json::from_str::<PullRequestResponse>(&response) {
        Ok(pull_request) => Ok(pull_request),
        Err(e) => {
            debug!("Response body: '{}'", response);
            Err(Error::Parse("get_pull_request", Box::new(e)))
        }
    }
}

fn new_client_with_common_headers(token: &str) -> Result<Client, Error> {
    let mut headers = HeaderMap::new();
    headers.insert(
        header::ACCEPT,
        HeaderValue::from_static("application/vnd.github+json"),
    );
    headers.insert(
        HeaderName::from_static("x-github-api-version"),
        HeaderValue::from_static("2022-11-28"),
    );
    headers.insert(header::USER_AGENT, HeaderValue::from_static(version::NAME));
    if !token.is_empty() {
        let bearer = format!("Bearer {token}");
        let bearer = HeaderValue::from_str(&bearer).map_err(|_| Error::InvalidBearerToken())?;
        headers.insert(header::AUTHORIZATION, bearer);
    }
    Client::builder()
        .default_headers(headers)
        .build()
        .map_err(Error::CreateRequest)
}

async fn send_request(builder: reqwest::RequestBuilder) -> Result<reqwest::Response, Error> {
    let response = builder.send().await.map_err(Error::Send)?;

    if !response.status().is_success() {
        let status = response.status();
        let url = response.url().to_string();

        debug!(
            "Request failed with: status='{}', body='{}'",
            status,
            response.text().await.unwrap_or_default(),
        );
        return Err(Error::NonOkStatus(url, status));
    }
    Ok(response)
}

async fn receive_body(response: reqwest::Response) -> Result<String, Error> {
    response.text().await.map_err(Error::ReceiveBody)
}
