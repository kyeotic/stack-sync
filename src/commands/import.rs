use anyhow::{Context, Result};
use std::path::Path;

use crate::config::{self, ResolvedGlobalConfig};
use crate::portainer::PortainerClient;
use crate::ssh::SshClient;

pub fn import_command(config_path: &str, stack: &str, force: bool) -> Result<()> {
    let path = Path::new(config_path);
    if !config::local_config_exists(path) {
        anyhow::bail!(
            "No config file found at '{}'. Run 'stack-sync init' first to create one.",
            config::local_config_path(path).display()
        );
    }
    let (global_config, _, local_config_path) = config::resolve_config_chain(path)?;
    match &global_config {
        ResolvedGlobalConfig::Portainer(p) => {
            import_portainer(&local_config_path, stack, &p.api_key, &p.host, force)
        }
        ResolvedGlobalConfig::Ssh(s) => {
            let client = SshClient::new(s);
            import_ssh(&local_config_path, stack, &client, force)
        }
    }
}

fn import_portainer(
    config_path: &Path,
    stack_name: &str,
    api_key: &str,
    host: &str,
    force: bool,
) -> Result<()> {
    let base_dir = config_path.parent().unwrap_or(Path::new("."));

    // Check if stack already exists in config
    if config::stack_exists_in_config(config_path, stack_name)? && !force {
        anyhow::bail!(
            "Stack '{}' already exists in config. Use --force to overwrite.",
            stack_name
        );
    }

    let client = PortainerClient::new(host, api_key);

    let stack = client
        .find_stack_by_name(stack_name)?
        .context(format!("Stack '{}' not found in Portainer", stack_name))?;

    // Define file paths
    let compose_filename = format!("{}.compose.yaml", stack_name);
    let env_filename = format!("{}.env", stack_name);
    let compose_path = base_dir.join(&compose_filename);
    let env_path = base_dir.join(&env_filename);

    // Check if files exist (unless force)
    if !force {
        if compose_path.exists() {
            anyhow::bail!(
                "Compose file '{}' already exists. Use --force to overwrite.",
                compose_path.display()
            );
        }
        if env_path.exists() && !stack.env.is_empty() {
            anyhow::bail!(
                "Env file '{}' already exists. Use --force to overwrite.",
                env_path.display()
            );
        }
    }

    // Fetch and write compose file
    let file_content = client.get_stack_file(stack.id)?;
    std::fs::write(&compose_path, &file_content).context(format!(
        "Failed to write compose file: {}",
        compose_path.display()
    ))?;
    println!("Wrote compose file to {}", compose_path.display());

    // Write env file if stack has env vars
    let env_file_ref = if !stack.env.is_empty() {
        config::write_env_file(&env_path, &stack.env)?;
        println!("Wrote env file to {}", env_path.display());
        Some(env_filename.as_str())
    } else {
        None
    };

    // Add stack to config
    config::append_stack_to_config(config_path, stack_name, &compose_filename, env_file_ref)?;
    println!("Added stack '{}' to config", stack_name);

    Ok(())
}

fn import_ssh(config_path: &Path, stack_name: &str, client: &SshClient, force: bool) -> Result<()> {
    let base_dir = config_path.parent().unwrap_or(Path::new("."));

    // Check if stack already exists in config
    if config::stack_exists_in_config(config_path, stack_name)? && !force {
        anyhow::bail!(
            "Stack '{}' already exists in config. Use --force to overwrite.",
            stack_name
        );
    }

    // Check if stack exists on remote
    if !client.stack_exists(stack_name)? {
        anyhow::bail!(
            "Stack '{}' not found on remote host {}",
            stack_name,
            client.host()
        );
    }

    // Define file paths
    let compose_filename = format!("{}.compose.yaml", stack_name);
    let env_filename = format!("{}.env", stack_name);
    let compose_path = base_dir.join(&compose_filename);
    let env_path = base_dir.join(&env_filename);

    // Check if files exist (unless force)
    if !force && compose_path.exists() {
        anyhow::bail!(
            "Compose file '{}' already exists. Use --force to overwrite.",
            compose_path.display()
        );
    }

    // Fetch and write compose file
    let compose_content = client.get_compose_content(stack_name)?;
    std::fs::write(&compose_path, &compose_content).context(format!(
        "Failed to write compose file: {}",
        compose_path.display()
    ))?;
    println!("Wrote compose file to {}", compose_path.display());

    // Fetch and write env file if it exists on remote
    let env_content = client.get_env_content(stack_name)?;
    let env_file_ref = if let Some(env) = env_content {
        if !force && env_path.exists() {
            anyhow::bail!(
                "Env file '{}' already exists. Use --force to overwrite.",
                env_path.display()
            );
        }
        std::fs::write(&env_path, &env)
            .context(format!("Failed to write env file: {}", env_path.display()))?;
        println!("Wrote env file to {}", env_path.display());
        Some(env_filename.as_str())
    } else {
        None
    };

    // Add stack to config
    config::append_stack_to_config(config_path, stack_name, &compose_filename, env_file_ref)?;
    println!("Added stack '{}' to config", stack_name);

    Ok(())
}
