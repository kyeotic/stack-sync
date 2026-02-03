use anyhow::{Context, Result};
use serde::{Deserialize, Deserializer, Serialize};

use crate::config::EnvVar;

fn deserialize_null_as_default<'de, D, T>(deserializer: D) -> std::result::Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: Default + Deserialize<'de>,
{
    Ok(Option::deserialize(deserializer)?.unwrap_or_default())
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Stack {
    pub id: u64,
    pub name: String,
    pub endpoint_id: u64,
    #[serde(rename = "Type")]
    pub stack_type: u64,
    pub status: u64,
    #[serde(default, deserialize_with = "deserialize_null_as_default")]
    pub env: Vec<EnvVar>,
    #[serde(default, rename = "createdBy")]
    pub created_by: String,
    #[serde(default, rename = "creationDate")]
    pub creation_date: u64,
    #[serde(default, rename = "updatedBy")]
    pub updated_by: String,
    #[serde(default, rename = "updateDate")]
    pub update_date: u64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct StackFileResponse {
    pub stack_file_content: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateStackPayload {
    pub name: String,
    pub stack_file_content: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub env: Vec<EnvVar>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateStackPayload {
    pub stack_file_content: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub env: Vec<EnvVar>,
    pub prune: bool,
    pub pull_image: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stack_deserialize() {
        let json = r#"{
            "Id": 42,
            "Name": "my-stack",
            "EndpointId": 1,
            "Type": 2,
            "Status": 1,
            "Env": [{"name": "FOO", "value": "bar"}],
            "createdBy": "admin",
            "creationDate": 1587399600,
            "updatedBy": "bob",
            "updateDate": 1587399700
        }"#;
        let stack: Stack = serde_json::from_str(json).unwrap();
        assert_eq!(stack.id, 42);
        assert_eq!(stack.name, "my-stack");
        assert_eq!(stack.endpoint_id, 1);
        assert_eq!(stack.stack_type, 2);
        assert_eq!(stack.status, 1);
        assert_eq!(stack.env.len(), 1);
        assert_eq!(stack.env[0].name, "FOO");
        assert_eq!(stack.created_by, "admin");
    }

    #[test]
    fn test_stack_deserialize_minimal() {
        let json = r#"{
            "Id": 1,
            "Name": "test",
            "EndpointId": 1,
            "Type": 2,
            "Status": 1
        }"#;
        let stack: Stack = serde_json::from_str(json).unwrap();
        assert_eq!(stack.id, 1);
        assert!(stack.env.is_empty());
        assert_eq!(stack.created_by, "");
        assert_eq!(stack.creation_date, 0);
    }

    #[test]
    fn test_stack_file_response_deserialize() {
        let json = r#"{"StackFileContent": "version: '3'\nservices:\n  web:\n    image: nginx"}"#;
        let resp: StackFileResponse = serde_json::from_str(json).unwrap();
        assert!(resp.stack_file_content.contains("nginx"));
    }

    #[test]
    fn test_create_payload_serialize() {
        let payload = CreateStackPayload {
            name: "test".to_string(),
            stack_file_content: "version: '3'".to_string(),
            env: vec![EnvVar {
                name: "KEY".to_string(),
                value: "val".to_string(),
            }],
        };
        let json = serde_json::to_value(&payload).unwrap();
        assert_eq!(json["name"], "test");
        assert_eq!(json["stackFileContent"], "version: '3'");
        assert_eq!(json["env"][0]["name"], "KEY");
    }

    #[test]
    fn test_create_payload_serialize_empty_env() {
        let payload = CreateStackPayload {
            name: "test".to_string(),
            stack_file_content: "version: '3'".to_string(),
            env: vec![],
        };
        let json = serde_json::to_value(&payload).unwrap();
        assert!(json.get("env").is_none());
    }

    #[test]
    fn test_update_payload_serialize() {
        let payload = UpdateStackPayload {
            stack_file_content: "version: '3'".to_string(),
            env: vec![],
            prune: false,
            pull_image: true,
        };
        let json = serde_json::to_value(&payload).unwrap();
        assert_eq!(json["stackFileContent"], "version: '3'");
        assert_eq!(json["prune"], false);
        assert_eq!(json["pullImage"], true);
        assert!(json.get("env").is_none());
    }

    #[test]
    fn test_client_base_url() {
        let client = PortainerClient::new("https://portainer.example.com", "key");
        assert_eq!(client.base_url, "https://portainer.example.com/api");
    }

    #[test]
    fn test_client_base_url_strips_trailing_slash() {
        let client = PortainerClient::new("https://portainer.example.com/", "key");
        assert_eq!(client.base_url, "https://portainer.example.com/api");
    }
}

fn api_error(method: &str, path: &str, err: ureq::Error) -> anyhow::Error {
    match &err {
        ureq::Error::StatusCode(status) => {
            anyhow::anyhow!("{} {} failed (HTTP {})", method, path, status)
        }
        other => anyhow::anyhow!("{} {} failed: {}", method, path, other),
    }
}

pub struct PortainerClient {
    base_url: String,
    api_key: String,
    agent: ureq::Agent,
}

impl PortainerClient {
    pub fn new(host: &str, api_key: &str) -> Self {
        let base_url = format!("{}/api", host.trim_end_matches('/'));
        Self {
            base_url,
            api_key: api_key.to_string(),
            agent: ureq::Agent::new_with_defaults(),
        }
    }

    fn get(&self, path: &str) -> ureq::RequestBuilder<ureq::typestate::WithoutBody> {
        self.agent
            .get(&format!("{}{}", self.base_url, path))
            .header("X-API-Key", &self.api_key)
    }

    fn post(&self, path: &str) -> ureq::RequestBuilder<ureq::typestate::WithBody> {
        self.agent
            .post(&format!("{}{}", self.base_url, path))
            .header("X-API-Key", &self.api_key)
    }

    fn put(&self, path: &str) -> ureq::RequestBuilder<ureq::typestate::WithBody> {
        self.agent
            .put(&format!("{}{}", self.base_url, path))
            .header("X-API-Key", &self.api_key)
    }

    pub fn list_stacks(&self) -> Result<Vec<Stack>> {
        let stacks: Vec<Stack> = self
            .get("/stacks")
            .call()
            .map_err(|e| api_error("GET", "/stacks", e))?
            .body_mut()
            .read_json()
            .context("Failed to parse stacks response")?;
        Ok(stacks)
    }

    pub fn find_stack_by_name(&self, name: &str) -> Result<Option<Stack>> {
        let stacks = self.list_stacks()?;
        Ok(stacks.into_iter().find(|s| s.name == name))
    }

    pub fn get_stack_file(&self, id: u64) -> Result<String> {
        let path = format!("/stacks/{}/file", id);
        let resp: StackFileResponse = self
            .get(&path)
            .call()
            .map_err(|e| api_error("GET", &path, e))?
            .body_mut()
            .read_json()
            .context("Failed to parse stack file response")?;
        Ok(resp.stack_file_content)
    }

    pub fn create_stack(
        &self,
        endpoint_id: u64,
        name: &str,
        file_content: &str,
        env: Vec<EnvVar>,
    ) -> Result<Stack> {
        let payload = CreateStackPayload {
            name: name.to_string(),
            stack_file_content: file_content.to_string(),
            env,
        };
        let path = format!(
            "/stacks/create/standalone/string?endpointId={}",
            endpoint_id
        );
        let stack: Stack = self
            .post(&path)
            .send_json(&payload)
            .map_err(|e| api_error("POST", &path, e))?
            .body_mut()
            .read_json()
            .context("Failed to parse create stack response")?;
        Ok(stack)
    }

    pub fn update_stack(
        &self,
        id: u64,
        endpoint_id: u64,
        file_content: &str,
        env: Vec<EnvVar>,
    ) -> Result<Stack> {
        let payload = UpdateStackPayload {
            stack_file_content: file_content.to_string(),
            env,
            prune: false,
            pull_image: true,
        };
        let path = format!("/stacks/{}?endpointId={}", id, endpoint_id);
        let stack: Stack = self
            .put(&path)
            .send_json(&payload)
            .map_err(|e| api_error("PUT", &path, e))?
            .body_mut()
            .read_json()
            .context("Failed to parse update stack response")?;
        Ok(stack)
    }
}
