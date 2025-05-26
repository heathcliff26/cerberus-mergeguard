package server

import (
	"crypto/hmac"
	"crypto/sha256"
	"fmt"
	"strings"
)

// Verify the X-Hub-Signature-256 signature of a GitHub webhook request
func verifyWebhookSignature(body []byte, secret string, signature string) error {
	hash := hmac.New(sha256.New, []byte(secret))
	_, err := hash.Write(body)
	if err != nil {
		return fmt.Errorf("failed to write body to hash: %w", err)
	}

	expectedSignature := fmt.Sprintf("%x", hash.Sum(nil))
	signature, _ = strings.CutPrefix(signature, "sha256=")
	if !hmac.Equal([]byte(expectedSignature), []byte(signature)) {
		return fmt.Errorf("signature mismatch: got '%s', calculated '%s'", signature, expectedSignature)
	}
	return nil
}
