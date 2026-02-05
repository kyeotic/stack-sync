use anyhow::{Context, Ok, Result};
use clap::Parser;
use std::path::Path;

use crate::stacks::resolve_stacks;

mod commands;
mod config;
mod portainer;
mod stacks;
mod styles;
mod update;

#[derive(Parser)]
#[command(name = "stack-sync", about = "Deploy and manage Portainer stacks")]
enum Cli {
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
    /// Print version information
    Version,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli {
        Cli::Sync {
            stacks,
            config: config_path,
            dry_run,
        } => commands::sync_command(&config_path, &stacks, dry_run)?,
        Cli::View {
            stacks,
            config: config_path,
        } => {
            let (api_key, configs) = resolve_stacks(&config_path, &stacks)?;
            for config in &configs {
                let client = portainer::PortainerClient::new(&config.host, &api_key);
                commands::view(config, &client)?;
            }
        }
        Cli::Import {
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
            )?
        }
        Cli::Init {
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
            )?
        }
        Cli::Redeploy {
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
        }
        Cli::Upgrade => update::upgrade()?,
        Cli::Version => println!("stack-sync {}", env!("CARGO_PKG_VERSION")),
    }

    return Ok(());
}
