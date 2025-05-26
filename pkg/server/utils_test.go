package server

import (
	"testing"

	"github.com/stretchr/testify/assert"
)

func TestVerifyWebhookSignature(t *testing.T) {
	assert := assert.New(t)

	body := []byte("test body")
	secret := "testsecret"
	signature := "sha256=f940fd6cb83a0567daa8d294f0f93ac29abfb5d9e9a25507bb6e88578dea344a"
	err := verifyWebhookSignature(body, secret, signature)
	assert.NoError(err, "Expected no error for valid signature")

	invalidSignature := "sha256=eac7882fb72ea50710dbd3f7b44ba83d2d0dfba80fc75a1396ac8bae33e8bab0"
	err = verifyWebhookSignature(body, secret, invalidSignature)
	assert.Error(err, "Expected error for invalid signature")
}
