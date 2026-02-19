use anyhow::{Ok, Result};
use clap::Parser;

mod commands;
mod config;
mod portainer;
mod reporter;
mod ssh;
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
        /// Show detailed stack information
        #[arg(short = 'V', long)]
        verbose: bool,
    },
    /// Show the state of a stack in Portainer
    View {
        /// Stack names to show (default: all stacks)
        stacks: Vec<String>,
        /// Path to the config file
        #[arg(short = 'C', long, default_value = ".")]
        config: String,
        /// Show detailed stack information
        #[arg(short = 'V', long)]
        verbose: bool,
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
        /// Deploy mode: "portainer" or "ssh"
        #[arg(long, default_value = "portainer")]
        mode: String,
        /// Portainer API key (required for portainer mode)
        #[arg(long)]
        portainer_api_key: Option<String>,
        /// Hostname (e.g. https://portainer.example.com or 192.168.0.20)
        #[arg(long)]
        host: String,
        /// Endpoint ID (optional, defaults to 2, portainer mode only)
        #[arg(long)]
        endpoint_id: Option<u64>,
        /// SSH user (optional, ssh mode only)
        #[arg(long)]
        ssh_user: Option<String>,
        /// SSH key path (optional, ssh mode only)
        #[arg(long)]
        ssh_key: Option<String>,
        /// Remote host directory for stacks (required for ssh mode)
        #[arg(long)]
        host_dir: Option<String>,
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
        /// Show detailed stack information
        #[arg(short = 'V', long)]
        verbose: bool,
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
            verbose,
        } => commands::sync_command(&config, &stacks, dry_run, verbose)?,
        Cli::View {
            stacks,
            config,
            verbose,
        } => commands::view_command(&config, &stacks, verbose)?,
        Cli::Import {
            stack,
            config,
            force,
        } => commands::import_command(&config, &stack, force)?,
        Cli::Init {
            mode,
            portainer_api_key,
            host,
            endpoint_id,
            ssh_user,
            ssh_key,
            host_dir,
            parent_dir,
            force,
        } => commands::init_command(
            &mode,
            portainer_api_key.as_deref(),
            &host,
            endpoint_id,
            ssh_user.as_deref(),
            ssh_key.as_deref(),
            host_dir.as_deref(),
            parent_dir.as_deref(),
            force,
        )?,
        Cli::Redeploy {
            stack,
            config,
            dry_run,
            verbose,
        } => commands::redeploy_command(&config, &stack, dry_run, verbose)?,
        Cli::Upgrade => update::upgrade()?,
        Cli::Version => println!("stack-sync {}", env!("CARGO_PKG_VERSION")),
    }

    Ok(())
}
