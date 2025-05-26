package client

import "net/http"

func commonHeaders(req *http.Request, token string) {
	req.Header.Set("accept", "application/vnd.github+json")
	req.Header.Set("X-GitHub-Api-Version", "2022-11-28")
	if token != "" {
		req.Header.Set("Authorization", "Bearer "+token)
	}
}
