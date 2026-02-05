use anyhow::{Ok, Result};
use std::path::Path;

use crate::config;

pub fn resolve_stacks(
    config_path: &str,
    filter: &[String],
) -> Result<(String, Vec<config::Config>)> {
    let path = Path::new(config_path);
    let (global_config, local_config, config_path) = config::resolve_config_chain(path)?;
    let base_dir = config_path.parent().unwrap_or(Path::new(".")).to_path_buf();

    let names: Vec<String> = if filter.is_empty() {
        let mut names: Vec<String> = local_config
            .stack_names()
            .into_iter()
            .map(String::from)
            .collect();
        names.sort();
        names
    } else {
        filter.to_vec()
    };

    let configs: Result<Vec<config::Config>> = names
        .iter()
        .map(|name| local_config.resolve(name, &global_config, &base_dir))
        .collect();

    Ok((global_config.api_key, configs?))
}
