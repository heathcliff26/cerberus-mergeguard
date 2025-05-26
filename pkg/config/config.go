package config

import (
	"fmt"
	"log/slog"
	"os"
	"strings"

	"sigs.k8s.io/yaml"
)

const (
	DEFAULT_CONFIG_PATH = "/config/config.yaml"

	DEFAULT_LOG_LEVEL   = "info"
	DEFAULT_SERVER_PORT = 8080

	DEFAULT_API_URL = "https://api.github.com"
)

var logLevel *slog.LevelVar

// Initialize the logger
func init() {
	logLevel = &slog.LevelVar{}
	opts := slog.HandlerOptions{
		Level: logLevel,
	}
	logger := slog.New(slog.NewTextHandler(os.Stdout, &opts))
	slog.SetDefault(logger)
}

type Config struct {
	LogLevel string       `json:"logLevel,omitempty"`
	Server   ServerConfig `json:"server,omitempty"`
	Github   GithubConfig `json:"github"`
}

type ServerConfig struct {
	Port int       `json:"port,omitempty"`
	SSL  SSLConfig `json:"ssl,omitempty"`
}

type SSLConfig struct {
	Enabled bool   `json:"enabled,omitempty"`
	Cert    string `json:"cert,omitempty"`
	Key     string `json:"key,omitempty"`
}

type GithubConfig struct {
	ClientID      string `json:"client-id"`
	PrivateKey    string `json:"private-key"`
	WebhookSecret string `json:"webhook-secret,omitempty"`
	API           string `json:"api,omitempty"`
}

// Returns a Config with default values set
func DefaultConfig() Config {
	return Config{
		LogLevel: DEFAULT_LOG_LEVEL,
		Server: ServerConfig{
			Port: DEFAULT_SERVER_PORT,
		},
		Github: GithubConfig{
			API: DEFAULT_API_URL,
		},
	}
}

// Loads config from file, returns error if config is invalid
// Arguments:
//
//		path: Path to config file, if empty will use either DEFAULT_CONFIG_PATH or DEFAULT_CONFIG_PATH_CONTAINER
//		env: Determines if enviroment variables in the file will be expanded before decoding
//	 logLevelOverride: Override the log level given by the config
func LoadConfig(path string, env bool, logLevelOverride string) (Config, error) {
	c, err := loadConfigFile(path, env)
	if err != nil {
		return Config{}, fmt.Errorf("failed to load configuration file '%s': %w", path, err)
	}

	if logLevelOverride == "" {
		err = setLogLevel(c.LogLevel)
	} else {
		err = setLogLevel(logLevelOverride)
	}
	if err != nil {
		return Config{}, fmt.Errorf("failed to set log level to '%s': %w", logLevelOverride, err)
	}

	if c.Server.SSL.Enabled && (c.Server.SSL.Cert == "" || c.Server.SSL.Key == "") {
		return Config{}, fmt.Errorf("incomplete SSL configuration: cert and key must be set if SSL is enabled")
	}

	if c.Github.ClientID == "" {
		return Config{}, fmt.Errorf("GitHub Client ID must be set in the configuration")
	}

	f, err := os.OpenFile(c.Github.PrivateKey, os.O_RDONLY, 0600)
	if err != nil {
		return Config{}, fmt.Errorf("can't open Github App private key '%s': %w", c.Github.PrivateKey, err)
	}
	defer f.Close()

	return c, nil
}

func loadConfigFile(path string, env bool) (Config, error) {
	c := DefaultConfig()

	p := path
	if p == "" {
		p = DEFAULT_CONFIG_PATH
	}

	// #nosec G304 -- Local users can decide on their file path themselves.
	f, err := os.ReadFile(p)
	if path == "" && os.IsNotExist(err) {
		slog.Info("No config file specified and default file does not exist, falling back to default values.", slog.String("default-path", p))
		return c, nil
	} else if err != nil {
		return Config{}, fmt.Errorf("failed to read config file '%s': %w", p, err)
	}

	if env {
		f = []byte(os.ExpandEnv(string(f)))
	}

	err = yaml.Unmarshal(f, &c)
	if err != nil {
		return Config{}, fmt.Errorf("failed to unmarshal config file '%s': %w", p, err)
	}

	return c, nil
}

// Parse a given string and set the resulting log level
func setLogLevel(level string) error {
	switch strings.ToLower(level) {
	case "debug":
		logLevel.Set(slog.LevelDebug)
	case "info":
		logLevel.Set(slog.LevelInfo)
	case "warn":
		logLevel.Set(slog.LevelWarn)
	case "error":
		logLevel.Set(slog.LevelError)
	default:
		return fmt.Errorf("invalid log level '%s'", level)
	}
	return nil
}
