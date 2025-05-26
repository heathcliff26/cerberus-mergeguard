package client

type CheckRunEvent struct {
	Action       string                `json:"action,omitempty"`
	CheckRun     CheckRun              `json:"check_run"`
	Installation GithubAppInstallation `json:"installation,omitempty"`
	Organization struct{}              `json:"organization,omitempty"`
	Repository   Repository            `json:"repository"`
	Sender       Sender                `json:"sender"`
}

type PullRequestEvent struct {
	Action       string                `json:"action"`
	Enterprise   struct{}              `json:"enterprise,omitempty"`
	Installation GithubAppInstallation `json:"installation,omitempty"`
	Number       int                   `json:"number"`
	Organization struct{}              `json:"organization,omitempty"`
	PullRequest  struct{}              `json:"pull_request"`
	Repository   Repository            `json:"repository"`
	Sender       Sender                `json:"sender"`
}

type GithubAppInstallation struct{}

type Repository struct {
	ID       int64  `json:"id"`
	Name     string `json:"name"`
	FullName string `json:"full_name"`
	URL      string `json:"url"`
}

type Sender struct {
	Login string `json:"login"`
	ID    int64  `json:"id"`
}

type CheckRun struct {
	ID          int64          `json:"id,omitempty"`
	Name        string         `json:"name,omitempty"`
	HeadSHA     string         `json:"head_sha,omitempty"`
	Status      string         `json:"status,omitempty"`
	Conclusion  string         `json:"conclusion,omitempty"`
	StartedAt   string         `json:"started_at,omitempty"`
	CompletedAt string         `json:"completed_at,omitempty"`
	Output      CheckRunOutput `json:"output,omitempty"`
}

type CheckRunOutput struct {
	Title   string `json:"title,omitempty"`
	Summary string `json:"summary,omitempty"`
}

type CheckRuns struct {
	TotalCount int        `json:"total_count"`
	CheckRuns  []CheckRun `json:"check_runs"`
}
