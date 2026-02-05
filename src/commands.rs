use anyhow::{Context, Result};
use std::path::Path;

use crate::config::{self, Config};
use crate::portainer::PortainerClient;

// ANSI color helpers
const BOLD: &str = "\x1b[1m";
// const GREEN: &str = "\x1b[32m";
const YELLOW: &str = "\x1b[33m";
const CYAN: &str = "\x1b[36m";
// const DIM: &str = "\x1b[2m";
const RESET: &str = "\x1b[0m";

mod sync;

pub use sync::*;

pub fn redeploy_dry_run(config: &Config, client: &PortainerClient) -> Result<()> {
    println!(
        "\n{BOLD}{CYAN}[dry-run]{RESET} Previewing redeploy for stack '{BOLD}{}{RESET}'",
        config.name
    );

    match client.find_stack_by_name(&config.name)? {
        Some(stack) => {
            let status = match stack.status {
                1 => "active",
                2 => "inactive",
                _ => "unknown",
            };
            println!(
                "{BOLD}{CYAN}[dry-run]{RESET} Stack found in Portainer (id: {})",
                stack.id
            );
            println!("{BOLD}{CYAN}[dry-run]{RESET} Current status: {}", status);
            println!(
                "{BOLD}{CYAN}[dry-run]{RESET} Endpoint ID: {}",
                stack.endpoint_id
            );
            println!("{BOLD}{CYAN}[dry-run]{RESET} Env vars: {}", stack.env.len());
            println!(
                "{BOLD}{YELLOW}[dry-run]{RESET} Would {BOLD}redeploy{RESET} stack '{BOLD}{}{RESET}' with prune=true, pull_image=true",
                config.name
            );
        }
        None => {
            println!(
                "{BOLD}{YELLOW}[dry-run]{RESET} Stack '{BOLD}{}{RESET}' not found in Portainer.",
                config.name
            );
            println!("{BOLD}{CYAN}[dry-run]{RESET} Use 'sync' to create the stack first.");
        }
    }

    Ok(())
}

pub fn redeploy(config: &Config, client: &PortainerClient) -> Result<()> {
    let stack = client.find_stack_by_name(&config.name)?.context(format!(
        "Stack '{}' not found in Portainer. Use 'sync' to create it first.",
        config.name
    ))?;

    println!("Redeploying stack '{}'...", config.name);

    let compose_content = client.get_stack_file(stack.id)?;

    let updated = client.update_stack(
        stack.id,
        stack.endpoint_id,
        &compose_content,
        stack.env.clone(),
        true,
        true,
    )?;

    println!("Stack '{}' redeployed (id: {})", updated.name, updated.id);

    Ok(())
}

pub fn view(config: &Config, client: &PortainerClient) -> Result<()> {
    let stack = client
        .find_stack_by_name(&config.name)?
        .context(format!("Stack '{}' not found", config.name))?;

    let status = match stack.status {
        1 => "active",
        2 => "inactive",
        _ => "unknown",
    };
    let stack_type = match stack.stack_type {
        1 => "Swarm",
        2 => "Compose",
        3 => "Kubernetes",
        _ => "unknown",
    };

    println!("Name:       {}", stack.name);
    println!("Id:         {}", stack.id);
    println!("Type:       {}", stack_type);
    println!("Status:     {}", status);
    println!("Endpoint:   {}", stack.endpoint_id);
    println!("Created by: {}", stack.created_by);
    println!("Created:    {}", format_timestamp(stack.creation_date));
    println!("Updated by: {}", stack.updated_by);
    println!("Updated:    {}", format_timestamp(stack.update_date));

    if !stack.env.is_empty() {
        println!("Env vars:   {}", stack.env.len());
    }

    Ok(())
}

pub fn import_stack(
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

pub fn init(
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

fn format_timestamp(ts: u64) -> String {
    if ts == 0 {
        return "n/a".to_string();
    }
    // Simple UTC formatting without pulling in chrono
    let secs = ts;
    let days = secs / 86400;
    let remaining = secs % 86400;
    let hours = remaining / 3600;
    let minutes = (remaining % 3600) / 60;

    // Approximate date from unix epoch (good enough for display)
    let (year, month, day) = days_to_ymd(days);
    format!(
        "{:04}-{:02}-{:02} {:02}:{:02} UTC",
        year, month, day, hours, minutes
    )
}

fn days_to_ymd(days: u64) -> (u64, u64, u64) {
    // Algorithm from http://howardhinnant.github.io/date_algorithms.html
    let z = days + 719468;
    let era = z / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y, m, d)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_timestamp_zero() {
        assert_eq!(format_timestamp(0), "n/a");
    }

    #[test]
    fn test_format_timestamp_epoch() {
        // 1970-01-01 00:00 UTC
        assert_eq!(format_timestamp(0), "n/a");
    }

    #[test]
    fn test_format_timestamp_known_date() {
        // 2020-04-20 18:00 UTC = 1587405600
        let result = format_timestamp(1587405600);
        assert_eq!(result, "2020-04-20 18:00 UTC");
    }

    #[test]
    fn test_format_timestamp_another_date() {
        // 2024-01-01 00:00 UTC = 1704067200
        let result = format_timestamp(1704067200);
        assert_eq!(result, "2024-01-01 00:00 UTC");
    }

    #[test]
    fn test_days_to_ymd_epoch() {
        assert_eq!(days_to_ymd(0), (1970, 1, 1));
    }

    #[test]
    fn test_days_to_ymd_known_date() {
        // 2020-04-20 is day 18372 from epoch
        assert_eq!(days_to_ymd(18372), (2020, 4, 20));
    }

    #[test]
    fn test_days_to_ymd_leap_year() {
        // 2000-02-29 is day 11016 from epoch
        assert_eq!(days_to_ymd(11016), (2000, 2, 29));
    }
}
