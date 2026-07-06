use anyhow::{Context, Result};

use crate::config::{self, Config, EnvVar, ResolvedGlobalConfig, resolve_stacks};
use crate::portainer::{self, PortainerClient};
use crate::reporter::{EnvChange, Reporter};
use crate::ssh::SshClient;

pub fn diff_command(config_path: &str, stacks: &[String]) -> Result<()> {
    let (global_config, configs) = resolve_stacks(config_path, stacks)?;
    match &global_config {
        ResolvedGlobalConfig::Portainer(p) => {
            let client = portainer::PortainerClient::new(&p.host, &p.api_key);
            for config in &configs {
                diff_portainer(config, &client)?;
            }
        }
        ResolvedGlobalConfig::Ssh(s) => {
            let client = SshClient::new(s);
            for config in &configs {
                diff_ssh(config, &client)?;
            }
        }
    }
    Ok(())
}

fn diff_portainer(config: &Config, client: &PortainerClient) -> Result<()> {
    let compose_path = config.compose_path();
    let local_compose = std::fs::read_to_string(&compose_path).context(format!(
        "Failed to read compose file: {}",
        compose_path.display()
    ))?;
    let local_env = match config.env_path() {
        Some(path) => config::parse_env_file(&path)?,
        None => vec![],
    };

    match client.find_stack_by_name(&config.name)? {
        Some(existing) => {
            let remote_compose = client.get_stack_file(existing.id)?;
            report_diff(
                &config.name,
                &remote_compose,
                &local_compose,
                &existing.env,
                &local_env,
            );
        }
        None => {
            Reporter::would_create(&config.name);
        }
    }

    Ok(())
}

fn diff_ssh(config: &Config, client: &SshClient) -> Result<()> {
    let compose_path = config.compose_path();
    let local_compose = std::fs::read_to_string(&compose_path).context(format!(
        "Failed to read compose file: {}",
        compose_path.display()
    ))?;
    let local_env = match config.env_path() {
        Some(path) => config::parse_env_file(&path)?,
        None => vec![],
    };

    if !client.stack_exists(&config.name)? {
        Reporter::would_create(&config.name);
        return Ok(());
    }

    let remote_compose = client.get_compose_content(&config.name)?;
    let remote_env = client
        .get_env_content(&config.name)?
        .map(|content| config::parse_env_str(&content))
        .unwrap_or_default();
    report_diff(
        &config.name,
        &remote_compose,
        &local_compose,
        &remote_env,
        &local_env,
    );

    Ok(())
}

fn report_diff(
    name: &str,
    remote_compose: &str,
    local_compose: &str,
    remote_env: &[EnvVar],
    local_env: &[EnvVar],
) {
    let compose_diff = unified_diff(remote_compose.trim_end(), local_compose.trim_end(), 3);
    let env_changes = diff_env(remote_env, local_env);

    if compose_diff.is_empty() && env_changes.is_empty() {
        Reporter::up_to_date(name);
        return;
    }

    Reporter::changed(name);
    Reporter::diff_details(&compose_diff, &env_changes);
}

/// Compare env vars by name; values are never included in the output.
/// Remote is the old state, local is the new state.
fn diff_env(remote: &[EnvVar], local: &[EnvVar]) -> Vec<EnvChange> {
    let mut changes = Vec::new();

    for var in local {
        match remote.iter().find(|r| r.name == var.name) {
            None => changes.push(EnvChange::Added(var.name.clone())),
            Some(r) if r.value != var.value => changes.push(EnvChange::Changed(var.name.clone())),
            Some(_) => {}
        }
    }
    for var in remote {
        if !local.iter().any(|l| l.name == var.name) {
            changes.push(EnvChange::Removed(var.name.clone()));
        }
    }

    changes
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum DiffOp {
    Equal,
    Delete,
    Insert,
}

/// Produce a git-style unified diff between old and new, with the given number
/// of context lines. Returns an empty Vec when the inputs are identical.
fn unified_diff(old: &str, new: &str, context: usize) -> Vec<String> {
    let old_lines: Vec<&str> = old.lines().collect();
    let new_lines: Vec<&str> = new.lines().collect();
    let ops = diff_ops(&old_lines, &new_lines);

    if !ops.iter().any(|(op, _)| *op != DiffOp::Equal) {
        return vec![];
    }

    // Group ops into hunks: runs of changes with surrounding context
    let mut output = Vec::new();
    let mut i = 0;
    while i < ops.len() {
        if ops[i].0 == DiffOp::Equal {
            i += 1;
            continue;
        }

        // Found a change; hunk starts `context` lines before it
        let hunk_start = i.saturating_sub(context);

        // Extend the hunk until we see a gap of equal lines longer than 2*context
        let mut hunk_end = i;
        let mut j = i;
        while j < ops.len() {
            if ops[j].0 != DiffOp::Equal {
                hunk_end = j + 1;
                j += 1;
            } else {
                let gap_start = j;
                while j < ops.len() && ops[j].0 == DiffOp::Equal {
                    j += 1;
                }
                if j < ops.len() && j - gap_start <= 2 * context {
                    continue; // gap is small, keep extending the hunk
                }
                break;
            }
        }
        let hunk_end_with_context = (hunk_end + context).min(ops.len());

        // Compute line numbers for the header
        let old_start = ops[..hunk_start]
            .iter()
            .filter(|(op, _)| *op != DiffOp::Insert)
            .count();
        let new_start = ops[..hunk_start]
            .iter()
            .filter(|(op, _)| *op != DiffOp::Delete)
            .count();
        let hunk = &ops[hunk_start..hunk_end_with_context];
        let old_count = hunk.iter().filter(|(op, _)| *op != DiffOp::Insert).count();
        let new_count = hunk.iter().filter(|(op, _)| *op != DiffOp::Delete).count();

        output.push(format!(
            "@@ -{},{} +{},{} @@",
            old_start + 1,
            old_count,
            new_start + 1,
            new_count
        ));
        for (op, line) in hunk {
            let prefix = match op {
                DiffOp::Equal => ' ',
                DiffOp::Delete => '-',
                DiffOp::Insert => '+',
            };
            output.push(format!("{}{}", prefix, line));
        }

        i = hunk_end_with_context;
    }

    output
}

/// LCS-based line diff. Returns the full edit script over both inputs.
fn diff_ops<'a>(old: &[&'a str], new: &[&'a str]) -> Vec<(DiffOp, &'a str)> {
    let n = old.len();
    let m = new.len();

    // DP table of LCS lengths
    let mut lcs = vec![vec![0usize; m + 1]; n + 1];
    for i in (0..n).rev() {
        for j in (0..m).rev() {
            lcs[i][j] = if old[i] == new[j] {
                lcs[i + 1][j + 1] + 1
            } else {
                lcs[i + 1][j].max(lcs[i][j + 1])
            };
        }
    }

    let mut ops = Vec::new();
    let (mut i, mut j) = (0, 0);
    while i < n && j < m {
        if old[i] == new[j] {
            ops.push((DiffOp::Equal, old[i]));
            i += 1;
            j += 1;
        } else if lcs[i + 1][j] >= lcs[i][j + 1] {
            ops.push((DiffOp::Delete, old[i]));
            i += 1;
        } else {
            ops.push((DiffOp::Insert, new[j]));
            j += 1;
        }
    }
    while i < n {
        ops.push((DiffOp::Delete, old[i]));
        i += 1;
    }
    while j < m {
        ops.push((DiffOp::Insert, new[j]));
        j += 1;
    }

    ops
}

#[cfg(test)]
mod tests {
    use super::*;

    fn var(name: &str, value: &str) -> EnvVar {
        EnvVar {
            name: name.to_string(),
            value: value.to_string(),
        }
    }

    #[test]
    fn test_unified_diff_identical() {
        assert!(unified_diff("a\nb\nc", "a\nb\nc", 3).is_empty());
    }

    #[test]
    fn test_unified_diff_change() {
        let diff = unified_diff("a\nb\nc", "a\nx\nc", 3);
        assert_eq!(diff, vec!["@@ -1,3 +1,3 @@", " a", "-b", "+x", " c"]);
    }

    #[test]
    fn test_unified_diff_addition() {
        let diff = unified_diff("a\nb", "a\nb\nc", 3);
        assert_eq!(diff, vec!["@@ -1,2 +1,3 @@", " a", " b", "+c"]);
    }

    #[test]
    fn test_unified_diff_removal() {
        let diff = unified_diff("a\nb\nc", "a\nc", 3);
        assert_eq!(diff, vec!["@@ -1,3 +1,2 @@", " a", "-b", " c"]);
    }

    #[test]
    fn test_unified_diff_limits_context() {
        let old = "1\n2\n3\n4\n5\n6\n7\n8\n9\n10";
        let new = "1\n2\n3\n4\n5\nX\n7\n8\n9\n10";
        let diff = unified_diff(old, new, 1);
        assert_eq!(diff, vec!["@@ -5,3 +5,3 @@", " 5", "-6", "+X", " 7"]);
    }

    #[test]
    fn test_unified_diff_separate_hunks() {
        let old = "1\n2\n3\n4\n5\n6\n7\n8\n9\n10";
        let new = "X\n2\n3\n4\n5\n6\n7\n8\n9\nY";
        let diff = unified_diff(old, new, 1);
        assert_eq!(
            diff,
            vec![
                "@@ -1,2 +1,2 @@",
                "-1",
                "+X",
                " 2",
                "@@ -9,2 +9,2 @@",
                " 9",
                "-10",
                "+Y",
            ]
        );
    }

    #[test]
    fn test_unified_diff_merges_close_hunks() {
        let old = "1\n2\n3\n4\n5";
        let new = "X\n2\n3\n4\nY";
        let diff = unified_diff(old, new, 3);
        // Gap of 3 equal lines <= 2*context, so a single hunk
        assert_eq!(
            diff,
            vec!["@@ -1,5 +1,5 @@", "-1", "+X", " 2", " 3", " 4", "-5", "+Y"]
        );
    }

    #[test]
    fn test_diff_env_no_changes() {
        let remote = vec![var("FOO", "bar")];
        let local = vec![var("FOO", "bar")];
        assert!(diff_env(&remote, &local).is_empty());
    }

    #[test]
    fn test_diff_env_added() {
        let remote = vec![var("FOO", "bar")];
        let local = vec![var("FOO", "bar"), var("NEW", "secret")];
        assert_eq!(
            diff_env(&remote, &local),
            vec![EnvChange::Added("NEW".to_string())]
        );
    }

    #[test]
    fn test_diff_env_removed() {
        let remote = vec![var("FOO", "bar"), var("OLD", "secret")];
        let local = vec![var("FOO", "bar")];
        assert_eq!(
            diff_env(&remote, &local),
            vec![EnvChange::Removed("OLD".to_string())]
        );
    }

    #[test]
    fn test_diff_env_changed_hides_value() {
        let remote = vec![var("SECRET", "old-value")];
        let local = vec![var("SECRET", "new-value")];
        let changes = diff_env(&remote, &local);
        assert_eq!(changes, vec![EnvChange::Changed("SECRET".to_string())]);
    }
}
