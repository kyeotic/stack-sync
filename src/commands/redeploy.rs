use anyhow::{Context, Result};

use crate::config::{Config, resolve_stacks};
use crate::portainer::{self, PortainerClient};
use crate::reporter::Reporter;

pub fn redeploy_command(
    config_path: &str,
    stack: &str,
    dry_run: bool,
    verbose: bool,
) -> Result<()> {
    let (api_key, configs) = resolve_stacks(config_path, &[stack.to_string()])?;
    let config = &configs[0];
    let client = portainer::PortainerClient::new(&config.host, &api_key);
    if dry_run {
        redeploy_dry_run(config, &client, verbose)
    } else {
        redeploy(config, &client)
    }
}

fn redeploy_dry_run(config: &Config, client: &PortainerClient, verbose: bool) -> Result<()> {
    if !config.enabled {
        Reporter::disabled(&config.name);
        return Ok(());
    }

    match client.find_stack_by_name(&config.name)? {
        Some(stack) => {
            Reporter::would_redeploy(&config.name);
            if verbose {
                Reporter::stack_details(
                    &config.host,
                    &config.compose_file,
                    0,
                    None,
                    stack.endpoint_id,
                );
            }
        }
        None => {
            Reporter::not_found(&config.name);
        }
    }

    Ok(())
}

fn redeploy(config: &Config, client: &PortainerClient) -> Result<()> {
    if !config.enabled {
        Reporter::disabled(&config.name);
        return Ok(());
    }

    let stack = client.find_stack_by_name(&config.name)?.context(format!(
        "Stack '{}' not found in Portainer. Use 'sync' to create it first.",
        config.name
    ))?;

    Reporter::redeploying(&config.name);

    let compose_content = client.get_stack_file(stack.id)?;

    let updated = client.update_stack(
        stack.id,
        stack.endpoint_id,
        &compose_content,
        stack.env.clone(),
        true,
        true,
    )?;

    Reporter::redeployed(&updated.name, updated.id);

    Ok(())
}
