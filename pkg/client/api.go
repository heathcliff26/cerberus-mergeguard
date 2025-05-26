package client

import (
	"bytes"
	"encoding/json"
	"fmt"
	"io"
	"log/slog"
	"net/http"
	"time"
)

type PRClient struct {
	// The api url of the repository, should be provided by the webhook payload.
	// Example: https://api.github.com/repos/<owner>/<repo>
	repoURL string
	// The commit SHA for which to fetch check runs.
	commit string
	// The authentication token to use for the api call.
	token string
}

// Fetch all check runs for a current pull request commit.
// API endpoint: GET /repos/{owner}/{repo}/commits/{ref}/check-runs
func (c *PRClient) GetCheckRuns() ([]CheckRun, error) {
	req, err := http.NewRequest(http.MethodGet, c.repoURL+"/commits/"+c.commit+"/check-runs", nil)
	if err != nil {
		return nil, fmt.Errorf("failed to create request for check-runs: %w", err)
	}
	commonHeaders(req, c.token)

	res, err := http.DefaultClient.Do(req)
	if err != nil {
		return nil, fmt.Errorf("failed to request check-runs from api: %w", err)
	}
	defer res.Body.Close()

	if res.StatusCode != http.StatusOK {
		return nil, fmt.Errorf("failed to get check-runs from api, status code: %d", res.StatusCode)
	}

	var runs CheckRuns
	err = json.NewDecoder(res.Body).Decode(&runs)
	if err != nil {
		return nil, fmt.Errorf("failed to decode check-runs response: %w", err)
	}

	return runs.CheckRuns, nil
}

// Create a check run for a specific commit.
// API endpoint: POST /repos/{owner}/{repo}/check-runs
func (c *PRClient) CreateCheckRun(name string) error {
	req, err := http.NewRequest(http.MethodPost, c.repoURL+"/check-runs", nil)
	if err != nil {
		return fmt.Errorf("failed to create request for check-run: %w", err)
	}
	commonHeaders(req, c.token)

	payload := CheckRun{
		Name:      name,
		HeadSHA:   c.commit,
		Status:    "pending",
		StartedAt: time.Now().Format(time.RFC3339),
		Output: CheckRunOutput{
			Title:   name,
			Summary: "Waiting for other checks to complete",
		},
	}

	body, err := json.Marshal(payload)
	if err != nil {
		return fmt.Errorf("failed to marshal check-run payload: %w", err)
	}
	req.Body = io.NopCloser(bytes.NewReader(body))

	res, err := http.DefaultClient.Do(req)
	if err != nil {
		return fmt.Errorf("failed to request check-run creation from api: %w", err)
	}
	defer res.Body.Close()
	if res.StatusCode != http.StatusCreated {
		return fmt.Errorf("failed to create check-run, status code: %d", res.StatusCode)
	}

	var createdRun CheckRun
	err = json.NewDecoder(res.Body).Decode(&createdRun)
	if err != nil {
		slog.Warn("Failed to decode created check-run response", slog.String("error", err.Error()))
	} else {
		slog.Debug("Check run created", slog.Int64("id", createdRun.ID))
	}
	return nil
}

// Update an existing check runs status.
// API endpoint: PATCH /repos/{owner}/{repo}/check-runs/{check_run_id}
func (c *PRClient) UpdateCheckRun(payload CheckRun) error {
	if payload.ID == 0 {
		return fmt.Errorf("check run ID must be set to update a check run")
	}

	req, err := http.NewRequest(http.MethodPatch, fmt.Sprintf("%s/check-runs/%d", c.repoURL, payload.ID), nil)
	if err != nil {
		return fmt.Errorf("failed to create request for check-run update: %w", err)
	}
	commonHeaders(req, c.token)

	body, err := json.Marshal(payload)
	if err != nil {
		return fmt.Errorf("failed to marshal check-run update payload: %w", err)
	}
	req.Body = io.NopCloser(bytes.NewReader(body))

	res, err := http.DefaultClient.Do(req)
	if err != nil {
		return fmt.Errorf("failed to update check-run: %w", err)
	}
	defer res.Body.Close()
	if res.StatusCode != http.StatusOK {
		return fmt.Errorf("failed to update check-run, status code: %d", res.StatusCode)
	}

	return nil
}
