use crate::{
    api,
    types::{CHECK_RUN_CONCLUSION, CheckRun},
};
use serde::{Deserialize, Serialize};
use tracing::{debug, warn};

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

#[derive(Clone)]
pub struct Client {
    client_id: String,
    key: jsonwebtoken::EncodingKey,
    api: String,
}

impl Client {
    /// Create a new GitHub client with the provided options.
    /// Will read the private key from the file system.
    pub fn build(options: ClientOptions) -> Result<Self, String> {
        let key = std::fs::read_to_string(&options.private_key)
            .map_err(|e| format!("Failed to read private key: {e}"))?;
        let key = jsonwebtoken::EncodingKey::from_rsa_pem(key.as_bytes())
            .map_err(|e| format!("Failed to create encoding key: {e}"))?;
        Ok(Client {
            client_id: options.client_id,
            key,
            api: options.api,
        })
    }
    /// Get an installations token for the GitHub App.
    async fn get_token(&self, app_installation_id: u64) -> Result<String, String> {
        let claims = JWTClaims::new(&self.client_id);
        let header = jsonwebtoken::Header::new(jsonwebtoken::Algorithm::RS256);
        let jwt = jsonwebtoken::encode(&header, &claims, &self.key)
            .map_err(|e| format!("Failed to create JWT token: {e}"))?;
        api::get_installation_token(&self.api, &jwt, app_installation_id).await
    }
    /// Return a list of current check runs for a commit in a repository.
    /// Needs to use the GitHub App installation token to authenticate.
    pub async fn get_check_runs(
        &self,
        app_installation_id: u64,
        repo: &str,
        commit: &str,
    ) -> Result<Vec<CheckRun>, String> {
        let token = self
            .get_token(app_installation_id)
            .await
            .map_err(|e| format!("Failed to get token: {e}"))?;

        api::get_check_runs(&self.api, &token, repo, commit).await
    }
    /// Create a new pending check run for a commit in a repository.
    /// Needs to use the GitHub App installation token to authenticate.
    pub async fn create_check_run(
        &self,
        app_installation_id: u64,
        repo: &str,
        commit: &str,
    ) -> Result<(), String> {
        let token = self
            .get_token(app_installation_id)
            .await
            .map_err(|e| format!("Failed to get token: {e}"))?;

        api::create_check_run(&self.api, &token, repo, &CheckRun::new(commit)).await
    }
    /// Check a collection of check runs and returns the number of uncompleted check runs.
    /// Additionally returns the check run created by this app. If there are multiple check-runs, the first will be returned.
    pub fn overall_check_status(&self, check_runs: &[CheckRun]) -> (u32, Option<CheckRun>) {
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
                        .is_some_and(|v| v == CHECK_RUN_CONCLUSION)
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
    /// Update the status of the check-run if necessary.
    pub async fn update_check_run(
        &self,
        app_installation_id: u64,
        repo: &str,
        commit: &str,
        success: bool,
        check_run: &mut Option<CheckRun>,
    ) -> Result<(), String> {
        let token = self
            .get_token(app_installation_id)
            .await
            .map_err(|e| format!("Failed to get token: {e}"))?;

        match check_run.as_mut() {
            Some(run) => {
                run.update_status(success);
                api::update_check_run(&self.api, &token, repo, run).await
            }
            None => {
                warn!("No check run found to update, creating a new one");
                let mut run = CheckRun::new(commit);
                run.update_status(success);
                api::create_check_run(&self.api, &token, repo, &run).await
            }
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
