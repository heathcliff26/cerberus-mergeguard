use crate::{
    api,
    error::Error,
    types::{CHECK_RUN_CONCLUSION, CheckRun, TokenResponse},
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::sync::Mutex;
use tracing::{debug, warn};

#[cfg(test)]
mod test;

/// Configuration options for creating the github client
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "kebab-case")]
pub struct ClientOptions {
    /// Client ID for the GitHub App
    pub client_id: String,

    /// Private key for the GitHub App
    pub private_key: String,

    /// URL to github api, defaults to "https://api.github.com"
    #[serde(skip_serializing_if = "str::is_empty", default = "default_api_url")]
    pub api: String,
}

fn default_api_url() -> String {
    "https://api.github.com".to_string()
}

impl ClientOptions {
    /// Validate the client options
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.client_id.is_empty() {
            return Err("GitHub Client ID must be set in the configuration");
        }
        Ok(())
    }
}

pub struct Client {
    client_id: String,
    key: jsonwebtoken::EncodingKey,
    api: String,
    token_cache: Mutex<HashMap<u64, TokenResponse>>,
}

impl Client {
    /// Create a new GitHub client with the provided options.
    /// Will read the private key from the file system.
    pub fn build(options: ClientOptions) -> Result<Self, Error> {
        let key = std::fs::read_to_string(&options.private_key)
            .map_err(|e| Error::ReadPrivateKey(options.private_key.clone(), e))?;
        let key =
            jsonwebtoken::EncodingKey::from_rsa_pem(key.as_bytes()).map_err(Error::EncodingKey)?;
        Ok(Client {
            client_id: options.client_id,
            key,
            api: options.api,
            token_cache: Mutex::new(HashMap::new()),
        })
    }

    /// Return a reference to the client ID.
    pub fn client_id(&self) -> &str {
        &self.client_id
    }

    /// Get an installations token for the GitHub App.
    async fn get_token(&self, app_installation_id: u64) -> Result<String, Error> {
        if let Some(token) = self.get_cached_token(app_installation_id).await {
            return Ok(token);
        }

        let claims = JWTClaims::new(&self.client_id);
        let header = jsonwebtoken::Header::new(jsonwebtoken::Algorithm::RS256);
        let jwt = jsonwebtoken::encode(&header, &claims, &self.key).map_err(Error::JWT)?;
        let token = api::get_installation_token(&self.api, &jwt, app_installation_id).await?;

        let mut cache = self.token_cache.lock().await;
        let token_value = token.token.clone();
        cache.insert(app_installation_id, token);

        Ok(token_value)
    }

    /// Create a new pending check run for a commit in a repository.
    /// Needs to use the GitHub App installation token to authenticate.
    pub async fn create_check_run(
        &self,
        app_installation_id: u64,
        repo: &str,
        commit: &str,
    ) -> Result<(), Error> {
        let token = self.get_token(app_installation_id).await?;

        api::create_check_run(&self.api, &token, repo, &CheckRun::new(commit)).await
    }

    /// Refresh the check_run status based on the current status.
    /// Will fetch the current check-runs first and then update the check-run status.
    /// This means 2 API calls will be made.
    pub async fn refresh_check_run_status(
        &self,
        app_id: u64,
        repo: &str,
        commit: &str,
    ) -> Result<(), Error> {
        let (uncompleted, own_run) = self.get_check_run_status(app_id, repo, commit).await?;
        self.update_check_run(app_id, repo, commit, uncompleted, own_run)
            .await
    }

    /// Get the combined status of all check-runs for a commit.
    pub async fn get_check_run_status(
        &self,
        app_installation_id: u64,
        repo: &str,
        commit: &str,
    ) -> Result<(u32, Option<CheckRun>), Error> {
        let check_runs = self
            .get_check_runs(app_installation_id, repo, commit)
            .await?;
        debug!(
            "Found {} check runs for commit '{}' in repository '{}'",
            check_runs.len(),
            commit,
            repo
        );

        Ok(self.overall_check_status(&check_runs))
    }

    /// Update the status of the check-run if necessary.
    pub async fn update_check_run(
        &self,
        app_installation_id: u64,
        repo: &str,
        commit: &str,
        count: u32,
        check_run: Option<CheckRun>,
    ) -> Result<(), Error> {
        let token = self.get_token(app_installation_id).await?;

        match check_run {
            Some(mut run) => {
                if run.update_status(count) {
                    api::update_check_run(&self.api, &token, repo, &run).await
                } else {
                    debug!("No changes to check run status, skipping update");
                    Ok(())
                }
            }
            None => {
                warn!("No check run found to update, creating a new one");
                let mut run = CheckRun::new(commit);
                run.update_status(count);
                api::create_check_run(&self.api, &token, repo, &run).await
            }
        }
    }

    /// Get the current head commit for a pull request.
    pub async fn get_pull_request_head_commit(
        &self,
        app_installation_id: u64,
        repo: &str,
        pull_number: u64,
    ) -> Result<String, Error> {
        let token = self.get_token(app_installation_id).await?;

        let pr = api::get_pull_request(&self.api, &token, repo, pull_number).await?;

        Ok(pr.head.sha)
    }

    /// Return a list of current check runs for a commit in a repository.
    /// Needs to use the GitHub App installation token to authenticate.
    async fn get_check_runs(
        &self,
        app_installation_id: u64,
        repo: &str,
        commit: &str,
    ) -> Result<Vec<CheckRun>, Error> {
        let token = self.get_token(app_installation_id).await?;

        api::get_check_runs(&self.api, &token, repo, commit).await
    }

    /// Check a collection of check runs and returns the number of uncompleted check runs.
    /// Additionally returns the check run created by this app. If there are multiple check-runs, the first will be returned.
    fn overall_check_status(&self, check_runs: &[CheckRun]) -> (u32, Option<CheckRun>) {
        if check_runs.is_empty() {
            warn!("Received empty check-runs list");
            return (0, None);
        }
        let mut uncompleted = 0;
        let mut own_check_run: Option<CheckRun> = None;

        for run in check_runs {
            if run
                .app
                .as_ref()
                .is_some_and(|app| app.client_id == self.client_id)
            {
                // This is a check run created by this app
                if own_check_run.is_none() {
                    own_check_run = Some(run.clone());
                } else {
                    warn!(
                        "Found multiple check runs created by this app: '{}' and '{}, commit: '{}'",
                        own_check_run.as_ref().unwrap().name,
                        run.name,
                        run.head_sha
                    );
                }
                debug!("Found own check run: {}", run.id);
                continue;
            }
            match run.status.as_str() {
                "completed" => {
                    if run
                        .conclusion
                        .as_ref()
                        .is_some_and(|v| v == CHECK_RUN_CONCLUSION || v == "skipped")
                    {
                        debug!("Check run '{}' is completed successfully", run.name);
                    } else {
                        debug!(
                            "Check run '{}' is completed not successfull: '{}'",
                            run.name,
                            run.conclusion.as_deref().unwrap_or("unknown")
                        );
                        uncompleted += 1;
                    }
                }
                _ => {
                    debug!(
                        "Check run '{}' is not completed, status: {}",
                        run.name, run.status
                    );
                    uncompleted += 1;
                }
            }
        }
        (uncompleted, own_check_run)
    }

    /// Check the cache for a token and return it if it exists.
    async fn get_cached_token(&self, app_installation_id: u64) -> Option<String> {
        let cache = self.token_cache.lock().await;
        if let Some(token) = cache.get(&app_installation_id) {
            let now = chrono::Utc::now() + chrono::Duration::seconds(30);
            if token.expires_at.ge(&now) {
                debug!(
                    "Using cached token for installation ID: {}",
                    app_installation_id
                );
                return Some(token.token.clone());
            }
            debug!(
                "Cached token for installation ID {} is expired, fetching a new one",
                app_installation_id
            );
        }
        None
    }

    #[cfg(test)]
    pub fn new_for_testing(client_id: &str, secret: &str, api: &str) -> Self {
        let key = jsonwebtoken::EncodingKey::from_secret(secret.as_bytes());

        Client {
            client_id: client_id.to_string(),
            key,
            api: api.to_string(),
            token_cache: Mutex::new(HashMap::new()),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct JWTClaims {
    /// Issued At
    /// Recommended to be 60 seconds in the past to account for clock drift
    iat: u64,
    /// Expires At
    /// Maximum of 10 minutes in the future
    exp: u64,
    /// Issuer
    /// The GitHub App's client ID
    iss: String,
}

impl JWTClaims {
    /// Create a new JWT claims object with the issued time 30s in the past
    pub fn new(client_id: &str) -> Self {
        debug!("Creating JWT claims for client ID: {}", client_id);
        let now = jsonwebtoken::get_current_timestamp();
        let iat = now - 30;
        let exp = now + 2 * 60;
        JWTClaims {
            iat,
            exp,
            iss: client_id.to_string(),
        }
    }
}
