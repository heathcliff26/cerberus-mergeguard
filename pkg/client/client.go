package client

import (
	"encoding/json"
	"fmt"
	"net/http"
	"os"
	"time"

	"github.com/golang-jwt/jwt/v5"
	"github.com/heathcliff26/cerberus-mergeguard/pkg/config"
)

type GithubClient struct {
	config.GithubConfig
}

type InstallationAccessTokenResponse struct {
	Token     string `json:"token"`
	ExpiresAt string `json:"expires_at"`
	//expires   time.Time `json:"-"`
}

// Create and initialize a new GithubClient
func NewGithubClient(cfg config.GithubConfig) *GithubClient {
	return &GithubClient{
		GithubConfig: cfg,
	}
}

// Get a new JWT for authentication
func (c *GithubClient) createJWT() (string, error) {
	f, err := os.ReadFile(c.PrivateKey)
	if err != nil {
		return "", fmt.Errorf("failed to read private key file '%s': %w", c.PrivateKey, err)
	}
	key, err := jwt.ParseRSAPrivateKeyFromPEM(f)
	if err != nil {
		return "", fmt.Errorf("failed to parse private key from PEM: %w", err)
	}

	token := jwt.NewWithClaims(jwt.SigningMethodRS256, jwt.MapClaims{
		// Use time of 30s earlier to avoid clock skew issues
		"iat": jwt.NewNumericDate(time.Now().Add(time.Second * -30)),
		// We don't re-use the token, so it should expire relatively soon
		"exp": jwt.NewNumericDate(time.Now().Add(time.Minute * 5)),
		"iss": c.ClientID,
		"alg": "RS256",
	})
	return token.SignedString(key)
}

// Get an installation access token
// API endpoint: POST /app/installations/{installation_id}/access_tokens
func (c *GithubClient) GetInstallationAccessToken(installationID int64) (string, error) {
	jwtToken, err := c.createJWT()
	if err != nil {
		return "", fmt.Errorf("failed to create JWT: %w", err)
	}

	req, err := http.NewRequest("POST", fmt.Sprintf("%s/app/installations/%d/access_tokens", c.API, installationID), nil)
	if err != nil {
		return "", fmt.Errorf("failed to create request for installation access token: %w", err)
	}
	commonHeaders(req, jwtToken)

	res, err := http.DefaultClient.Do(req)
	if err != nil {
		return "", fmt.Errorf("failed to get access token: %w", err)
	}
	defer res.Body.Close()

	if res.StatusCode != http.StatusCreated {
		return "", fmt.Errorf("failed to get access token, status code: %d", res.StatusCode)
	}

	var tokenResponse InstallationAccessTokenResponse
	err = json.NewDecoder(res.Body).Decode(&tokenResponse)
	if err != nil {
		return "", fmt.Errorf("failed to decode installation access token response: %w", err)
	}

	return tokenResponse.Token, nil
}

func (c *GithubClient) HandlePullRequestEvent(event PullRequestEvent) {
	// TODO: Implement
}

func (c *GithubClient) HandleCheckRunEvent(event CheckRunEvent) {
	// TODO: Implement
}
