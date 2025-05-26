package client

import (
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"testing"

	"github.com/stretchr/testify/assert"
	"github.com/stretchr/testify/require"
)

func TestGetCheckRuns(t *testing.T) {
	assert := assert.New(t)

	s := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		assert.Equal("/repos/testowner/testrepo/commits/testcommit/check-runs", r.URL.Path)
		assert.Equal("application/vnd.github+json", r.Header.Get("accept"))
		assert.Equal("2022-11-28", r.Header.Get("X-GitHub-Api-Version"))
		assert.Equal("Bearer testtoken", r.Header.Get("Authorization"))

		w.WriteHeader(200)
		_, _ = w.Write([]byte(`{
			"total_count": 1,
			"check_runs": [{
				"id": 123456,
				"name": "test-check",
				"head_sha": "testcommit",
				"status": "completed",
				"conclusion": "success"
			}]
		}`))
	}))

	client := &PRClient{
		repoURL: s.URL + "/repos/testowner/testrepo",
		commit:  "testcommit",
		token:   "testtoken",
	}

	checkRuns, err := client.GetCheckRuns()
	assert.NoError(err, "Expected no error when fetching check runs")
	require.Len(t, checkRuns, 1, "Expected one check run")

	expectedRun := CheckRun{
		ID:         123456,
		Name:       "test-check",
		HeadSHA:    "testcommit",
		Status:     "completed",
		Conclusion: "success",
	}
	assert.Equal(expectedRun, checkRuns[0], "Should return the expected check run")
}

func TestCreateCheckRun(t *testing.T) {
	assert := assert.New(t)

	s := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		assert.Equal("/repos/testowner/testrepo/check-runs", r.URL.Path)
		assert.Equal("application/vnd.github+json", r.Header.Get("accept"))
		assert.Equal("2022-11-28", r.Header.Get("X-GitHub-Api-Version"))
		assert.Equal("Bearer testtoken", r.Header.Get("Authorization"))

		var checkRun CheckRun
		require.NoError(t, json.NewDecoder(r.Body).Decode(&checkRun))
		assert.Equal("test-check", checkRun.Name)
		assert.Equal("testcommit", checkRun.HeadSHA)
		assert.Equal("pending", checkRun.Status)

		w.WriteHeader(http.StatusCreated)
		_, _ = w.Write([]byte(`{
			"id": 654321,
			"name": "test-check",
			"head_sha": "testcommit",
			"status": "pending"
		}`))
	}))

	client := &PRClient{
		repoURL: s.URL + "/repos/testowner/testrepo",
		commit:  "testcommit",
		token:   "testtoken",
	}

	err := client.CreateCheckRun("test-check")
	assert.NoError(err, "Expected no error when creating check run")
}

func TestUpdateCheckRun(t *testing.T) {
	assert := assert.New(t)

	s := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		assert.Equal("/repos/testowner/testrepo/check-runs/654321", r.URL.Path)
		assert.Equal("application/vnd.github+json", r.Header.Get("accept"))
		assert.Equal("2022-11-28", r.Header.Get("X-GitHub-Api-Version"))
		assert.Equal("Bearer testtoken", r.Header.Get("Authorization"))

		var checkRun CheckRun
		require.NoError(t, json.NewDecoder(r.Body).Decode(&checkRun))
		assert.Equal(int64(654321), checkRun.ID)
		assert.Equal("completed", checkRun.Status)
		assert.Equal("success", checkRun.Conclusion)

		w.WriteHeader(http.StatusOK)
	}))

	client := &PRClient{
		repoURL: s.URL + "/repos/testowner/testrepo",
		commit:  "testcommit",
		token:   "testtoken",
	}

	payload := CheckRun{
		ID:         654321,
		Status:     "completed",
		Conclusion: "success",
	}
	err := client.UpdateCheckRun(payload)
	assert.NoError(err, "Expected no error when updating check run")
}
