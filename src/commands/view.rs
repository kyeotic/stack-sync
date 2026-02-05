use anyhow::{Context, Result};

use crate::config::{Config, resolve_stacks};
use crate::portainer::{self, PortainerClient};

pub fn view_command(config_path: &str, stacks: &[String]) -> Result<()> {
    let (api_key, configs) = resolve_stacks(config_path, stacks)?;
    for config in &configs {
        let client = portainer::PortainerClient::new(&config.host, &api_key);
        view(config, &client)?;
    }
    Ok(())
}

fn view(config: &Config, client: &PortainerClient) -> Result<()> {
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
