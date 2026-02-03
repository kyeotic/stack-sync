mod commands;
mod config;
mod portainer;
mod update;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use std::path::Path;

#[derive(Parser)]
#[command(name = "stack-sync", version, about = "Deploy and manage Portainer stacks")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Create or update a stack in Portainer
    Sync {
        /// Path to the config file (default: stack-sync.toml)
        #[arg(default_value = "stack-sync.toml")]
        config: String,
        /// Preview what would happen without making changes
        #[arg(long)]
        dry_run: bool,
    },
    /// Show the state of a stack in Portainer
    View {
        /// Path to the config file (default: stack-sync.toml)
        #[arg(default_value = "stack-sync.toml")]
        config: String,
    },
    /// Pull a stack's compose file and env vars from Portainer
    Pull {
        /// Portainer hostname (e.g. https://portainer.example.com)
        #[arg(long)]
        host: String,
        /// Name of the stack in Portainer
        #[arg(long)]
        stack: String,
        /// Path to write the compose file to
        #[arg(long)]
        file: String,
        /// Path to write the env vars to
        #[arg(long)]
        env: String,
    },
    /// Upgrade to the latest version
    Upgrade,
}

fn get_api_key() -> Result<String> {
    std::env::var("PORTAINER_API_KEY").context(
        "PORTAINER_API_KEY environment variable is required. \
         Create an API key in Portainer under User Settings > Access Tokens.",
    )
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Sync { config: config_path, dry_run } => {
            let api_key = get_api_key()?;
            let config = config::Config::load(Path::new(&config_path))?;
            let client = portainer::PortainerClient::new(&config.host, &api_key);
            if dry_run {
                commands::sync_dry_run(&config, &client)
            } else {
                commands::sync(&config, &client)
            }
        }
        Commands::View { config: config_path } => {
            let api_key = get_api_key()?;
            let config = config::Config::load(Path::new(&config_path))?;
            let client = portainer::PortainerClient::new(&config.host, &api_key);
            commands::view(&config, &client)
        }
        Commands::Pull {
            host,
            stack,
            file,
            env,
        } => {
            let api_key = get_api_key()?;
            commands::pull(&host, &stack, &file, &env, &api_key)
        }
        Commands::Upgrade => update::upgrade(),
    }
}
