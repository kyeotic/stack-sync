use anyhow::{Context, Result};

use crate::config::{self, Config, resolve_stacks};
use crate::portainer::{self, PortainerClient};

use crate::reporter::Reporter;

pub fn sync_command(
    config_path: &str,
    stacks: &[String],
    dry_run: bool,
    verbose: bool,
) -> Result<()> {
    let (api_key, configs) = resolve_stacks(config_path, stacks)?;
    for config in &configs {
        let client = portainer::PortainerClient::new(&config.host, &api_key);
        if dry_run {
            sync_dry_run(config, &client, verbose)?;
        } else {
            sync(config, &client)?;
        }
    }
    Ok(())
}

fn sync_dry_run(config: &Config, client: &PortainerClient, verbose: bool) -> Result<()> {
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
                Reporter::up_to_date(&config.name);
            } else {
                Reporter::would_update(&config.name, existing.id);
            }
        }
        None => {
            Reporter::would_create(&config.name);
        }
    }

    if verbose {
        let env_info = config
            .env_path()
            .map(|p| (p.display().to_string(), env_vars.len()));
        Reporter::stack_details(
            &config.host,
            compose_path.display(),
            compose_content.len(),
            env_info,
            config.endpoint_id,
        );
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
                Reporter::up_to_date(&config.name);
                return Ok(());
            }
            Reporter::updating(&config.name);
            let stack = client.update_stack(
                existing.id,
                config.endpoint_id,
                &compose_content,
                env_vars,
                false,
                true,
            )?;
            Reporter::updated(&stack.name, stack.id);
        }
        None => {
            Reporter::creating(&config.name);
            let stack = client.create_stack(
                config.endpoint_id,
                &config.name,
                &compose_content,
                env_vars,
            )?;
            Reporter::created(&stack.name, stack.id);
        }
    }

    Ok(())
}
