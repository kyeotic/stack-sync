use anyhow::{Context, Result};
use std::path::Path;

use crate::config;

#[allow(clippy::too_many_arguments)]
pub fn init_command(
    mode: &str,
    api_key: Option<&str>,
    host: &str,
    endpoint_id: Option<u64>,
    ssh_user: Option<&str>,
    ssh_key: Option<&str>,
    host_dir: Option<&str>,
    parent_dir: Option<&str>,
    force: bool,
) -> Result<()> {
    let parent = parent_dir
        .map(std::path::PathBuf::from)
        .or_else(|| std::env::var("HOME").ok().map(std::path::PathBuf::from))
        .context("Could not determine parent directory. Set --parent-dir or $HOME.")?;
    let local = std::env::current_dir().context("Could not determine current directory")?;

    match mode {
        "portainer" => {
            let api_key = api_key.context("--portainer-api-key is required for portainer mode")?;
            init_portainer(&parent, &local, api_key, host, endpoint_id, force)
        }
        "ssh" => {
            let host_dir = host_dir.context("--host-dir is required for ssh mode")?;
            init_ssh(&parent, &local, host, host_dir, ssh_user, ssh_key, force)
        }
        other => anyhow::bail!("Unknown mode '{}'. Use 'portainer' or 'ssh'.", other),
    }
}

fn init_portainer(
    parent_dir: &Path,
    local_dir: &Path,
    api_key: &str,
    host: &str,
    endpoint_id: Option<u64>,
    force: bool,
) -> Result<()> {
    let parent_config_path = parent_dir.join(".stack-sync.toml");
    let local_config_path = local_dir.join(".stack-sync.toml");

    check_dirs_differ(parent_dir, local_dir)?;
    check_existing_files(&parent_config_path, &local_config_path, force)?;

    config::write_parent_config(&parent_config_path, api_key, host, endpoint_id)?;
    println!("Created parent config at {}", parent_config_path.display());

    config::write_local_config_template(&local_config_path)?;
    println!("Created local config at {}", local_config_path.display());

    Ok(())
}

fn init_ssh(
    parent_dir: &Path,
    local_dir: &Path,
    host: &str,
    host_dir: &str,
    ssh_user: Option<&str>,
    ssh_key: Option<&str>,
    force: bool,
) -> Result<()> {
    let parent_config_path = parent_dir.join(".stack-sync.toml");
    let local_config_path = local_dir.join(".stack-sync.toml");

    check_dirs_differ(parent_dir, local_dir)?;
    check_existing_files(&parent_config_path, &local_config_path, force)?;

    config::write_ssh_parent_config(&parent_config_path, host, host_dir, ssh_user, ssh_key)?;
    println!("Created parent config at {}", parent_config_path.display());

    config::write_local_config_template(&local_config_path)?;
    println!("Created local config at {}", local_config_path.display());

    Ok(())
}

fn check_dirs_differ(parent_dir: &Path, local_dir: &Path) -> Result<()> {
    let parent_canonical = parent_dir
        .canonicalize()
        .unwrap_or_else(|_| parent_dir.to_path_buf());
    let local_canonical = local_dir
        .canonicalize()
        .unwrap_or_else(|_| local_dir.to_path_buf());

    if parent_canonical == local_canonical {
        anyhow::bail!(
            "Parent directory and current directory are the same ({}). \
             Use --parent-dir to specify a different parent directory.",
            parent_canonical.display()
        );
    }
    Ok(())
}

fn check_existing_files(
    parent_config_path: &Path,
    local_config_path: &Path,
    force: bool,
) -> Result<()> {
    if !force {
        if parent_config_path.exists() {
            anyhow::bail!(
                "Parent config '{}' already exists. Use --force to overwrite.",
                parent_config_path.display()
            );
        }
        if local_config_path.exists() {
            anyhow::bail!(
                "Local config '{}' already exists. Use --force to overwrite.",
                local_config_path.display()
            );
        }
    }
    Ok(())
}
