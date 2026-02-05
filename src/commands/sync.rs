use anyhow::{Context, Result};
use std::path::Path;

use crate::config::{self, Config};
use crate::portainer::{self, PortainerClient};

use crate::stacks::resolve_stacks;
use crate::styles::AppStyles;

// ANSI color helpers
const BOLD: &str = "\x1b[1m";
const GREEN: &str = "\x1b[32m";
const YELLOW: &str = "\x1b[33m";
const CYAN: &str = "\x1b[36m";
const DIM: &str = "\x1b[2m";
const RESET: &str = "\x1b[0m";

pub fn sync_command(config_path: &str, stacks: &[String], dry_run: bool) -> Result<()> {
    let (api_key, configs) = resolve_stacks(&config_path, stacks)?;
    for config in &configs {
        let client = portainer::PortainerClient::new(&config.host, &api_key);
        if dry_run {
            sync_dry_run(config, &client)?;
        } else {
            sync(config, &client)?;
        }
    }
    Ok(())
}

fn sync_dry_run(config: &Config, client: &PortainerClient) -> Result<()> {
    println!(
        "\n {:>12} Previewing sync for stack {}",
        "[dry-run]".would_update(),
        config.name
    );
    // println!(
    //     "\n{BOLD}{CYAN}[dry-run]{RESET} Previewing sync for stack '{BOLD}{}{RESET}'",
    //     config.name
    // );

    let compose_path = config.compose_path();
    let compose_content = std::fs::read_to_string(&compose_path).context(format!(
        "Failed to read compose file: {}",
        compose_path.display()
    ))?;
    let env_vars = match config.env_path() {
        Some(path) => config::parse_env_file(&path)?,
        None => vec![],
    };

    println!("{BOLD}{CYAN}[dry-run]{RESET} Host:         {}", config.host);
    println!(
        "{BOLD}{CYAN}[dry-run]{RESET} Compose file: {} {DIM}({} bytes){RESET}",
        compose_path.display(),
        compose_content.len()
    );
    if let Some(env_path) = config.env_path() {
        println!(
            "{BOLD}{CYAN}[dry-run]{RESET} Env file:     {} {DIM}({} vars){RESET}",
            env_path.display(),
            env_vars.len()
        );
    } else {
        println!("{BOLD}{CYAN}[dry-run]{RESET} Env file:     {DIM}(none){RESET}");
    }

    println!(
        "{BOLD}{CYAN}[dry-run]{RESET} Endpoint ID:  {}",
        config.endpoint_id
    );

    match client.find_stack_by_name(&config.name)? {
        Some(existing) => {
            let remote_compose = client.get_stack_file(existing.id)?;
            if remote_compose.trim_end() == compose_content.trim_end() && existing.env == env_vars {
                println!(
                    "{BOLD}{GREEN}[dry-run]{RESET} Stack '{BOLD}{}{RESET}' is already in sync.",
                    config.name
                );
            } else {
                println!(
                    "{BOLD}{YELLOW}[dry-run]{RESET} Would {BOLD}update{RESET} existing stack '{BOLD}{}{RESET}' (id: {})",
                    existing.name, existing.id
                );
                if !env_vars.is_empty() {
                    println!("{BOLD}{CYAN}[dry-run]{RESET} ENV defined");
                }
            }
        }
        None => {
            println!(
                "{BOLD}{GREEN}[dry-run]{RESET} Would {BOLD}create{RESET} new stack '{BOLD}{}{RESET}'",
                config.name
            );
            if !env_vars.is_empty() {
                println!("{BOLD}{CYAN}[dry-run]{RESET} ENV defined");
            }
        }
    }

    Ok(())
}

fn sync(config: &Config, client: &PortainerClient) -> Result<()> {
    let compose_path = config.compose_path();
    let compose_content = std::fs::read_to_string(&compose_path).context(format!(
        "Failed to read compose file: {}",
        compose_path.display()
    ))?;
    let env_vars = match config.env_path() {
        Some(path) => config::parse_env_file(&path)?,
        None => vec![],
    };

    match client.find_stack_by_name(&config.name)? {
        Some(existing) => {
            let remote_compose = client.get_stack_file(existing.id)?;
            if remote_compose.trim_end() == compose_content.trim_end() && existing.env == env_vars {
                println!("Stack '{}' is already in sync.", config.name);
                return Ok(());
            }
            println!("Updating stack '{}'...", config.name);
            let stack = client.update_stack(
                existing.id,
                config.endpoint_id,
                &compose_content,
                env_vars,
                false,
                true,
            )?;
            println!("Stack '{}' updated (id: {})", stack.name, stack.id);
        }
        None => {
            println!("Creating stack '{}'...", config.name);
            let stack = client.create_stack(
                config.endpoint_id,
                &config.name,
                &compose_content,
                env_vars,
            )?;
            println!("Stack '{}' created (id: {})", stack.name, stack.id);
        }
    }

    Ok(())
}
