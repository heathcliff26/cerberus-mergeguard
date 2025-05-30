use serde::{Deserialize, Serialize};

#[cfg(test)]
mod test;

pub const CHECK_RUN_NAME: &str = "cerberus-mergeguard";
pub const CHECK_RUN_INITIAL_STATUS: &str = "queued";
pub const CHECK_RUN_COMPLETED_STATUS: &str = "completed";
pub const CHECK_RUN_CONCLUSION: &str = "success";
pub const CHECK_RUN_INITIAL_TITLE: &str = "Waiting for other checks to complete";
pub const CHECK_RUN_COMPLETED_TITLE: &str = "All status checks have passed";
pub const CHECK_RUN_SUMMARY: &str = "Will block merging until all other checks have completed";

/// Represents a GitHub webhook event for pull requests.
#[derive(Debug, Serialize, Deserialize)]
pub struct PullRequestEvent {
    pub action: String,
    pub installation: Option<Installation>,
    pub number: u64,
    pub pull_request: PullRequest,
    pub repository: Repo,
    pub sender: User,
}

/// Represents a GitHub check run event.
#[derive(Debug, Serialize, Deserialize)]
pub struct CheckRunEvent {
    pub action: String,
    pub check_run: CheckRun,
    pub installation: Option<Installation>,
    pub repository: Repo,
    pub sender: User,
}

/// Represents a GitHub pull request.
#[derive(Debug, Serialize, Deserialize)]
pub struct PullRequest {
    pub number: u64,
    pub title: String,
    pub state: String,
    pub merged: bool,
    pub user: User,
    pub base: BranchRef,
    pub head: BranchRef,
}

/// Represents a GitHub user.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct User {
    pub login: String,
    pub id: u64,
}

/// Represents a branch reference in a pull request.
#[derive(Debug, Serialize, Deserialize)]
pub struct BranchRef {
    pub label: String,
    #[serde(rename = "ref")]
    pub ref_field: String,
    pub sha: String,
    pub user: User,
    pub repo: Repo,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Repo {
    pub id: u64,
    pub name: String,
    pub full_name: String,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct CheckRun {
    #[serde(skip_serializing_if = "is_zero")]
    pub id: u64,
    pub name: String,
    pub head_sha: String,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conclusion: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub started_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<CheckRunOutput>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub app: Option<App>,
}

fn is_zero(value: &u64) -> bool {
    *value == 0
}

impl CheckRun {
    /// Create a new check-run for the given commit.
    pub fn new(commit: &str) -> Self {
        CheckRun {
            name: CHECK_RUN_NAME.to_string(),
            head_sha: commit.to_string(),
            status: CHECK_RUN_INITIAL_STATUS.to_string(),
            output: Some(CheckRunOutput {
                title: Some(CHECK_RUN_INITIAL_TITLE.to_string()),
                summary: Some(CHECK_RUN_SUMMARY.to_string()),
            }),
            ..Default::default()
        }
    }
    /// Update the status based on the success flag.
    pub fn update_status(&mut self, success: bool) {
        if success {
            self.status = CHECK_RUN_COMPLETED_STATUS.to_string();
            self.conclusion = Some(CHECK_RUN_CONCLUSION.to_string());
            if let Some(mut output) = self.output.take() {
                output.title = Some(CHECK_RUN_COMPLETED_TITLE.to_string());
                self.output = Some(output);
            }
        } else {
            self.status = CHECK_RUN_INITIAL_STATUS.to_string();
            self.conclusion = None;
            if let Some(mut output) = self.output.take() {
                output.title = Some(CHECK_RUN_INITIAL_TITLE.to_string());
                self.output = Some(output);
            }
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CheckRunOutput {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct App {
    pub id: u64,
    pub client_id: String,
    pub slug: String,
    pub name: String,
    pub owner: User,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Installation {
    pub id: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CheckRunsResponse {
    pub total_count: u64,
    pub check_runs: Vec<CheckRun>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TokenResponse {
    pub token: String,
    pub expires_at: String,
}
