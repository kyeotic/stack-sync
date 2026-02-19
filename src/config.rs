use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Deserialize, Clone, Default, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum DeployMode {
    #[default]
    Portainer,
    Ssh,
}

#[derive(Debug, Deserialize, Clone)]
pub struct StackEntry {
    pub compose_file: String,
    pub env_file: Option<String>,
    pub endpoint_id: Option<u64>,
    pub enabled: Option<bool>,
}

#[derive(Debug)]
pub struct Config {
    pub name: String,
    pub compose_file: String,
    pub env_file: Option<String>,
    pub host: String,
    pub endpoint_id: u64,
    pub enabled: bool,
    pub base_dir: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EnvVar {
    pub name: String,
    pub value: String,
}

fn default_endpoint_id() -> u64 {
    2
}

/// Partial config file for hierarchical resolution - all fields optional
#[derive(Debug, Deserialize, Clone, Default)]
pub struct PartialConfigFile {
    pub mode: Option<DeployMode>,
    pub portainer_api_key: Option<String>,
    pub host: Option<String>,
    pub endpoint_id: Option<u64>,
    pub ssh_user: Option<String>,
    pub ssh_key: Option<String>,
    pub host_dir: Option<String>,
    #[serde(default)]
    pub stacks: HashMap<String, StackEntry>,
}

/// Portainer-specific global config
#[derive(Debug)]
pub struct PortainerGlobalConfig {
    pub api_key: String,
    pub host: String,
    pub endpoint_id: u64,
}

/// SSH-specific global config
#[derive(Debug)]
pub struct SshGlobalConfig {
    pub host: String,
    pub ssh_user: Option<String>,
    pub ssh_key: Option<String>,
    pub host_dir: String,
}

/// Resolved global config with all required fields validated
#[derive(Debug)]
pub enum ResolvedGlobalConfig {
    Portainer(PortainerGlobalConfig),
    Ssh(SshGlobalConfig),
}

impl PartialConfigFile {
    pub fn resolve(
        &self,
        stack_name: &str,
        global: &ResolvedGlobalConfig,
        base_dir: &Path,
    ) -> Result<Config> {
        let entry = self
            .stacks
            .get(stack_name)
            .context(format!("Stack '{}' not found in config", stack_name))?;

        let (host, endpoint_id) = match global {
            ResolvedGlobalConfig::Portainer(p) => {
                (p.host.clone(), entry.endpoint_id.unwrap_or(p.endpoint_id))
            }
            ResolvedGlobalConfig::Ssh(s) => (s.host.clone(), 0),
        };

        Ok(Config {
            name: stack_name.to_string(),
            compose_file: entry.compose_file.clone(),
            env_file: entry.env_file.clone(),
            host,
            endpoint_id,
            enabled: entry.enabled.unwrap_or(true),
            base_dir: base_dir.to_path_buf(),
        })
    }

    pub fn stack_names(&self) -> Vec<&str> {
        self.stacks.keys().map(|s| s.as_str()).collect()
    }
}

/// Result of walking the config chain
struct ConfigChainResult {
    mode: Option<DeployMode>,
    api_key: Option<String>,
    host: Option<String>,
    endpoint_id: Option<u64>,
    ssh_user: Option<String>,
    ssh_key: Option<String>,
    host_dir: Option<String>,
    local_config: Option<PartialConfigFile>,
    local_config_path: Option<PathBuf>,
}

/// Walk up directories from start_dir to $HOME, collecting config values.
/// If explicit_local_file is provided, it is used as the local config instead of
/// the first .stack-sync.toml found during the walk.
/// Returns partial results - validation happens in resolve_config_chain().
fn walk_config_chain(
    start_dir: &Path,
    explicit_local_file: Option<&Path>,
) -> Result<ConfigChainResult> {
    let home_dir = std::env::var("HOME")
        .ok()
        .map(PathBuf::from)
        .and_then(|p| p.canonicalize().ok());

    // Start with env var for API key (highest priority)
    let mut api_key = std::env::var("PORTAINER_API_KEY").ok();
    let mut mode: Option<DeployMode> = None;
    let mut host: Option<String> = None;
    let mut endpoint_id: Option<u64> = None;
    let mut ssh_user: Option<String> = None;
    let mut ssh_key: Option<String> = None;
    let mut host_dir: Option<String> = None;
    let mut local_config: Option<PartialConfigFile> = None;
    let mut local_config_path: Option<PathBuf> = None;

    // If an explicit local file was provided, load it before the walk so the walk
    // won't replace it with a .stack-sync.toml found in the same directory.
    if let Some(explicit) = explicit_local_file {
        let content = std::fs::read_to_string(explicit).context(format!(
            "Failed to read config file: {}",
            explicit.display()
        ))?;
        let partial: PartialConfigFile = toml::from_str(&content).context(format!(
            "Failed to parse config file: {}",
            explicit.display()
        ))?;
        local_config = Some(partial);
        local_config_path = Some(explicit.to_path_buf());
    }

    // Canonicalize starting directory
    let start_canonical = start_dir
        .canonicalize()
        .context(format!("Directory not found: {}", start_dir.display()))?;

    let mut current = Some(start_canonical.as_path());

    while let Some(dir) = current {
        // Check if we've escaped $HOME via symlinks
        if let Some(ref home) = home_dir
            && !dir.starts_with(home)
            && local_config.is_some()
        {
            // We've escaped HOME and already have a local config, stop
            break;
        }

        let config_path = dir.join(".stack-sync.toml");
        if config_path.exists() {
            let content = std::fs::read_to_string(&config_path).context(format!(
                "Failed to read config file: {}",
                config_path.display()
            ))?;
            let partial: PartialConfigFile = toml::from_str(&content).context(format!(
                "Failed to parse config file: {}",
                config_path.display()
            ))?;

            // First config found becomes the local config (has stacks)
            if local_config.is_none() {
                local_config = Some(partial.clone());
                local_config_path = Some(config_path);
            }

            // Inherit values if not already set (earlier configs have priority)
            if mode.is_none() {
                mode = partial.mode;
            }
            if api_key.is_none() {
                api_key = partial.portainer_api_key;
            }
            if host.is_none() {
                host = partial.host;
            }
            if endpoint_id.is_none() {
                endpoint_id = partial.endpoint_id;
            }
            if ssh_user.is_none() {
                ssh_user = partial.ssh_user;
            }
            if ssh_key.is_none() {
                ssh_key = partial.ssh_key;
            }
            if host_dir.is_none() {
                host_dir = partial.host_dir;
            }

            // Early termination - mode-aware
            let resolved_mode = mode.clone().unwrap_or_default();
            let have_all = match resolved_mode {
                DeployMode::Portainer => {
                    api_key.is_some() && host.is_some() && endpoint_id.is_some()
                }
                DeployMode::Ssh => host.is_some() && host_dir.is_some(),
            };
            if have_all {
                break;
            }
        }

        // Stop at $HOME
        if let Some(ref home) = home_dir
            && dir == home.as_path()
        {
            break;
        }

        current = dir.parent();
    }

    Ok(ConfigChainResult {
        mode,
        api_key,
        host,
        endpoint_id,
        ssh_user,
        ssh_key,
        host_dir,
        local_config,
        local_config_path,
    })
}

/// Resolve the config chain and validate required fields.
/// Returns (ResolvedGlobalConfig, PartialConfigFile, config_path).
pub fn resolve_config_chain(
    start_path: &Path,
) -> Result<(ResolvedGlobalConfig, PartialConfigFile, PathBuf)> {
    // If path is a file, use its parent as the walk start and pass it as the explicit local file.
    // Otherwise treat the path as a directory.
    let (start_dir, explicit_local_file): (&Path, Option<&Path>) = if start_path.is_file() {
        (
            start_path.parent().unwrap_or(Path::new(".")),
            Some(start_path),
        )
    } else if start_path.is_dir() {
        (start_path, None)
    } else {
        // Path doesn't exist yet, try to use it as a directory
        (start_path, None)
    };

    let result = walk_config_chain(start_dir, explicit_local_file)?;

    let local_config = result
        .local_config
        .context("No config file found. Create a .stack-sync.toml file with stack definitions.")?;

    let local_config_path = result
        .local_config_path
        .expect("local_config_path should be set when local_config is set");

    let mode = result.mode.unwrap_or_default();

    let global = match mode {
        DeployMode::Portainer => {
            let api_key = result.api_key.context(
                "API key not found. Set PORTAINER_API_KEY environment variable or add \
                 'portainer_api_key' to a .stack-sync.toml config file.",
            )?;
            let host = result
                .host
                .context("Host not found. Add 'host' to a .stack-sync.toml config file.")?;
            let endpoint_id = result.endpoint_id.unwrap_or_else(default_endpoint_id);
            ResolvedGlobalConfig::Portainer(PortainerGlobalConfig {
                api_key,
                host,
                endpoint_id,
            })
        }
        DeployMode::Ssh => {
            let host = result
                .host
                .context("Host not found. Add 'host' to a .stack-sync.toml config file.")?;
            let host_dir = result.host_dir.context(
                "host_dir not found. Add 'host_dir' to a .stack-sync.toml config file for SSH mode.",
            )?;
            ResolvedGlobalConfig::Ssh(SshGlobalConfig {
                host,
                ssh_user: result.ssh_user,
                ssh_key: result.ssh_key,
                host_dir,
            })
        }
    };

    Ok((global, local_config, local_config_path))
}

impl Config {
    pub fn compose_path(&self) -> PathBuf {
        self.base_dir.join(&self.compose_file)
    }

    pub fn env_path(&self) -> Option<PathBuf> {
        self.env_file.as_ref().map(|f| self.base_dir.join(f))
    }
}

pub fn parse_env_file(path: &Path) -> Result<Vec<EnvVar>> {
    let content = std::fs::read_to_string(path)
        .context(format!("Failed to read env file: {}", path.display()))?;
    Ok(parse_env_str(&content))
}

pub fn parse_env_str(content: &str) -> Vec<EnvVar> {
    content
        .lines()
        .filter(|line| {
            let trimmed = line.trim();
            !trimmed.is_empty() && !trimmed.starts_with('#')
        })
        .filter_map(|line| {
            let (key, value) = line.split_once('=')?;
            Some(EnvVar {
                name: key.trim().to_string(),
                value: value.trim().to_string(),
            })
        })
        .collect()
}

pub fn write_env_file(path: &Path, vars: &[EnvVar]) -> Result<()> {
    let content: String = vars
        .iter()
        .map(|v| format!("{}={}", v.name, v.value))
        .collect::<Vec<_>>()
        .join("\n");
    std::fs::write(path, content).context(format!("Failed to write env file: {}", path.display()))
}

/// Check if a local config file exists in the given directory
pub fn local_config_exists(dir: &Path) -> bool {
    dir.join(".stack-sync.toml").exists()
}

/// Get the path to the local config file
pub fn local_config_path(dir: &Path) -> PathBuf {
    dir.join(".stack-sync.toml")
}

/// Append a stack entry to an existing config file
pub fn append_stack_to_config(
    config_path: &Path,
    stack_name: &str,
    compose_file: &str,
    env_file: Option<&str>,
) -> Result<()> {
    let content = std::fs::read_to_string(config_path).context(format!(
        "Failed to read config file: {}",
        config_path.display()
    ))?;

    let mut config: PartialConfigFile = toml::from_str(&content).context(format!(
        "Failed to parse config file: {}",
        config_path.display()
    ))?;

    let entry = StackEntry {
        compose_file: compose_file.to_string(),
        env_file: env_file.map(String::from),
        endpoint_id: None,
        enabled: None,
    };

    config.stacks.insert(stack_name.to_string(), entry);

    let new_content = serialize_config(&config)?;
    std::fs::write(config_path, new_content).context(format!(
        "Failed to write config file: {}",
        config_path.display()
    ))
}

/// Check if a stack exists in the config file
pub fn stack_exists_in_config(config_path: &Path, stack_name: &str) -> Result<bool> {
    let content = std::fs::read_to_string(config_path).context(format!(
        "Failed to read config file: {}",
        config_path.display()
    ))?;

    let config: PartialConfigFile = toml::from_str(&content).context(format!(
        "Failed to parse config file: {}",
        config_path.display()
    ))?;

    Ok(config.stacks.contains_key(stack_name))
}

/// Serialize a config file to TOML string
fn serialize_config(config: &PartialConfigFile) -> Result<String> {
    // Build the config manually to control ordering
    let mut lines = Vec::new();

    if let Some(ref mode) = config.mode {
        let mode_str = match mode {
            DeployMode::Portainer => "portainer",
            DeployMode::Ssh => "ssh",
        };
        lines.push(format!("mode = {:?}", mode_str));
    }
    if let Some(ref key) = config.portainer_api_key {
        lines.push(format!("portainer_api_key = {:?}", key));
    }
    if let Some(ref host) = config.host {
        lines.push(format!("host = {:?}", host));
    }
    if let Some(endpoint_id) = config.endpoint_id {
        lines.push(format!("endpoint_id = {}", endpoint_id));
    }
    if let Some(ref user) = config.ssh_user {
        lines.push(format!("ssh_user = {:?}", user));
    }
    if let Some(ref key) = config.ssh_key {
        lines.push(format!("ssh_key = {:?}", key));
    }
    if let Some(ref dir) = config.host_dir {
        lines.push(format!("host_dir = {:?}", dir));
    }

    // Sort stack names for deterministic output
    let mut stack_names: Vec<_> = config.stacks.keys().collect();
    stack_names.sort();

    for name in stack_names {
        let entry = &config.stacks[name];
        lines.push(String::new());
        lines.push(format!("[stacks.{}]", name));
        lines.push(format!("compose_file = {:?}", entry.compose_file));
        if let Some(ref env) = entry.env_file {
            lines.push(format!("env_file = {:?}", env));
        }
        if let Some(endpoint_id) = entry.endpoint_id {
            lines.push(format!("endpoint_id = {}", endpoint_id));
        }
        if entry.enabled == Some(false) {
            lines.push("enabled = false".to_string());
        }
    }

    Ok(lines.join("\n") + "\n")
}

/// Create a parent config file with Portainer credentials
pub fn write_parent_config(
    path: &Path,
    api_key: &str,
    host: &str,
    endpoint_id: Option<u64>,
) -> Result<()> {
    let config = PartialConfigFile {
        portainer_api_key: Some(api_key.to_string()),
        host: Some(host.to_string()),
        endpoint_id,
        ..Default::default()
    };

    let content = serialize_config(&config)?;
    std::fs::write(path, content)
        .context(format!("Failed to write config file: {}", path.display()))
}

/// Create a parent config file with SSH settings
pub fn write_ssh_parent_config(
    path: &Path,
    host: &str,
    host_dir: &str,
    ssh_user: Option<&str>,
    ssh_key: Option<&str>,
) -> Result<()> {
    let config = PartialConfigFile {
        mode: Some(DeployMode::Ssh),
        host: Some(host.to_string()),
        host_dir: Some(host_dir.to_string()),
        ssh_user: ssh_user.map(String::from),
        ssh_key: ssh_key.map(String::from),
        ..Default::default()
    };

    let content = serialize_config(&config)?;
    std::fs::write(path, content)
        .context(format!("Failed to write config file: {}", path.display()))
}

/// Create a local config file with example stack commented out
pub fn write_local_config_template(path: &Path) -> Result<()> {
    let content = r#"# Example stack configuration:
# [stacks.my-stack]
# compose_file = "my-stack.compose.yaml"
# env_file = "my-stack.env"
"#;
    std::fs::write(path, content)
        .context(format!("Failed to write config file: {}", path.display()))
}

pub fn resolve_stacks(
    config_path: &str,
    filter: &[String],
) -> Result<(ResolvedGlobalConfig, Vec<Config>)> {
    let path = Path::new(config_path);
    let (global_config, local_config, config_path) = resolve_config_chain(path)?;
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

    let configs: Result<Vec<Config>> = names
        .iter()
        .map(|name| local_config.resolve(name, &global_config, &base_dir))
        .collect();

    Ok((global_config, configs?))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn portainer_global() -> ResolvedGlobalConfig {
        ResolvedGlobalConfig::Portainer(PortainerGlobalConfig {
            api_key: "test_key".to_string(),
            host: "https://portainer.example.com".to_string(),
            endpoint_id: 2,
        })
    }

    #[test]
    fn test_parse_env_str_basic() {
        let input = "FOO=bar\nBAZ=qux";
        let vars = parse_env_str(input);
        assert_eq!(vars.len(), 2);
        assert_eq!(vars[0].name, "FOO");
        assert_eq!(vars[0].value, "bar");
        assert_eq!(vars[1].name, "BAZ");
        assert_eq!(vars[1].value, "qux");
    }

    #[test]
    fn test_parse_env_str_skips_comments_and_blanks() {
        let input = "# comment\nFOO=bar\n\n  # another\nBAZ=qux\n";
        let vars = parse_env_str(input);
        assert_eq!(vars.len(), 2);
    }

    #[test]
    fn test_parse_env_str_handles_values_with_equals() {
        let input = "URL=https://example.com?foo=bar";
        let vars = parse_env_str(input);
        assert_eq!(vars.len(), 1);
        assert_eq!(vars[0].name, "URL");
        assert_eq!(vars[0].value, "https://example.com?foo=bar");
    }

    #[test]
    fn test_parse_env_str_empty() {
        let vars = parse_env_str("");
        assert!(vars.is_empty());
    }

    #[test]
    fn test_env_file_round_trip() {
        let dir = std::env::temp_dir().join("stack-sync-test");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(".env.test");

        let vars = vec![
            EnvVar {
                name: "FOO".to_string(),
                value: "bar".to_string(),
            },
            EnvVar {
                name: "BAZ".to_string(),
                value: "qux=123".to_string(),
            },
        ];
        write_env_file(&path, &vars).unwrap();
        let parsed = parse_env_file(&path).unwrap();
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0].name, "FOO");
        assert_eq!(parsed[0].value, "bar");
        assert_eq!(parsed[1].name, "BAZ");
        assert_eq!(parsed[1].value, "qux=123");

        std::fs::remove_file(&path).ok();
        std::fs::remove_dir(&dir).ok();
    }

    #[test]
    fn test_partial_config_without_env_file() {
        let toml_str = r#"
[stacks.my-stack]
compose_file = "compose.yaml"
"#;
        let config: PartialConfigFile = toml::from_str(toml_str).unwrap();
        let global = portainer_global();
        let resolved = config.resolve("my-stack", &global, Path::new(".")).unwrap();
        assert_eq!(resolved.env_file, None);
    }

    #[test]
    fn test_partial_config_stack_not_found() {
        let toml_str = r#"
[stacks.my-stack]
compose_file = "compose.yaml"
"#;
        let config: PartialConfigFile = toml::from_str(toml_str).unwrap();
        let global = portainer_global();
        let result = config.resolve("nonexistent", &global, Path::new("."));
        assert!(result.is_err());
    }

    #[test]
    fn test_partial_config_stack_names() {
        let toml_str = r#"
[stacks.alpha]
compose_file = "a.yaml"

[stacks.beta]
compose_file = "b.yaml"
"#;
        let config: PartialConfigFile = toml::from_str(toml_str).unwrap();
        let mut names = config.stack_names();
        names.sort();
        assert_eq!(names, vec!["alpha", "beta"]);
    }

    #[test]
    fn test_partial_config_file_parses_all_fields() {
        let toml_str = r#"
portainer_api_key = "ptr_test123"
host = "https://portainer.example.com"
endpoint_id = 5

[stacks.my-stack]
compose_file = "compose.yaml"
env_file = ".env"
"#;
        let config: PartialConfigFile = toml::from_str(toml_str).unwrap();
        assert_eq!(config.portainer_api_key, Some("ptr_test123".to_string()));
        assert_eq!(
            config.host,
            Some("https://portainer.example.com".to_string())
        );
        assert_eq!(config.endpoint_id, Some(5));
        assert_eq!(config.stacks.len(), 1);
    }

    #[test]
    fn test_partial_config_file_parses_minimal() {
        let toml_str = r#"
[stacks.my-stack]
compose_file = "compose.yaml"
"#;
        let config: PartialConfigFile = toml::from_str(toml_str).unwrap();
        assert_eq!(config.portainer_api_key, None);
        assert_eq!(config.host, None);
        assert_eq!(config.endpoint_id, None);
        assert_eq!(config.stacks.len(), 1);
    }

    #[test]
    fn test_partial_config_file_resolve() {
        let toml_str = r#"
[stacks.my-stack]
compose_file = "compose.yaml"
env_file = ".env"
"#;
        let config: PartialConfigFile = toml::from_str(toml_str).unwrap();
        let global = ResolvedGlobalConfig::Portainer(PortainerGlobalConfig {
            api_key: "test_key".to_string(),
            host: "https://example.com".to_string(),
            endpoint_id: 2,
        });
        let resolved = config
            .resolve("my-stack", &global, Path::new("/test"))
            .unwrap();
        assert_eq!(resolved.name, "my-stack");
        assert_eq!(resolved.host, "https://example.com");
        assert_eq!(resolved.endpoint_id, 2);
    }

    #[test]
    fn test_partial_config_file_resolve_with_stack_endpoint_override() {
        let toml_str = r#"
[stacks.my-stack]
compose_file = "compose.yaml"
endpoint_id = 7
"#;
        let config: PartialConfigFile = toml::from_str(toml_str).unwrap();
        let global = ResolvedGlobalConfig::Portainer(PortainerGlobalConfig {
            api_key: "test_key".to_string(),
            host: "https://example.com".to_string(),
            endpoint_id: 2,
        });
        let resolved = config
            .resolve("my-stack", &global, Path::new("/test"))
            .unwrap();
        assert_eq!(resolved.endpoint_id, 7);
    }

    #[test]
    fn test_parse_ssh_mode_config() {
        let toml_str = r#"
mode = "ssh"
host = "192.168.0.20"
ssh_user = "root"
ssh_key = "~/.ssh/id_ed25519"
host_dir = "/mnt/app_config/docker"

[stacks.my-stack]
compose_file = "compose.yaml"
"#;
        let config: PartialConfigFile = toml::from_str(toml_str).unwrap();
        assert_eq!(config.mode, Some(DeployMode::Ssh));
        assert_eq!(config.host, Some("192.168.0.20".to_string()));
        assert_eq!(config.ssh_user, Some("root".to_string()));
        assert_eq!(config.ssh_key, Some("~/.ssh/id_ed25519".to_string()));
        assert_eq!(config.host_dir, Some("/mnt/app_config/docker".to_string()));
    }

    #[test]
    fn test_mode_defaults_to_portainer() {
        let toml_str = r#"
[stacks.my-stack]
compose_file = "compose.yaml"
"#;
        let config: PartialConfigFile = toml::from_str(toml_str).unwrap();
        assert_eq!(config.mode, None);
        // Default should be Portainer
        assert_eq!(config.mode.unwrap_or_default(), DeployMode::Portainer);
    }

    #[test]
    fn test_ssh_resolve_sets_endpoint_id_zero() {
        let toml_str = r#"
[stacks.my-stack]
compose_file = "compose.yaml"
"#;
        let config: PartialConfigFile = toml::from_str(toml_str).unwrap();
        let global = ResolvedGlobalConfig::Ssh(SshGlobalConfig {
            host: "192.168.0.20".to_string(),
            ssh_user: None,
            ssh_key: None,
            host_dir: "/mnt/docker".to_string(),
        });
        let resolved = config
            .resolve("my-stack", &global, Path::new("/test"))
            .unwrap();
        assert_eq!(resolved.endpoint_id, 0);
        assert_eq!(resolved.host, "192.168.0.20");
    }
}
