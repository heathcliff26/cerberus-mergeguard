use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[cfg(test)]
mod test;

/// Name of check-runs created by the bot
pub const CHECK_RUN_NAME: &str = "cerberus-mergeguard";
/// Status for unfinished check-runs from the bot
/// Using 'queued', because while 'pending' is valid according to docs, the actual API does not allow it.
pub const CHECK_RUN_INITIAL_STATUS: &str = "queued";
/// Status for completed check-runs from the bot
pub const CHECK_RUN_COMPLETED_STATUS: &str = "completed";
/// Conclusion for completed check-runs from the bot
pub const CHECK_RUN_CONCLUSION: &str = "success";
/// Title for unfinished check-runs from the bot
pub const CHECK_RUN_INITIAL_TITLE: &str = "Waiting for other checks to complete";
/// Title for completed check-runs from the bot
pub const CHECK_RUN_COMPLETED_TITLE: &str = "All status checks have passed";
/// Summary for check-runs from the bot
pub const CHECK_RUN_SUMMARY: &str = "Will block merging until all other checks have completed";

/// Partial fields of a pull_request event webhook payload.
#[derive(Debug, Serialize, Deserialize)]
pub struct PullRequestEvent {
    pub action: String,
    pub installation: Option<Installation>,
    pub number: u64,
    pub pull_request: PullRequest,
    pub repository: Repo,
}

/// Partial fields of a check_run event webhook payload.
#[derive(Debug, Serialize, Deserialize)]
pub struct CheckRunEvent {
    pub action: String,
    pub check_run: CheckRun,
    pub installation: Option<Installation>,
    pub repository: Repo,
}

/// Partial fields of an issue_comment event webhook payload.
#[derive(Debug, Serialize, Deserialize)]
pub struct IssueCommentEvent {
    pub action: String,
    pub issue: Issue,
    pub comment: Comment,
    pub installation: Option<Installation>,
    pub repository: Repo,
}

/// Partial fields of a pull_request object.
#[derive(Debug, Serialize, Deserialize)]
pub struct PullRequest {
    pub number: u64,
    pub title: String,
    pub head: BranchRef,
}

/// Partial fields of a branch reference object.
#[derive(Debug, Serialize, Deserialize)]
pub struct BranchRef {
    pub label: String,
    #[serde(rename = "ref")]
    pub ref_field: String,
    pub sha: String,
    pub repo: Repo,
}

/// Partial fields of a repository object.
#[derive(Debug, Serialize, Deserialize)]
pub struct Repo {
    pub id: u64,
    pub name: String,
    pub full_name: String,
}

/// Partial fields of a check_run object.
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
    /// Update the status based on the count of uncompleted check-runs.
    /// Returns if the content of the check-run has changed.
    pub fn update_status(&mut self, count: u32) -> bool {
        let status: String;
        let conclusion: Option<String>;
        let output_title: Option<String>;

        if count == 0 {
            status = CHECK_RUN_COMPLETED_STATUS.to_string();
            conclusion = Some(CHECK_RUN_CONCLUSION.to_string());
            output_title = Some(CHECK_RUN_COMPLETED_TITLE.to_string());
        } else {
            status = CHECK_RUN_INITIAL_STATUS.to_string();
            conclusion = None;
            output_title = Some(format!("Waiting for {count} other checks to complete"));
        }

        let mut changed = false;

        if self.status != status {
            changed = true;
            self.status = status;
        }
        if self.conclusion != conclusion {
            changed = true;
            self.conclusion = conclusion;
        }
        match &mut self.output {
            Some(output) => {
                if output.title != output_title {
                    changed = true;
                    output.title = output_title;
                }
            }
            None => {
                changed = true;
                self.output = Some(CheckRunOutput {
                    title: output_title,
                    summary: Some(CHECK_RUN_SUMMARY.to_string()),
                });
            }
        }

        changed
    }
}

/// Partial fields of a check_run output object.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CheckRunOutput {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
}

/// Partial fields of a GitHub App object.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct App {
    pub id: u64,
    pub client_id: String,
    pub slug: String,
    pub name: String,
}

/// Partial fields of an installation object.
#[derive(Debug, Serialize, Deserialize)]
pub struct Installation {
    pub id: u64,
}

/// Partial fields of a comment object.
#[derive(Debug, Serialize, Deserialize)]
pub struct Comment {
    pub id: u64,
    pub body: String,
}

/// Partial fields of an issue object.
#[derive(Debug, Serialize, Deserialize)]
pub struct Issue {
    pub id: u64,
    pub number: u64,
}

/// Response to check-run requests from the GitHub API.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CheckRunsResponse {
    pub total_count: u64,
    pub check_runs: Vec<CheckRun>,
}

/// Response to installation token requests from the GitHub API.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TokenResponse {
    pub token: String,
    pub expires_at: DateTime<Utc>,
}

/// Response to get pull request from the GitHub API.
#[derive(Debug, Serialize, Deserialize)]
pub struct PullRequestResponse {
    pub id: u64,
    pub number: u64,
    pub head: BranchRef,
}
