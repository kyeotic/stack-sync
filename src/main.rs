use anyhow::{Ok, Result};
use clap::Parser;

mod commands;
mod config;
mod portainer;
mod reporter;
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
            config,
            dry_run,
        } => commands::sync_command(&config, &stacks, dry_run)?,
        Cli::View { stacks, config } => commands::view_command(&config, &stacks)?,
        Cli::Import {
            stack,
            config,
            force,
        } => commands::import_command(&config, &stack, force)?,
        Cli::Init {
            portainer_api_key,
            host,
            endpoint_id,
            parent_dir,
            force,
        } => commands::init_command(
            &portainer_api_key,
            &host,
            endpoint_id,
            parent_dir.as_deref(),
            force,
        )?,
        Cli::Redeploy {
            stack,
            config,
            dry_run,
        } => commands::redeploy_command(&config, &stack, dry_run)?,
        Cli::Upgrade => update::upgrade()?,
        Cli::Version => println!("stack-sync {}", env!("CARGO_PKG_VERSION")),
    }

    Ok(())
}
