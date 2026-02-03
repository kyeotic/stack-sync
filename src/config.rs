use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Deserialize)]
pub struct ConfigFile {
    pub host: String,
    #[serde(default = "default_endpoint_id")]
    pub endpoint_id: u64,
    pub stacks: HashMap<String, StackEntry>,
}

#[derive(Debug, Deserialize)]
pub struct StackEntry {
    pub compose_file: String,
    pub env_file: Option<String>,
    pub endpoint_id: Option<u64>,
}

#[derive(Debug)]
pub struct Config {
    pub name: String,
    pub compose_file: String,
    pub env_file: Option<String>,
    pub host: String,
    pub endpoint_id: u64,
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

/// Find a config file by checking common names in the given directory
pub fn find_config_file(base_path: &Path) -> Result<PathBuf> {
    // If the path is a file, use it directly
    if base_path.is_file() {
        return Ok(base_path.to_path_buf());
    }
    
    // Otherwise treat it as a directory and check for config files
    let dir = if base_path.is_dir() {
        base_path
    } else {
        base_path.parent().unwrap_or(Path::new("."))
    };
    
    // Check for .stack-sync.toml first (hidden), then stack-sync.toml
    let dotfile = dir.join(".stack-sync.toml");
    if dotfile.exists() {
        return Ok(dotfile);
    }
    
    let regular = dir.join("stack-sync.toml");
    if regular.exists() {
        return Ok(regular);
    }
    
    // If neither exists, return the regular name as default (for error messages)
    Ok(regular)
}

impl ConfigFile {
    pub fn load(path: &Path) -> Result<Self> {
        let path = path
            .canonicalize()
            .context(format!("Config file not found: {}", path.display()))?;
        let content = std::fs::read_to_string(&path)
            .context(format!("Failed to read config file: {}", path.display()))?;
        let config: ConfigFile = toml::from_str(&content).context("Failed to parse config file")?;
        Ok(config)
    }

    pub fn resolve(&self, stack_name: &str, base_dir: &Path) -> Result<Config> {
        let entry = self
            .stacks
            .get(stack_name)
            .context(format!("Stack '{}' not found in config", stack_name))?;
        Ok(Config {
            name: stack_name.to_string(),
            compose_file: entry.compose_file.clone(),
            env_file: entry.env_file.clone(),
            host: self.host.clone(),
            endpoint_id: entry.endpoint_id.unwrap_or(self.endpoint_id),
            base_dir: base_dir.to_path_buf(),
        })
    }

    pub fn stack_names(&self) -> Vec<&str> {
        self.stacks.keys().map(|s| s.as_str()).collect()
    }

    pub fn base_dir(path: &Path) -> Result<PathBuf> {
        let path = path
            .canonicalize()
            .context(format!("Config file not found: {}", path.display()))?;
        Ok(path.parent().unwrap_or(Path::new(".")).to_path_buf())
    }
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
    fn test_config_file_load_and_resolve() {
        let dir = std::env::temp_dir().join("stack-sync-config-test");
        std::fs::create_dir_all(&dir).unwrap();
        let config_path = dir.join("stack-sync.toml");
        std::fs::write(
            &config_path,
            r#"
host = "https://example.com"

[stacks.my-stack]
compose_file = "compose.yaml"
env_file = ".env"
"#,
        )
        .unwrap();

        let config_file = ConfigFile::load(&config_path).unwrap();
        let base_dir = ConfigFile::base_dir(&config_path).unwrap();
        let config = config_file.resolve("my-stack", &base_dir).unwrap();
        assert_eq!(config.name, "my-stack");
        assert_eq!(config.compose_path(), base_dir.join("compose.yaml"));
        assert_eq!(config.env_path(), Some(base_dir.join(".env")));
        assert_eq!(config.endpoint_id, 2);

        std::fs::remove_file(&config_path).ok();
        std::fs::remove_dir(&dir).ok();
    }

    #[test]
    fn test_config_file_without_env_file() {
        let toml_str = r#"
host = "https://portainer.example.com"

[stacks.my-stack]
compose_file = "compose.yaml"
"#;
        let config_file: ConfigFile = toml::from_str(toml_str).unwrap();
        let config = config_file.resolve("my-stack", Path::new(".")).unwrap();
        assert_eq!(config.env_file, None);
    }

    #[test]
    fn test_config_file_per_stack_endpoint_override() {
        let toml_str = r#"
host = "https://portainer.example.com"
endpoint_id = 2

[stacks.default-stack]
compose_file = "compose.yaml"

[stacks.custom-stack]
compose_file = "other/compose.yaml"
endpoint_id = 5
"#;
        let config_file: ConfigFile = toml::from_str(toml_str).unwrap();
        let default = config_file
            .resolve("default-stack", Path::new("."))
            .unwrap();
        assert_eq!(default.endpoint_id, 2);
        let custom = config_file.resolve("custom-stack", Path::new(".")).unwrap();
        assert_eq!(custom.endpoint_id, 5);
    }

    #[test]
    fn test_config_file_stack_not_found() {
        let toml_str = r#"
host = "https://portainer.example.com"

[stacks.my-stack]
compose_file = "compose.yaml"
"#;
        let config_file: ConfigFile = toml::from_str(toml_str).unwrap();
        let result = config_file.resolve("nonexistent", Path::new("."));
        assert!(result.is_err());
    }

    #[test]
    fn test_config_file_stack_names() {
        let toml_str = r#"
host = "https://portainer.example.com"

[stacks.alpha]
compose_file = "a.yaml"

[stacks.beta]
compose_file = "b.yaml"
"#;
        let config_file: ConfigFile = toml::from_str(toml_str).unwrap();
        let mut names = config_file.stack_names();
        names.sort();
        assert_eq!(names, vec!["alpha", "beta"]);
    }

    #[test]
    fn test_find_config_file_dotfile_priority() {
        let dir = std::env::temp_dir().join("stack-sync-find-test");
        std::fs::create_dir_all(&dir).unwrap();

        // Create both config files
        let dotfile = dir.join(".stack-sync.toml");
        let regular = dir.join("stack-sync.toml");
        std::fs::write(&dotfile, "# dotfile").unwrap();
        std::fs::write(&regular, "# regular").unwrap();

        // Should prefer the dotfile
        let found = find_config_file(&dir).unwrap();
        assert_eq!(found, dotfile);

        std::fs::remove_file(&dotfile).ok();
        std::fs::remove_file(&regular).ok();
        std::fs::remove_dir(&dir).ok();
    }

    #[test]
    fn test_find_config_file_fallback_to_regular() {
        let dir = std::env::temp_dir().join("stack-sync-find-test2");
        std::fs::create_dir_all(&dir).unwrap();

        // Create only the regular file
        let regular = dir.join("stack-sync.toml");
        std::fs::write(&regular, "# regular").unwrap();

        // Should find the regular file
        let found = find_config_file(&dir).unwrap();
        assert_eq!(found, regular);

        std::fs::remove_file(&regular).ok();
        std::fs::remove_dir(&dir).ok();
    }

    #[test]
    fn test_find_config_file_direct_path() {
        let dir = std::env::temp_dir().join("stack-sync-find-test3");
        std::fs::create_dir_all(&dir).unwrap();

        // Create a custom-named file
        let custom = dir.join("my-custom-config.toml");
        std::fs::write(&custom, "# custom").unwrap();

        // Should return the exact path when it's a file
        let found = find_config_file(&custom).unwrap();
        assert_eq!(found, custom);

        std::fs::remove_file(&custom).ok();
        std::fs::remove_dir(&dir).ok();
    }}