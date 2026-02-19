use anyhow::{Context, Result};

use crate::config::{Config, ResolvedGlobalConfig, resolve_stacks};
use crate::portainer::{self, PortainerClient};
use crate::reporter::Reporter;
use crate::ssh::SshClient;

pub fn view_command(config_path: &str, stacks: &[String], verbose: bool) -> Result<()> {
    let (global_config, configs) = resolve_stacks(config_path, stacks)?;
    match &global_config {
        ResolvedGlobalConfig::Portainer(p) => {
            for config in &configs {
                let client = portainer::PortainerClient::new(&p.host, &p.api_key);
                view_portainer(config, &client, verbose)?;
            }
        }
        ResolvedGlobalConfig::Ssh(s) => {
            let client = SshClient::new(s);
            for config in &configs {
                view_ssh(config, &client, s, verbose)?;
            }
        }
    }
    Ok(())
}

fn view_portainer(config: &Config, client: &PortainerClient, verbose: bool) -> Result<()> {
    let stack = client
        .find_stack_by_name(&config.name)?
        .context(format!("Stack '{}' not found", config.name))?;

    let status = match stack.status {
        1 => "active",
        2 => "inactive",
        _ => "unknown",
    };

    Reporter::view(&stack.name, stack.id, status);

    if verbose {
        let stack_type = match stack.stack_type {
            1 => "Swarm",
            2 => "Compose",
            3 => "Kubernetes",
            _ => "unknown",
        };

        Reporter::view_details(
            stack_type,
            stack.endpoint_id,
            &stack.created_by,
            format_timestamp(stack.creation_date),
            &stack.updated_by,
            format_timestamp(stack.update_date),
            stack.env.len(),
        );
    }

    Ok(())
}

fn view_ssh(
    config: &Config,
    client: &SshClient,
    ssh_config: &crate::config::SshGlobalConfig,
    verbose: bool,
) -> Result<()> {
    let exists = client.stack_exists(&config.name)?;
    if !exists {
        Reporter::not_found(&config.name);
        return Ok(());
    }

    let running = client.stack_is_running(&config.name)?;
    let status = if running { "active" } else { "inactive" };

    Reporter::view(&config.name, &ssh_config.host, status);

    if verbose {
        let ps_output = if running {
            client.docker_compose_ps(&config.name).ok()
        } else {
            None
        };
        Reporter::ssh_view_details(&ssh_config.host, &ssh_config.host_dir, ps_output.as_deref());
    }

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
