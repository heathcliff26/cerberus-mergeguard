package server

import (
	"encoding/json"
	"errors"
	"fmt"
	"io"
	"log/slog"
	"net/http"
	"strconv"
	"time"

	"github.com/heathcliff26/cerberus-mergeguard/pkg/client"
	"github.com/heathcliff26/cerberus-mergeguard/pkg/config"
	"github.com/heathcliff26/simple-fileserver/pkg/middleware"
)

type Server struct {
	addr   string
	ssl    config.SSLConfig
	github *client.GithubClient
}

func NewServer(cfgServer config.ServerConfig, github *client.GithubClient) *Server {
	return &Server{
		addr:   ":" + strconv.Itoa(cfgServer.Port),
		ssl:    cfgServer.SSL,
		github: github,
	}
}

// Handle incoming github webhook events
// URL: POST /webhook
func (s *Server) webhookHandler(res http.ResponseWriter, req *http.Request) {
	signatureHeader := req.Header.Get("X-Hub-Signature-256")
	if signatureHeader == "" && s.github.WebhookSecret != "" {
		slog.Error("Missing X-Hub-Signature-256 header")
		res.WriteHeader(http.StatusBadRequest)
		return
	}

	body, err := io.ReadAll(req.Body)
	if err != nil {
		slog.Error("Failed to read request body", slog.String("err", err.Error()))
		res.WriteHeader(http.StatusBadRequest)
		return
	}

	if signatureHeader != "" && s.github.WebhookSecret != "" {
		err := verifyWebhookSignature(body, s.github.WebhookSecret, signatureHeader)
		if err != nil {
			slog.Error("Failed to verify webhook signature", slog.String("err", err.Error()))
			res.WriteHeader(http.StatusUnauthorized)
			return
		}
	}

	switch req.Header.Get("X-GitHub-Event") {
	case "pull_request":
		slog.Info("Handling pull request event")
		var event client.PullRequestEvent
		err = json.Unmarshal(body, &event)
		if err != nil {
			slog.Error("Failed to unmarshal pull request event", slog.String("err", err.Error()))
			res.WriteHeader(http.StatusBadRequest)
		} else {
			s.github.HandlePullRequestEvent(event)
		}
	case "check_run":
		slog.Info("Handling check run event")
		var event client.CheckRunEvent
		err = json.Unmarshal(body, &event)
		if err != nil {
			slog.Error("Failed to unmarshal check-run event", slog.String("err", err.Error()))
			res.WriteHeader(http.StatusBadRequest)
		} else {
			s.github.HandleCheckRunEvent(event)
		}
	default:
		slog.Warn("Unhandled GitHub event", slog.String("event", req.Header.Get("X-GitHub-Event")))
	}
}

// Return a health status of the server
// URL: /healthz
func (s *Server) handleHealthCheck(rw http.ResponseWriter, _ *http.Request) {
	rw.Header().Set("Content-Type", "application/json")
	_, err := rw.Write([]byte(`{"status":"ok"}`))
	if err != nil {
		slog.Error("Failed to write health check response", slog.String("err", err.Error()))
		rw.WriteHeader(http.StatusInternalServerError)
		return
	}
}

// Starts the server and exits with error if that fails
func (s *Server) Run() error {
	router := http.NewServeMux()
	router.HandleFunc("POST /webhook", s.webhookHandler)
	router.HandleFunc("/healthz", s.handleHealthCheck)

	server := http.Server{
		Addr:         s.addr,
		Handler:      middleware.Logging(router),
		ReadTimeout:  10 * time.Second,
		WriteTimeout: 10 * time.Second,
	}

	var err error
	if s.ssl.Enabled {
		slog.Info("Starting server", slog.String("addr", s.addr), slog.String("sslKey", s.ssl.Key), slog.String("sslCert", s.ssl.Cert))
		err = server.ListenAndServeTLS(s.ssl.Cert, s.ssl.Key)
	} else {
		slog.Info("Starting server", slog.String("addr", s.addr))
		err = server.ListenAndServe()
	}

	// This just means the server was closed after running
	if errors.Is(err, http.ErrServerClosed) {
		slog.Info("Server closed, exiting")
		return nil
	}
	return fmt.Errorf("failed to start server: %w", err)
}
