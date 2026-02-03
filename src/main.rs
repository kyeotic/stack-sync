mod commands;
mod config;
mod portainer;
mod update;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use std::path::Path;

#[derive(Parser)]
#[command(
    name = "stack-sync",
    version,
    about = "Deploy and manage Portainer stacks"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Create or update a stack in Portainer
    Sync {
        /// Stack names to deploy (default: all stacks)
        stacks: Vec<String>,
        /// Path to the config file
        #[arg(short = 'C', long, default_value = ".")]
        config: String,
        /// Preview what would happen without making changes
        #[arg(long)]
        dry_run: bool,
    },
    /// Show the state of a stack in Portainer
    View {
        /// Stack names to show (default: all stacks)
        stacks: Vec<String>,
        /// Path to the config file
        #[arg(short = 'C', long, default_value = ".")]
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

fn resolve_stacks(config_path: &str, filter: &[String]) -> Result<(String, Vec<config::Config>)> {
    let path = Path::new(config_path);
    let (global_config, local_config, config_path) = config::resolve_config_chain(path)?;
    let base_dir = config_path.parent().unwrap_or(Path::new(".")).to_path_buf();

    let names: Vec<String> = if filter.is_empty() {
        let mut names: Vec<String> = local_config
            .stack_names()
            .into_iter()
            .map(String::from)
            .collect();
        names.sort();
        names
    } else {
        filter.to_vec()
    };

    let configs: Result<Vec<config::Config>> = names
        .iter()
        .map(|name| local_config.resolve(name, &global_config, &base_dir))
        .collect();

    Ok((global_config.api_key, configs?))
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Sync {
            stacks,
            config: config_path,
            dry_run,
        } => {
            let (api_key, configs) = resolve_stacks(&config_path, &stacks)?;
            for config in &configs {
                let client = portainer::PortainerClient::new(&config.host, &api_key);
                if dry_run {
                    commands::sync_dry_run(config, &client)?;
                } else {
                    commands::sync(config, &client)?;
                }
            }
            Ok(())
        }
        Commands::View {
            stacks,
            config: config_path,
        } => {
            let (api_key, configs) = resolve_stacks(&config_path, &stacks)?;
            for config in &configs {
                let client = portainer::PortainerClient::new(&config.host, &api_key);
                commands::view(config, &client)?;
            }
            Ok(())
        }
        Commands::Pull {
            host,
            stack,
            file,
            env,
        } => {
            let api_key = std::env::var("PORTAINER_API_KEY").context(
                "PORTAINER_API_KEY environment variable is required for pull. \
                 Create an API key in Portainer under User Settings > Access Tokens.",
            )?;
            commands::pull(&host, &stack, &file, &env, &api_key)
        }
        Commands::Upgrade => update::upgrade(),
    }
}
