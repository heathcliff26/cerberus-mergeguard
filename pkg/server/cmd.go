package server

import (
	"fmt"
	"log/slog"
	"os"

	"github.com/heathcliff26/cerberus-mergeguard/pkg/client"
	"github.com/heathcliff26/cerberus-mergeguard/pkg/config"
	"github.com/heathcliff26/cerberus-mergeguard/pkg/version"
	"github.com/spf13/cobra"
)

const (
	flagNameConfig   = "config"
	flagNameLogLevel = "log"
	flagNameEnv      = "env"
)

func Execute() {
	err := NewServerCmd().Execute()
	if err != nil {
		slog.Error("Failed to execute command", "err", err)
		os.Exit(1)
	}
}

func NewServerCmd() *cobra.Command {
	cobra.AddTemplateFunc(
		"ProgramName", func() string {
			return version.Name
		},
	)

	rootCmd := &cobra.Command{
		Use:   version.Name,
		Short: version.Name + " github bot for guarding pull request merges",
		Run: func(cmd *cobra.Command, args []string) {
			err := run(cmd)
			if err != nil {
				fmt.Println("Fatal: " + err.Error())
				os.Exit(1)
			}
		},
	}

	rootCmd.Flags().StringP(flagNameConfig, "c", "", "Config file to use")
	rootCmd.Flags().String(flagNameLogLevel, "", "Override the log level given in the config file")
	rootCmd.Flags().Bool(flagNameEnv, false, "Expand enviroment variables in the config file")

	rootCmd.AddCommand(
		version.NewCommand(),
	)

	return rootCmd
}

func run(cmd *cobra.Command) error {
	configPath, err := cmd.Flags().GetString(flagNameConfig)
	if err != nil {
		return fmt.Errorf("failed to get config flag: %w", err)
	}
	logLevel, err := cmd.Flags().GetString(flagNameLogLevel)
	if err != nil {
		return fmt.Errorf("failed to get log level flag: %w", err)
	}
	env, err := cmd.Flags().GetBool(flagNameEnv)
	if err != nil {
		return fmt.Errorf("failed to get env flag: %w", err)
	}

	cfg, err := config.LoadConfig(configPath, env, logLevel)
	if err != nil {
		return fmt.Errorf("failed to load config: %w", err)
	}

	github := client.NewGithubClient(cfg.Github)

	server := NewServer(cfg.Server, github)

	return server.Run()
}
