use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Deserialize)]
pub struct Config {
    pub name: String,
    pub compose_file: String,
    pub env_file: Option<String>,
    pub host: String,
    #[serde(default = "default_endpoint_id")]
    pub endpoint_id: u64,
    #[serde(skip)]
    pub base_dir: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvVar {
    pub name: String,
    pub value: String,
}

fn default_endpoint_id() -> u64 {
    2
}

impl Config {
    pub fn load(path: &Path) -> Result<Self> {
        let path = path.canonicalize().context(format!(
            "Config file not found: {}",
            path.display()
        ))?;
        let content =
            std::fs::read_to_string(&path).context(format!("Failed to read config file: {}", path.display()))?;
        let mut config: Config = toml::from_str(&content).context("Failed to parse config file")?;
        config.base_dir = path.parent().unwrap_or(Path::new(".")).to_path_buf();
        Ok(config)
    }

    pub fn compose_path(&self) -> PathBuf {
        self.base_dir.join(&self.compose_file)
    }

    pub fn env_path(&self) -> Option<PathBuf> {
        self.env_file.as_ref().map(|f| self.base_dir.join(f))
    }
}

pub fn parse_env_file(path: &Path) -> Result<Vec<EnvVar>> {
    let content =
        std::fs::read_to_string(path).context(format!("Failed to read env file: {}", path.display()))?;
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

#[cfg(test)]
mod tests {
    use super::*;

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
            EnvVar { name: "FOO".to_string(), value: "bar".to_string() },
            EnvVar { name: "BAZ".to_string(), value: "qux=123".to_string() },
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
    fn test_config_resolved_paths() {
        let dir = std::env::temp_dir().join("stack-sync-config-test");
        std::fs::create_dir_all(&dir).unwrap();
        let config_path = dir.join("stack-sync.toml");
        std::fs::write(
            &config_path,
            r#"
name = "test"
compose_file = "compose.yaml"
env_file = ".env"
host = "https://example.com"
"#,
        )
        .unwrap();

        let config = Config::load(&config_path).unwrap();
        assert_eq!(config.compose_path(), dir.canonicalize().unwrap().join("compose.yaml"));
        assert_eq!(config.env_path(), Some(dir.canonicalize().unwrap().join(".env")));

        std::fs::remove_file(&config_path).ok();
        std::fs::remove_dir(&dir).ok();
    }

    #[test]
    fn test_config_without_env_file() {
        let toml_str = r#"
name = "my-stack"
compose_file = "compose.yaml"
host = "https://portainer.example.com"
"#;
        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.env_file, None);
    }

    #[test]
    fn test_config_deserialize() {
        let toml_str = r#"
name = "my-stack"
compose_file = "compose.yaml"
env_file = ".env"
host = "https://portainer.example.com"
"#;
        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.name, "my-stack");
        assert_eq!(config.compose_file, "compose.yaml");
        assert_eq!(config.env_file, Some(".env".to_string()));
        assert_eq!(config.host, "https://portainer.example.com");
    }
}
