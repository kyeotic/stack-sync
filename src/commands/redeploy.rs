use anyhow::{Context, Result};

use crate::config::{Config, ResolvedGlobalConfig, resolve_stacks};
use crate::portainer::{self, PortainerClient};
use crate::reporter::Reporter;
use crate::ssh::SshClient;

pub fn redeploy_command(
    config_path: &str,
    stack: &str,
    dry_run: bool,
    verbose: bool,
) -> Result<()> {
    let (global_config, configs) = resolve_stacks(config_path, &[stack.to_string()])?;
    let config = &configs[0];
    match &global_config {
        ResolvedGlobalConfig::Portainer(p) => {
            let client = portainer::PortainerClient::new(&p.host, &p.api_key);
            if dry_run {
                redeploy_portainer_dry_run(config, &client, verbose)
            } else {
                redeploy_portainer(config, &client)
            }
        }
        ResolvedGlobalConfig::Ssh(s) => {
            let client = SshClient::new(s);
            if dry_run {
                redeploy_ssh_dry_run(config, &client, s, verbose)
            } else {
                redeploy_ssh(config, &client, s)
            }
        }
    }
}

fn redeploy_portainer_dry_run(
    config: &Config,
    client: &PortainerClient,
    verbose: bool,
) -> Result<()> {
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

fn redeploy_portainer(config: &Config, client: &PortainerClient) -> Result<()> {
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

fn redeploy_ssh_dry_run(
    config: &Config,
    client: &SshClient,
    ssh_config: &crate::config::SshGlobalConfig,
    verbose: bool,
) -> Result<()> {
    if !config.enabled {
        Reporter::disabled(&config.name);
        return Ok(());
    }

    let exists = client.stack_exists(&config.name)?;
    if exists {
        Reporter::would_redeploy(&config.name);
        if verbose {
            Reporter::ssh_stack_details(
                &ssh_config.host,
                &config.compose_file,
                0,
                None,
                &ssh_config.host_dir,
            );
        }
    } else {
        Reporter::not_found(&config.name);
    }

    Ok(())
}

fn redeploy_ssh(
    config: &Config,
    client: &SshClient,
    ssh_config: &crate::config::SshGlobalConfig,
) -> Result<()> {
    if !config.enabled {
        Reporter::disabled(&config.name);
        return Ok(());
    }

    if !client.stack_exists(&config.name)? {
        anyhow::bail!(
            "Stack '{}' not found on remote host {}. Use 'sync' to deploy it first.",
            config.name,
            client.host()
        );
    }

    Reporter::redeploying(&config.name);
    client.redeploy_stack(&config.name)?;
    Reporter::redeployed(&config.name, &ssh_config.host);

    Ok(())
}
