use anyhow::{Context, Result};
use std::process::Command;

use crate::config::SshGlobalConfig;

pub struct SshClient {
    host: String,
    user: Option<String>,
    key: Option<String>,
    host_dir: String,
}

impl SshClient {
    pub fn new(config: &SshGlobalConfig) -> Self {
        Self {
            host: config.host.clone(),
            user: config.ssh_user.clone(),
            key: config.ssh_key.as_ref().map(|k| shellexpand_tilde(k)),
            host_dir: config.host_dir.clone(),
        }
    }

    fn destination(&self) -> String {
        match &self.user {
            Some(user) => format!("{}@{}", user, self.host),
            None => self.host.clone(),
        }
    }

    fn ssh_args(&self) -> Vec<String> {
        let mut args = vec![];
        if let Some(ref key) = self.key {
            args.push("-i".to_string());
            args.push(key.clone());
        }
        args
    }

    fn stack_dir(&self, name: &str) -> String {
        format!("{}/{}", self.host_dir, name)
    }

    fn compose_file_path(&self, name: &str) -> String {
        format!("{}/compose.yaml", self.stack_dir(name))
    }

    fn env_file_path(&self, name: &str) -> String {
        format!("{}/.env", self.stack_dir(name))
    }

    pub fn run_ssh(&self, cmd: &str) -> Result<String> {
        let mut args = self.ssh_args();
        args.push(self.destination());
        args.push(cmd.to_string());

        let output = Command::new("ssh")
            .args(&args)
            .output()
            .context("Failed to execute ssh command")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!(
                "SSH command failed (exit {}): {}",
                output.status.code().unwrap_or(-1),
                stderr.trim()
            );
        }

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    fn read_remote_file(&self, path: &str) -> Result<String> {
        self.run_ssh(&format!("cat {}", path))
    }

    pub fn stack_exists(&self, name: &str) -> Result<bool> {
        let mut args = self.ssh_args();
        args.push(self.destination());
        args.push(format!("test -f {}", self.compose_file_path(name)));

        let output = Command::new("ssh")
            .args(&args)
            .output()
            .context("Failed to execute ssh command")?;

        Ok(output.status.success())
    }

    pub fn stack_is_running(&self, name: &str) -> Result<bool> {
        let dir = self.stack_dir(name);
        let mut args = self.ssh_args();
        args.push(self.destination());
        args.push(format!("cd {} && docker compose ps -q 2>/dev/null", dir));

        let output = Command::new("ssh")
            .args(&args)
            .output()
            .context("Failed to execute ssh command")?;

        if !output.status.success() {
            return Ok(false);
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        Ok(!stdout.trim().is_empty())
    }

    pub fn deploy_stack(
        &self,
        name: &str,
        compose_content: &str,
        env_content: Option<&str>,
    ) -> Result<()> {
        let dir = self.stack_dir(name);

        // Create directory
        self.run_ssh(&format!("mkdir -p {}", dir))?;

        // Write compose file via stdin to avoid temp files
        let compose_path = self.compose_file_path(name);
        self.write_remote_file(&compose_path, compose_content)?;

        // Write env file if provided
        if let Some(env) = env_content {
            let env_path = self.env_file_path(name);
            self.write_remote_file(&env_path, env)?;
        }

        // docker compose up -d
        self.run_ssh(&format!("cd {} && docker compose up -d", dir))?;

        Ok(())
    }

    fn write_remote_file(&self, remote_path: &str, content: &str) -> Result<()> {
        let mut args = self.ssh_args();
        args.push(self.destination());
        args.push(format!("cat > {}", remote_path));

        let mut child = Command::new("ssh")
            .args(&args)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .context("Failed to spawn ssh command")?;

        use std::io::Write;
        if let Some(ref mut stdin) = child.stdin {
            stdin
                .write_all(content.as_bytes())
                .context("Failed to write to ssh stdin")?;
        }
        // Drop stdin to close it so the remote cat exits
        drop(child.stdin.take());

        let output = child
            .wait_with_output()
            .context("Failed to wait for ssh command")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!(
                "Failed to write remote file {} (exit {}): {}",
                remote_path,
                output.status.code().unwrap_or(-1),
                stderr.trim()
            );
        }

        Ok(())
    }

    pub fn stop_stack(&self, name: &str) -> Result<()> {
        let dir = self.stack_dir(name);
        self.run_ssh(&format!("cd {} && docker compose down", dir))?;
        Ok(())
    }

    pub fn get_compose_content(&self, name: &str) -> Result<String> {
        self.read_remote_file(&self.compose_file_path(name))
    }

    pub fn get_env_content(&self, name: &str) -> Result<Option<String>> {
        let env_path = self.env_file_path(name);
        let mut args = self.ssh_args();
        args.push(self.destination());
        args.push(format!("test -f {} && cat {}", env_path, env_path));

        let output = Command::new("ssh")
            .args(&args)
            .output()
            .context("Failed to execute ssh command")?;

        if !output.status.success() {
            return Ok(None);
        }

        let content = String::from_utf8_lossy(&output.stdout).to_string();
        if content.trim().is_empty() {
            Ok(None)
        } else {
            Ok(Some(content))
        }
    }

    pub fn redeploy_stack(&self, name: &str) -> Result<()> {
        let dir = self.stack_dir(name);
        self.run_ssh(&format!(
            "cd {} && docker compose pull && docker compose up -d --force-recreate",
            dir
        ))?;
        Ok(())
    }

    pub fn docker_compose_ps(&self, name: &str) -> Result<String> {
        let dir = self.stack_dir(name);
        self.run_ssh(&format!("cd {} && docker compose ps", dir))
    }

    pub fn host(&self) -> &str {
        &self.host
    }
}

fn shellexpand_tilde(path: &str) -> String {
    if let Some(rest) = path.strip_prefix("~/")
        && let Ok(home) = std::env::var("HOME")
    {
        return format!("{}/{}", home, rest);
    }
    path.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config(user: Option<&str>, key: Option<&str>) -> SshGlobalConfig {
        SshGlobalConfig {
            host: "192.168.0.20".to_string(),
            ssh_user: user.map(String::from),
            ssh_key: key.map(String::from),
            host_dir: "/mnt/docker".to_string(),
        }
    }

    #[test]
    fn test_destination_with_user() {
        let client = SshClient::new(&test_config(Some("root"), None));
        assert_eq!(client.destination(), "root@192.168.0.20");
    }

    #[test]
    fn test_destination_without_user() {
        let client = SshClient::new(&test_config(None, None));
        assert_eq!(client.destination(), "192.168.0.20");
    }

    #[test]
    fn test_ssh_args_without_key() {
        let client = SshClient::new(&test_config(None, None));
        let args = client.ssh_args();
        assert!(args.is_empty());
    }

    #[test]
    fn test_ssh_args_with_key() {
        let client = SshClient::new(&test_config(None, Some("/home/user/.ssh/id_ed25519")));
        let args = client.ssh_args();
        assert_eq!(args, vec!["-i", "/home/user/.ssh/id_ed25519"]);
    }

    #[test]
    fn test_stack_dir() {
        let client = SshClient::new(&test_config(None, None));
        assert_eq!(client.stack_dir("my-app"), "/mnt/docker/my-app");
    }

    #[test]
    fn test_compose_file_path() {
        let client = SshClient::new(&test_config(None, None));
        assert_eq!(
            client.compose_file_path("my-app"),
            "/mnt/docker/my-app/compose.yaml"
        );
    }

    #[test]
    fn test_env_file_path() {
        let client = SshClient::new(&test_config(None, None));
        assert_eq!(client.env_file_path("my-app"), "/mnt/docker/my-app/.env");
    }

    #[test]
    fn test_shellexpand_tilde() {
        // Test with ~ prefix
        let expanded = shellexpand_tilde("~/test/path");
        if let Ok(home) = std::env::var("HOME") {
            assert_eq!(expanded, format!("{}/test/path", home));
        }

        // Test without ~ prefix (no change)
        assert_eq!(shellexpand_tilde("/absolute/path"), "/absolute/path");

        // Test with just ~
        assert_eq!(shellexpand_tilde("~"), "~");
    }
}
