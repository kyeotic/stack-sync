use anyhow::{Context, Result};

use crate::config::{self, Config, ResolvedGlobalConfig, resolve_stacks};
use crate::portainer::{self, PortainerClient};
use crate::reporter::Reporter;
use crate::ssh::SshClient;

pub fn sync_command(
    config_path: &str,
    stacks: &[String],
    dry_run: bool,
    verbose: bool,
    redeploy: bool,
) -> Result<()> {
    let (global_config, configs) = resolve_stacks(config_path, stacks)?;
    match &global_config {
        ResolvedGlobalConfig::Portainer(p) => {
            for config in &configs {
                let client = portainer::PortainerClient::new(&p.host, &p.api_key);
                if dry_run {
                    sync_portainer_dry_run(config, &client, verbose, redeploy)?;
                } else {
                    sync_portainer(config, &client, redeploy)?;
                }
            }
        }
        ResolvedGlobalConfig::Ssh(s) => {
            let client = SshClient::new(s);
            for config in &configs {
                if dry_run {
                    sync_ssh_dry_run(config, &client, s, verbose, redeploy)?;
                } else {
                    sync_ssh(config, &client, s, redeploy)?;
                }
            }
        }
    }
    Ok(())
}

fn sync_portainer_dry_run(
    config: &Config,
    client: &PortainerClient,
    verbose: bool,
    redeploy: bool,
) -> Result<()> {
    if !config.enabled {
        match client.find_stack_by_name(&config.name)? {
            Some(existing) if existing.status == 1 => {
                Reporter::would_stop(&config.name, existing.id);
            }
            Some(_) => {
                Reporter::already_stopped(&config.name);
            }
            None => {
                Reporter::disabled(&config.name);
            }
        }
        return Ok(());
    }

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
            let orphans =
                client.find_orphans(config.endpoint_id, &config.name, &compose_content)?;
            if remote_compose.trim_end() == compose_content.trim_end() && existing.env == env_vars {
                if redeploy {
                    Reporter::would_redeploy(&config.name);
                } else if !orphans.is_empty() {
                    Reporter::would_update(&config.name, existing.id);
                } else {
                    Reporter::up_to_date(&config.name);
                }
            } else {
                Reporter::would_update(&config.name, existing.id);
            }
            Reporter::orphans(&orphans);
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

fn sync_portainer(config: &Config, client: &PortainerClient, redeploy: bool) -> Result<()> {
    if !config.enabled {
        match client.find_stack_by_name(&config.name)? {
            Some(existing) if existing.status == 1 => {
                Reporter::stopping(&config.name);
                let stack = client.stop_stack(existing.id, config.endpoint_id)?;
                Reporter::stopped(&stack.name, stack.id);
            }
            Some(_) => {
                Reporter::already_stopped(&config.name);
            }
            None => {
                Reporter::disabled(&config.name);
            }
        }
        return Ok(());
    }

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
            let needs_update =
                remote_compose.trim_end() != compose_content.trim_end() || existing.env != env_vars;
            let was_inactive = existing.status == 2;

            if needs_update {
                Reporter::updating(&config.name);
                let stack = client.update_stack(
                    existing.id,
                    config.endpoint_id,
                    &compose_content,
                    env_vars,
                    true,
                    true,
                )?;
                Reporter::updated(&stack.name, stack.id);
            } else if was_inactive {
                Reporter::starting(&config.name);
                let stack = client.start_stack(existing.id, config.endpoint_id)?;
                Reporter::started(&stack.name, stack.id);
            } else if redeploy {
                Reporter::redeploying(&config.name);
                let stack = client.update_stack(
                    existing.id,
                    config.endpoint_id,
                    &compose_content,
                    env_vars,
                    true,
                    true,
                )?;
                Reporter::redeployed(&stack.name, stack.id);
            } else if !client
                .find_orphans(config.endpoint_id, &config.name, &compose_content)?
                .is_empty()
            {
                // Unchanged file, but containers from removed services linger
                Reporter::updating(&config.name);
                let stack = client.update_stack(
                    existing.id,
                    config.endpoint_id,
                    &compose_content,
                    env_vars,
                    true,
                    true,
                )?;
                Reporter::updated(&stack.name, stack.id);
            } else {
                Reporter::up_to_date(&config.name);
            }
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

fn sync_ssh_dry_run(
    config: &Config,
    client: &SshClient,
    ssh_config: &config::SshGlobalConfig,
    verbose: bool,
    redeploy: bool,
) -> Result<()> {
    if !config.enabled {
        let exists = client.stack_exists(&config.name)?;
        if exists {
            let running = client.stack_is_running(&config.name)?;
            if running {
                Reporter::would_stop(&config.name, client.host());
            } else {
                Reporter::already_stopped(&config.name);
            }
        } else {
            Reporter::disabled(&config.name);
        }
        return Ok(());
    }

    let compose_path = config.compose_path();
    let compose_content = std::fs::read_to_string(&compose_path).context(format!(
        "Failed to read compose file: {}",
        compose_path.display()
    ))?;
    let env_content = match config.env_path() {
        Some(path) => Some(
            std::fs::read_to_string(&path)
                .context(format!("Failed to read env file: {}", path.display()))?,
        ),
        None => None,
    };

    // Checked for new stacks too: containers from another source sharing the
    // project name would be removed by the first deploy
    let orphans = client.find_orphans(&config.name, &compose_content)?;
    let exists = client.stack_exists(&config.name)?;
    if exists {
        let remote_compose = client.get_compose_content(&config.name)?;
        let remote_env = client.get_env_content(&config.name)?;
        let compose_changed = remote_compose.trim_end() != compose_content.trim_end();
        let env_changed = remote_env.as_deref().map(|s| s.trim_end())
            != env_content.as_deref().map(|s| s.trim_end());

        if compose_changed || env_changed {
            Reporter::would_update(&config.name, client.host());
        } else {
            let running = client.stack_is_running(&config.name)?;
            if !running {
                Reporter::would_update(&config.name, client.host());
            } else if redeploy {
                Reporter::would_redeploy(&config.name);
            } else if !orphans.is_empty() {
                Reporter::would_update(&config.name, client.host());
            } else {
                Reporter::up_to_date(&config.name);
            }
        }
    } else {
        Reporter::would_create(&config.name);
    }
    Reporter::orphans(&orphans);

    if verbose {
        let env_info = config.env_path().map(|p| {
            let vars = config::parse_env_file(&p).unwrap_or_default();
            (p.display().to_string(), vars.len())
        });
        Reporter::ssh_stack_details(
            &ssh_config.host,
            compose_path.display(),
            compose_content.len(),
            env_info,
            &ssh_config.host_dir,
        );
    }

    Ok(())
}

fn sync_ssh(
    config: &Config,
    client: &SshClient,
    ssh_config: &config::SshGlobalConfig,
    redeploy: bool,
) -> Result<()> {
    if !config.enabled {
        let exists = client.stack_exists(&config.name)?;
        if exists {
            let running = client.stack_is_running(&config.name)?;
            if running {
                Reporter::stopping(&config.name);
                client.stop_stack(&config.name)?;
                Reporter::stopped(&config.name, &ssh_config.host);
            } else {
                Reporter::already_stopped(&config.name);
            }
        } else {
            Reporter::disabled(&config.name);
        }
        return Ok(());
    }

    let compose_path = config.compose_path();
    let compose_content = std::fs::read_to_string(&compose_path).context(format!(
        "Failed to read compose file: {}",
        compose_path.display()
    ))?;
    let env_content = match config.env_path() {
        Some(path) => Some(
            std::fs::read_to_string(&path)
                .context(format!("Failed to read env file: {}", path.display()))?,
        ),
        None => None,
    };

    let exists = client.stack_exists(&config.name)?;
    if exists {
        let remote_compose = client.get_compose_content(&config.name)?;
        let remote_env = client.get_env_content(&config.name)?;
        let compose_changed = remote_compose.trim_end() != compose_content.trim_end();
        let env_changed = remote_env.as_deref().map(|s| s.trim_end())
            != env_content.as_deref().map(|s| s.trim_end());
        let running = client.stack_is_running(&config.name)?;

        if compose_changed || env_changed {
            Reporter::updating(&config.name);
            client.deploy_stack(
                &config.name,
                &compose_content,
                env_content.as_deref(),
                redeploy,
            )?;
            Reporter::updated(&config.name, &ssh_config.host);
        } else if !running {
            Reporter::starting(&config.name);
            client.deploy_stack(
                &config.name,
                &compose_content,
                env_content.as_deref(),
                redeploy,
            )?;
            Reporter::started(&config.name, &ssh_config.host);
        } else if redeploy {
            Reporter::redeploying(&config.name);
            client.deploy_stack(&config.name, &compose_content, env_content.as_deref(), true)?;
            Reporter::redeployed(&config.name, &ssh_config.host);
        } else if !client
            .find_orphans(&config.name, &compose_content)?
            .is_empty()
        {
            // Unchanged file, but containers from removed services linger
            Reporter::updating(&config.name);
            client.deploy_stack(
                &config.name,
                &compose_content,
                env_content.as_deref(),
                false,
            )?;
            Reporter::updated(&config.name, &ssh_config.host);
        } else {
            Reporter::up_to_date(&config.name);
        }
    } else {
        Reporter::creating(&config.name);
        client.deploy_stack(
            &config.name,
            &compose_content,
            env_content.as_deref(),
            redeploy,
        )?;
        Reporter::created(&config.name, &ssh_config.host);
    }

    Ok(())
}
