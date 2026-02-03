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
    /// Import a stack from Portainer into the local config
    Import {
        /// Name of the stack in Portainer to import
        stack: String,
        /// Path to the config file or directory
        #[arg(short = 'C', long, default_value = ".")]
        config: String,
        /// Overwrite existing files
        #[arg(long)]
        force: bool,
    },
    /// Initialize config files for stack-sync
    Init {
        /// Portainer API key
        #[arg(long)]
        portainer_api_key: String,
        /// Portainer hostname (e.g. https://portainer.example.com)
        #[arg(long)]
        host: String,
        /// Endpoint ID (optional, defaults to 2)
        #[arg(long)]
        endpoint_id: Option<u64>,
        /// Parent directory for global config (defaults to $HOME)
        #[arg(long)]
        parent_dir: Option<String>,
        /// Overwrite existing files
        #[arg(long)]
        force: bool,
    },
    /// Redeploy a stack to pull new images
    Redeploy {
        /// Stack name to redeploy (must exist in config)
        stack: String,
        /// Path to the config file
        #[arg(short = 'C', long, default_value = ".")]
        config: String,
        /// Preview what would happen without making changes
        #[arg(long)]
        dry_run: bool,
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
        Commands::Import {
            stack,
            config: config_path,
            force,
        } => {
            let path = Path::new(&config_path);
            if !config::local_config_exists(path) {
                anyhow::bail!(
                    "No config file found at '{}'. Run 'stack-sync init' first to create one.",
                    config::local_config_path(path).display()
                );
            }
            let (global_config, _, local_config_path) = config::resolve_config_chain(path)?;
            commands::import_stack(
                &local_config_path,
                &stack,
                &global_config.api_key,
                &global_config.host,
                force,
            )
        }
        Commands::Init {
            portainer_api_key,
            host,
            endpoint_id,
            parent_dir,
            force,
        } => {
            let parent = parent_dir
                .map(std::path::PathBuf::from)
                .or_else(|| std::env::var("HOME").ok().map(std::path::PathBuf::from))
                .context("Could not determine parent directory. Set --parent-dir or $HOME.")?;
            let local = std::env::current_dir().context("Could not determine current directory")?;
            commands::init(
                &parent,
                &local,
                &portainer_api_key,
                &host,
                endpoint_id,
                force,
            )
        }
        Commands::Redeploy {
            stack,
            config: config_path,
            dry_run,
        } => {
            let (api_key, configs) = resolve_stacks(&config_path, &[stack])?;
            let config = &configs[0];
            let client = portainer::PortainerClient::new(&config.host, &api_key);
            if dry_run {
                commands::redeploy_dry_run(config, &client)?;
            } else {
                commands::redeploy(config, &client)?;
            }
            Ok(())
        }
        Commands::Upgrade => update::upgrade(),
    }
}
