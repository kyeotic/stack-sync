use anyhow::{Context, Result};
use std::path::Path;

use crate::config;

pub fn init_command(
    api_key: &str,
    host: &str,
    endpoint_id: Option<u64>,
    parent_dir: Option<&str>,
    force: bool,
) -> Result<()> {
    let parent = parent_dir
        .map(std::path::PathBuf::from)
        .or_else(|| std::env::var("HOME").ok().map(std::path::PathBuf::from))
        .context("Could not determine parent directory. Set --parent-dir or $HOME.")?;
    let local = std::env::current_dir().context("Could not determine current directory")?;
    init(&parent, &local, api_key, host, endpoint_id, force)
}

fn init(
    parent_dir: &Path,
    local_dir: &Path,
    api_key: &str,
    host: &str,
    endpoint_id: Option<u64>,
    force: bool,
) -> Result<()> {
    let parent_config_path = parent_dir.join(".stack-sync.toml");
    let local_config_path = local_dir.join(".stack-sync.toml");

    // Check that parent and local are different directories
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

    // Check if files exist (unless force)
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

    // Create parent config with credentials
    config::write_parent_config(&parent_config_path, api_key, host, endpoint_id)?;
    println!("Created parent config at {}", parent_config_path.display());

    // Create local config with example
    config::write_local_config_template(&local_config_path)?;
    println!("Created local config at {}", local_config_path.display());

    Ok(())
}
