#![doc = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/README.md"))]
use clap::{Args, Parser, Subcommand};
use tracing::Level;

mod api;
mod client;
mod config;
mod server;
#[cfg(test)]
mod test;
#[cfg(any(test, feature = "e2e"))]
pub mod testutils;
mod types;
mod version;

/// Guard PRs from merging until all triggered checks have passed.
#[derive(Debug, Parser)]
#[clap(disable_version_flag = true)]
pub struct App {
    /// Global cli options
    #[clap(flatten)]
    pub global_opts: GlobalOpts,

    /// The subcommand to run
    #[clap(subcommand)]
    pub command: Command,
}

impl App {
    /// Run the application based on the provided command and options.
    pub async fn run(self) -> Result<(), String> {
        if let Command::Version = self.command {
            version::print_version_and_exit();
        }

        let config = config::Configuration::load(&self.global_opts.config)
            .map_err(|e| format!("Failed to load configuration: {e}"))?;

        let log_level = match self.global_opts.log {
            Some(level) => level,
            None => config.log_level,
        };
        set_log_level(&log_level);

        let client = client::Client::build(config.github)?;

        match self.command {
            Command::Server => {
                let server = server::Server::new(config.server);
                server.run(client).await?;
            }
            Command::Create { cli_opts } => {
                return client
                    .create_check_run(
                        cli_opts.app_installation_id,
                        &cli_opts.repo,
                        &cli_opts.commit,
                    )
                    .await;
            }
            Command::Refresh { cli_opts } => {
                let (uncompleted, own_run) = get_and_print_status(&cli_opts, &client).await?;
                if uncompleted == 0 {
                    println!("All check runs are completed, setting check-run to 'completed'");
                }
                if own_run.is_none() {
                    println!("No cerberus check-run found, creating a new one");
                }
                client
                    .update_check_run(
                        cli_opts.app_installation_id,
                        &cli_opts.repo,
                        &cli_opts.commit,
                        uncompleted,
                        own_run,
                    )
                    .await?;
                println!("Updated PR status");
            }
            Command::Status { cli_opts } => {
                get_and_print_status(&cli_opts, &client).await?;
            }
            Command::Version => {
                version::print_version_and_exit();
            }
        }
        Ok(())
    }
}

/// The available subcommands.
#[derive(Debug, Subcommand)]
pub enum Command {
    /// Run the bot and listen for webhook events on /webhook
    Server,
    /// Create a new pending status check for a commit
    Create {
        #[clap(flatten)]
        cli_opts: CLIOptions,
    },
    /// Refresh the state of the status check of a commit
    Refresh {
        #[clap(flatten)]
        cli_opts: CLIOptions,
    },
    /// Check the status of a commit
    Status {
        #[clap(flatten)]
        cli_opts: CLIOptions,
    },
    /// Print the version and exit
    Version,
}

// TODO: Consider testing the env option of clap
/// Gobal cli options used by all commands (except `version`).
#[derive(Debug, Args)]
pub struct GlobalOpts {
    /// Log level to use, overrides the level given in the config file
    #[clap(long, global = true)]
    pub log: Option<String>,

    /// Path to the config file
    #[clap(long, short, global = true, default_value = "/config/config.yaml")]
    pub config: String,
}

/// Addtional cli options used by the local client commands like `create`, `refresh`, and `status`.
#[derive(Debug, Args)]
pub struct CLIOptions {
    /// Github App installation ID
    #[clap(index = 1)]
    pub app_installation_id: u64,
    /// Repository in the format "owner/repo"
    #[clap(index = 2)]
    pub repo: String,
    /// Commit SHA to check
    #[clap(index = 3)]
    pub commit: String,
}

fn set_log_level(level: &str) {
    let level = match level.to_lowercase().as_str() {
        "error" => Level::ERROR,
        "warn" => Level::WARN,
        "info" => Level::INFO,
        "debug" => Level::DEBUG,
        _ => {
            eprintln!("Invalid log level: {level}. Defaulting to 'info'.");
            Level::INFO
        }
    };
    let logger = tracing_subscriber::fmt()
        .with_max_level(level)
        .with_ansi(false);
    #[cfg(not(test))]
    logger.init();

    // We can only init the logger once, but testing might call the parent function multiple times.
    #[cfg(test)]
    logger.try_init().unwrap_or_default();
}

async fn get_and_print_status(
    cli_opts: &CLIOptions,
    client: &client::Client,
) -> Result<(u32, Option<types::CheckRun>), String> {
    let (count, own_run) = client
        .get_check_run_status(
            cli_opts.app_installation_id,
            &cli_opts.repo,
            &cli_opts.commit,
        )
        .await?;
    println!("Waiting on '{count}' check runs to complete");
    if let Some(own_run) = own_run.clone() {
        println!(
            "Found {} check-run, status: '{}', conclusion: '{}'",
            types::CHECK_RUN_NAME,
            own_run.status,
            own_run.conclusion.unwrap_or("null".to_string())
        );
    } else {
        println!(
            "No {} check-run found for this commit",
            types::CHECK_RUN_NAME
        );
    };
    Ok((count, own_run))
}
