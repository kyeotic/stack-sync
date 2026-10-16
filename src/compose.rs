use anyhow::{Context, Result};
use yaml_rust2::{Yaml, YamlLoader};

/// A container that belongs to a compose project, as reported by Docker.
#[derive(Debug, Clone, PartialEq)]
pub struct ProjectContainer {
    pub name: String,
    pub service: String,
    pub state: String,
    pub oneoff: bool,
}

fn load(content: &str) -> Result<Yaml> {
    let mut docs = YamlLoader::load_from_str(content).context("Failed to parse compose file")?;
    if docs.is_empty() {
        Ok(Yaml::Null)
    } else {
        Ok(docs.swap_remove(0))
    }
}

/// Service names defined in the compose file. Returns None when the full set
/// can't be known locally: the file pulls in services via `include:`, or
/// defines none (compose would refuse to deploy it anyway).
pub fn defined_services(content: &str) -> Result<Option<Vec<String>>> {
    let doc = load(content)?;
    if !doc["include"].is_badvalue() {
        return Ok(None);
    }
    let services: Vec<String> = doc["services"]
        .as_hash()
        .map(|services| {
            services
                .keys()
                .filter_map(|key| match key {
                    Yaml::String(s) => Some(s.clone()),
                    Yaml::Integer(i) => Some(i.to_string()),
                    _ => None,
                })
                .collect()
        })
        .unwrap_or_default();
    Ok((!services.is_empty()).then_some(services))
}

/// The project name `docker compose` derives for a stack directory: the file's
/// top-level `name:` when set, otherwise the directory (stack) name. Interpolated
/// names are ignored since they can't be resolved locally.
pub fn project_name(stack_name: &str, content: &str) -> Result<String> {
    let doc = load(content)?;
    let name = match doc["name"].as_str().map(str::trim) {
        Some(name) if !name.is_empty() && !name.contains("${") => name,
        _ => stack_name,
    };
    Ok(normalize_project_name(name))
}

/// Normalize a project name the way compose does: lowercase, drop characters
/// outside `[a-z0-9_-]`, and trim leading `_`/`-`.
pub fn normalize_project_name(name: &str) -> String {
    let normalized: String = name
        .to_lowercase()
        .chars()
        .filter(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || *c == '-' || *c == '_')
        .collect();
    normalized.trim_start_matches(['-', '_']).to_string()
}

/// Containers `docker compose up --remove-orphans` would remove. Mirrors
/// compose's own check: a one-off (`compose run`) container is an orphan once
/// it has finished; any other container is one when its service is no longer
/// defined.
pub fn find_orphans(
    containers: Vec<ProjectContainer>,
    services: &[String],
) -> Vec<ProjectContainer> {
    containers
        .into_iter()
        .filter(|c| {
            if c.oneoff {
                c.state == "exited" || c.state == "dead"
            } else {
                !services.contains(&c.service)
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn container(name: &str, service: &str, state: &str, oneoff: bool) -> ProjectContainer {
        ProjectContainer {
            name: name.to_string(),
            service: service.to_string(),
            state: state.to_string(),
            oneoff,
        }
    }

    #[test]
    fn test_defined_services() {
        let content = "services:\n  web:\n    image: nginx\n  db:\n    image: postgres\n    profiles: [debug]\n";
        assert_eq!(
            defined_services(content).unwrap(),
            Some(vec!["web".to_string(), "db".to_string()])
        );
    }

    #[test]
    fn test_defined_services_with_include_is_unknown() {
        let content = "include:\n  - other.yaml\nservices:\n  web:\n    image: nginx\n";
        assert_eq!(defined_services(content).unwrap(), None);
    }

    #[test]
    fn test_defined_services_empty_is_unknown() {
        assert_eq!(defined_services("services:\n").unwrap(), None);
        assert_eq!(defined_services("").unwrap(), None);
    }

    #[test]
    fn test_defined_services_invalid_yaml() {
        assert!(defined_services("services: [unclosed").is_err());
    }

    #[test]
    fn test_project_name_falls_back_to_stack_name() {
        let content = "services:\n  web:\n    image: nginx\n";
        assert_eq!(project_name("My.Stack", content).unwrap(), "mystack");
    }

    #[test]
    fn test_project_name_uses_name_key() {
        let content = "name: Media\nservices:\n  web:\n    image: nginx\n";
        assert_eq!(project_name("my-stack", content).unwrap(), "media");
    }

    #[test]
    fn test_project_name_ignores_interpolated_name() {
        let content = "name: ${PROJECT}\nservices:\n  web:\n    image: nginx\n";
        assert_eq!(project_name("my-stack", content).unwrap(), "my-stack");
    }

    #[test]
    fn test_normalize_project_name() {
        assert_eq!(normalize_project_name("_My Stack-1"), "mystack-1");
        assert_eq!(normalize_project_name("media_server"), "media_server");
    }

    #[test]
    fn test_find_orphans() {
        let services = vec!["web".to_string(), "db".to_string()];
        let containers = vec![
            container("app-web-1", "web", "running", false),
            container("app-old-1", "old", "running", false),
            container("app-gone-1", "gone", "exited", false),
            container("app-web-run-1", "web", "running", true),
            container("app-web-run-2", "web", "exited", true),
        ];
        let names: Vec<String> = find_orphans(containers, &services)
            .into_iter()
            .map(|c| c.name)
            .collect();
        assert_eq!(names, vec!["app-old-1", "app-gone-1", "app-web-run-2"]);
    }
}
