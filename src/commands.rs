use anyhow::{Context, Result};
use std::path::Path;

use crate::config::{self, Config, EnvVar};
use crate::portainer::PortainerClient;

// ANSI color helpers
const BOLD: &str = "\x1b[1m";
const GREEN: &str = "\x1b[32m";
const YELLOW: &str = "\x1b[33m";
const CYAN: &str = "\x1b[36m";
const DIM: &str = "\x1b[2m";
const RESET: &str = "\x1b[0m";

pub fn sync_dry_run(config: &Config, client: &PortainerClient) -> Result<()> {
    println!(
        "\n{BOLD}{CYAN}[dry-run]{RESET} Previewing sync for stack '{BOLD}{}{RESET}'",
        config.name
    );

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
            println!(
                "{BOLD}{YELLOW}[dry-run]{RESET} Would {BOLD}update{RESET} existing stack '{BOLD}{}{RESET}' (id: {})",
                existing.name, existing.id
            );
        }
        None => {
            println!(
                "{BOLD}{GREEN}[dry-run]{RESET} Would {BOLD}create{RESET} new stack '{BOLD}{}{RESET}'",
                config.name
            );
        }
    }

    if !env_vars.is_empty() {
        println!("{BOLD}{CYAN}[dry-run]{RESET} ENV defined");
    }

    println!("{BOLD}{CYAN}[dry-run]{RESET} No changes were made.");
    Ok(())
}

pub fn sync(config: &Config, client: &PortainerClient) -> Result<()> {
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
            if remote_compose == compose_content && existing.env == env_vars {
                println!("Stack '{}' is already in sync.", config.name);
                return Ok(());
            }
            println!("Updating stack '{}'...", config.name);
            let stack =
                client.update_stack(existing.id, config.endpoint_id, &compose_content, env_vars)?;
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

pub fn pull(
    host: &str,
    stack_name: &str,
    file_path: &str,
    env_path: &str,
    api_key: &str,
) -> Result<()> {
    let client = PortainerClient::new(host, api_key);

    let stack = client
        .find_stack_by_name(stack_name)?
        .context(format!("Stack '{}' not found", stack_name))?;

    let file_content = client.get_stack_file(stack.id)?;
    std::fs::write(file_path, &file_content)
        .context(format!("Failed to write compose file: {}", file_path))?;
    println!("Wrote compose file to {}", file_path);

    let env_vars: Vec<EnvVar> = stack.env;
    config::write_env_file(Path::new(env_path), &env_vars)?;
    println!("Wrote env file to {}", env_path);

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
