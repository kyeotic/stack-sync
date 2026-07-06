use owo_colors::{OwoColorize, Style};
use std::fmt::Display;

use crate::styles::{AnsiPadding, AppStyles};

#[derive(Debug, PartialEq)]
pub enum EnvChange {
    Added(String),
    Removed(String),
    Changed(String),
}

type ByteRange = std::ops::Range<usize>;

/// A classified unified-diff line. Paired variants carry the byte range of
/// the segment that differs from their counterpart line.
#[derive(Debug, PartialEq)]
enum DiffLine {
    Header(String),
    Context(String),
    Delete(String),
    Insert(String),
    PairedDelete(String, ByteRange),
    PairedInsert(String, ByteRange),
}

/// Classify raw unified-diff lines and pair up delete/insert runs of equal
/// length (like git's diff-highlight) so changed segments can be emphasized.
fn pair_diff_lines(lines: &[String]) -> Vec<DiffLine> {
    let mut result = Vec::with_capacity(lines.len());
    let mut i = 0;
    while i < lines.len() {
        let line = &lines[i];
        if !line.starts_with('-') {
            result.push(match line.chars().next() {
                Some('+') => DiffLine::Insert(line[1..].to_string()),
                Some('@') => DiffLine::Header(line.clone()),
                _ => DiffLine::Context(line.clone()),
            });
            i += 1;
            continue;
        }

        // Collect the delete run and any insert run that immediately follows
        let del_start = i;
        while i < lines.len() && lines[i].starts_with('-') {
            i += 1;
        }
        let ins_start = i;
        while i < lines.len() && lines[i].starts_with('+') {
            i += 1;
        }
        let deletes = &lines[del_start..ins_start];
        let inserts = &lines[ins_start..i];

        if deletes.len() == inserts.len() {
            let mut paired_inserts = Vec::with_capacity(inserts.len());
            for (del, ins) in deletes.iter().zip(inserts) {
                let (del_mid, ins_mid) = changed_segments(&del[1..], &ins[1..]);
                result.push(DiffLine::PairedDelete(del[1..].to_string(), del_mid));
                paired_inserts.push(DiffLine::PairedInsert(ins[1..].to_string(), ins_mid));
            }
            result.extend(paired_inserts);
        } else {
            result.extend(deletes.iter().map(|l| DiffLine::Delete(l[1..].to_string())));
            result.extend(inserts.iter().map(|l| DiffLine::Insert(l[1..].to_string())));
        }
    }
    result
}

/// Find the differing middle segment of two lines by trimming their common
/// prefix and suffix. Returns byte ranges into each line.
fn changed_segments(old: &str, new: &str) -> (ByteRange, ByteRange) {
    let prefix = old
        .char_indices()
        .zip(new.char_indices())
        .find(|((_, a), (_, b))| a != b)
        .map(|((i, _), _)| i)
        .unwrap_or_else(|| old.len().min(new.len()));

    let suffix = old[prefix..]
        .chars()
        .rev()
        .zip(new[prefix..].chars().rev())
        .take_while(|(a, b)| a == b)
        .map(|(a, _)| a.len_utf8())
        .sum::<usize>();

    (prefix..old.len() - suffix, prefix..new.len() - suffix)
}

pub struct Reporter;

impl Reporter {
    fn bold(text: &str) -> String {
        text.style_if_supported(Style::new().bold())
    }

    const ACTION_LABEL_WIDTH: usize = 15;

    // --- action labels ---

    pub fn would_update(name: &str, id: impl Display) {
        println!(
            " {} {} {}",
            "Would Update"
                .would_update()
                .align_right(Self::ACTION_LABEL_WIDTH),
            Self::bold(name),
            format!("(id: {})", id).dimmed()
        );
    }

    pub fn would_create(name: &str) {
        println!(
            " {} {}",
            "Would Create"
                .would_update()
                .align_right(Self::ACTION_LABEL_WIDTH),
            Self::bold(name)
        );
    }

    pub fn updating(name: &str) {
        println!(
            " {} {}...",
            "Updating".waiting().align_right(Self::ACTION_LABEL_WIDTH),
            Self::bold(name)
        );
    }

    pub fn updated(name: &str, id: impl Display) {
        println!(
            " {} {} {}",
            "Updated".updated().align_right(Self::ACTION_LABEL_WIDTH),
            Self::bold(name),
            format!("(id: {})", id).dimmed()
        );
    }

    pub fn creating(name: &str) {
        println!(
            " {} {}...",
            "Creating".waiting().align_right(Self::ACTION_LABEL_WIDTH),
            Self::bold(name)
        );
    }

    pub fn created(name: &str, id: impl Display) {
        println!(
            " {} {} {}",
            "Created".updated().align_right(Self::ACTION_LABEL_WIDTH),
            Self::bold(name),
            format!("(id: {})", id).dimmed()
        );
    }

    pub fn up_to_date(name: &str) {
        println!(
            " {} {}",
            "Up-to-Date"
                .up_to_date()
                .align_right(Self::ACTION_LABEL_WIDTH),
            Self::bold(name)
        );
    }

    pub fn would_redeploy(name: &str) {
        println!(
            " {} {}",
            "Would Redeploy"
                .would_update()
                .align_right(Self::ACTION_LABEL_WIDTH),
            Self::bold(name)
        );
    }

    pub fn redeploying(name: &str) {
        println!(
            " {} {}...",
            "Redeploying"
                .waiting()
                .align_right(Self::ACTION_LABEL_WIDTH),
            Self::bold(name)
        );
    }

    pub fn redeployed(name: &str, id: impl Display) {
        println!(
            " {} {} {}",
            "Redeployed".updated().align_right(Self::ACTION_LABEL_WIDTH),
            Self::bold(name),
            format!("(id: {})", id).dimmed()
        );
    }

    pub fn changed(name: &str) {
        println!(
            " {} {}",
            "Changed"
                .would_update()
                .align_right(Self::ACTION_LABEL_WIDTH),
            Self::bold(name)
        );
    }

    pub fn would_stop(name: &str, id: impl Display) {
        println!(
            " {} {} {}",
            "Would Stop"
                .would_update()
                .align_right(Self::ACTION_LABEL_WIDTH),
            Self::bold(name),
            format!("(id: {})", id).dimmed()
        );
    }

    pub fn stopping(name: &str) {
        println!(
            " {} {}...",
            "Stopping".waiting().align_right(Self::ACTION_LABEL_WIDTH),
            Self::bold(name)
        );
    }

    pub fn stopped(name: &str, id: impl Display) {
        println!(
            " {} {} {}",
            "Stopped".updated().align_right(Self::ACTION_LABEL_WIDTH),
            Self::bold(name),
            format!("(id: {})", id).dimmed()
        );
    }

    pub fn already_stopped(name: &str) {
        println!(
            " {} {}",
            "Already Stopped"
                .up_to_date()
                .align_right(Self::ACTION_LABEL_WIDTH),
            Self::bold(name)
        );
    }

    pub fn starting(name: &str) {
        println!(
            " {} {}...",
            "Starting".waiting().align_right(Self::ACTION_LABEL_WIDTH),
            Self::bold(name)
        );
    }

    pub fn started(name: &str, id: impl Display) {
        println!(
            " {} {} {}",
            "Started".updated().align_right(Self::ACTION_LABEL_WIDTH),
            Self::bold(name),
            format!("(id: {})", id).dimmed()
        );
    }

    pub fn disabled(name: &str) {
        println!(
            " {} {}",
            "Disabled"
                .up_to_date()
                .align_right(Self::ACTION_LABEL_WIDTH),
            Self::bold(name)
        );
    }

    pub fn not_found(name: &str) {
        println!(
            " {} {}",
            "Not Found"
                .would_update()
                .align_right(Self::ACTION_LABEL_WIDTH),
            Self::bold(name)
        );
    }

    pub fn view(name: &str, id: impl Display, status: &str) {
        println!(
            " {} {} {} {}",
            "View".up_to_date().align_right(Self::ACTION_LABEL_WIDTH),
            Self::bold(name),
            format!("(id: {})", id).dimmed(),
            status.dimmed()
        );
    }

    // --- detail block ---

    // +2 accounts for the leading space in action labels and a small indent
    const FIELD_LABEL_WIDTH: usize = Self::ACTION_LABEL_WIDTH + 2;

    pub fn stack_details(
        host: &str,
        compose_path: impl Display,
        compose_bytes: usize,
        env: Option<(String, usize)>,
        endpoint_id: impl Display,
    ) {
        let w = Self::FIELD_LABEL_WIDTH;
        println!("{:w$}{}:         {}", "", "Host".field_label(), host);
        println!(
            "{:w$}{}: {} {}",
            "",
            "Compose file".field_label(),
            compose_path,
            format!("({} bytes)", compose_bytes).dimmed()
        );
        match &env {
            Some((path, vars)) => {
                println!(
                    "{:w$}{}:     {} {}",
                    "",
                    "Env file".field_label(),
                    path,
                    format!("({} vars)", vars).dimmed()
                );
            }
            None => {
                println!(
                    "{:w$}{}:     {}",
                    "",
                    "Env file".field_label(),
                    "(none)".dimmed()
                );
            }
        }
        println!("{:w$}{}:  {}", "", "Endpoint ID".field_label(), endpoint_id);
        if env.is_some_and(|(_, vars)| vars > 0) {
            println!("{:w$}{}", "", "ENV           defined".field_label());
        }
    }

    pub fn view_details(
        stack_type: &str,
        endpoint_id: u64,
        created_by: &str,
        created: impl Display,
        updated_by: &str,
        updated: impl Display,
        env_count: usize,
    ) {
        let w = Self::FIELD_LABEL_WIDTH;
        println!("{:w$}{}:       {}", "", "Type".field_label(), stack_type);
        println!("{:w$}{}:   {}", "", "Endpoint".field_label(), endpoint_id);
        println!("{:w$}{}: {}", "", "Created by".field_label(), created_by);
        println!("{:w$}{}:    {}", "", "Created".field_label(), created);
        println!("{:w$}{}: {}", "", "Updated by".field_label(), updated_by);
        println!("{:w$}{}:    {}", "", "Updated".field_label(), updated);
        if env_count > 0 {
            println!("{:w$}{}:   {}", "", "Env vars".field_label(), env_count);
        }
    }

    pub fn ssh_stack_details(
        host: &str,
        compose_path: impl Display,
        compose_bytes: usize,
        env: Option<(String, usize)>,
        host_dir: &str,
    ) {
        let w = Self::FIELD_LABEL_WIDTH;
        println!("{:w$}{}:         {}", "", "Host".field_label(), host);
        println!(
            "{:w$}{}: {} {}",
            "",
            "Compose file".field_label(),
            compose_path,
            format!("({} bytes)", compose_bytes).dimmed()
        );
        match &env {
            Some((path, vars)) => {
                println!(
                    "{:w$}{}:     {} {}",
                    "",
                    "Env file".field_label(),
                    path,
                    format!("({} vars)", vars).dimmed()
                );
            }
            None => {
                println!(
                    "{:w$}{}:     {}",
                    "",
                    "Env file".field_label(),
                    "(none)".dimmed()
                );
            }
        }
        println!("{:w$}{}:     {}", "", "Host Dir".field_label(), host_dir);
        if env.is_some_and(|(_, vars)| vars > 0) {
            println!("{:w$}{}", "", "ENV           defined".field_label());
        }
    }

    pub fn diff_details(compose_diff: &[String], env_changes: &[EnvChange]) {
        for line in &pair_diff_lines(compose_diff) {
            let styled = match line {
                DiffLine::Insert(text) => {
                    Self::styled_diff_line('+', text, None, Style::new().green())
                }
                DiffLine::Delete(text) => {
                    Self::styled_diff_line('-', text, None, Style::new().red())
                }
                DiffLine::PairedInsert(text, mid) => {
                    Self::styled_diff_line('+', text, Some(mid.clone()), Style::new().green())
                }
                DiffLine::PairedDelete(text, mid) => {
                    Self::styled_diff_line('-', text, Some(mid.clone()), Style::new().red())
                }
                DiffLine::Header(text) => text.style_if_supported(Style::new().cyan()),
                DiffLine::Context(text) => text.clone(),
            };
            println!("    {}", styled);
        }

        if !env_changes.is_empty() {
            println!("    {}", "env changes (values hidden):".field_label());
            for change in env_changes {
                let styled = match change {
                    EnvChange::Added(n) => {
                        format!("+ {}", n).style_if_supported(Style::new().green())
                    }
                    EnvChange::Removed(n) => {
                        format!("- {}", n).style_if_supported(Style::new().red())
                    }
                    EnvChange::Changed(n) => {
                        format!("~ {}", n).style_if_supported(Style::new().yellow())
                    }
                };
                println!("    {}", styled);
            }
        }
    }

    /// Render one diff line, optionally emphasizing the changed segment
    /// (byte range into `content`) with reverse video.
    fn styled_diff_line(
        prefix: char,
        content: &str,
        mid: Option<std::ops::Range<usize>>,
        style: Style,
    ) -> String {
        match mid {
            Some(mid) if !mid.is_empty() => format!(
                "{}{}{}",
                format!("{}{}", prefix, &content[..mid.start]).style_if_supported(style),
                (&content[mid.clone()]).style_if_supported(style.reversed()),
                (&content[mid.end..]).style_if_supported(style),
            ),
            _ => format!("{}{}", prefix, content).style_if_supported(style),
        }
    }

    pub fn ssh_view_details(host: &str, host_dir: &str, ps_output: Option<&str>) {
        let w = Self::FIELD_LABEL_WIDTH;
        println!("{:w$}{}:       SSH", "", "Mode".field_label());
        println!("{:w$}{}:         {}", "", "Host".field_label(), host);
        println!("{:w$}{}:     {}", "", "Host Dir".field_label(), host_dir);
        if let Some(ps) = ps_output {
            println!("{:w$}{}:", "", "Containers".field_label());
            for line in ps.lines() {
                println!("{:w$}  {}", "", line);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{DiffLine, EnvChange, Reporter, changed_segments, pair_diff_lines};

    #[test]
    fn test_changed_segments_uncomment() {
        let (old, new) = changed_segments("  # firefox:", "  firefox:");
        assert_eq!(&"  # firefox:"[old], "# ");
        assert_eq!(&"  firefox:"[new], "");
    }

    #[test]
    fn test_changed_segments_value_change() {
        let (old, new) = changed_segments("  image: nginx:1.27", "  image: nginx:1.28");
        assert_eq!(&"  image: nginx:1.27"[old], "7");
        assert_eq!(&"  image: nginx:1.28"[new], "8");
    }

    #[test]
    fn test_changed_segments_disjoint() {
        let (old, new) = changed_segments("abc", "xyz");
        assert_eq!(old, 0..3);
        assert_eq!(new, 0..3);
    }

    #[test]
    fn test_changed_segments_multibyte() {
        let (old, new) = changed_segments("héllo", "héllö");
        assert_eq!(&"héllo"[old], "o");
        assert_eq!(&"héllö"[new], "ö");
    }

    #[test]
    fn test_pair_diff_lines_equal_runs() {
        let lines = vec![
            "@@ -1,2 +1,2 @@".to_string(),
            " context".to_string(),
            "-old line".to_string(),
            "+new line".to_string(),
        ];
        let paired = pair_diff_lines(&lines);
        assert_eq!(paired[0], DiffLine::Header("@@ -1,2 +1,2 @@".to_string()));
        assert_eq!(paired[1], DiffLine::Context(" context".to_string()));
        assert_eq!(
            paired[2],
            DiffLine::PairedDelete("old line".to_string(), 0..3)
        );
        assert_eq!(
            paired[3],
            DiffLine::PairedInsert("new line".to_string(), 0..3)
        );
    }

    #[test]
    fn test_pair_diff_lines_unequal_runs_not_paired() {
        let lines = vec![
            "-removed one".to_string(),
            "-removed two".to_string(),
            "+added".to_string(),
        ];
        let paired = pair_diff_lines(&lines);
        assert_eq!(paired[0], DiffLine::Delete("removed one".to_string()));
        assert_eq!(paired[1], DiffLine::Delete("removed two".to_string()));
        assert_eq!(paired[2], DiffLine::Insert("added".to_string()));
    }

    #[test]
    fn style_gallery() {
        Reporter::would_update("my-stack", 42);
        Reporter::would_create("my-stack");
        Reporter::updating("my-stack");
        Reporter::updated("my-stack", 42);
        Reporter::creating("my-stack");
        Reporter::created("my-stack", 42);
        Reporter::up_to_date("my-stack");
        Reporter::would_redeploy("my-stack");
        Reporter::redeploying("my-stack");
        Reporter::redeployed("my-stack", 42);
        Reporter::would_stop("my-stack", 42);
        Reporter::stopping("my-stack");
        Reporter::stopped("my-stack", 42);
        Reporter::already_stopped("my-stack");
        Reporter::starting("my-stack");
        Reporter::started("my-stack", 42);
        Reporter::disabled("my-stack");
        Reporter::not_found("my-stack");
        Reporter::changed("my-stack");
        Reporter::diff_details(
            &[
                "@@ -1,8 +1,8 @@".to_string(),
                " services:".to_string(),
                // paired change: only the differing segment is highlighted
                "-  image: nginx:1.27".to_string(),
                "+  image: nginx:1.28".to_string(),
                // paired uncomment: highlights the removed comment marker
                "-  # firefox:".to_string(),
                "-  #   image: firefox:latest".to_string(),
                "+  firefox:".to_string(),
                "+    image: firefox:latest".to_string(),
                // unequal runs: no pairing, plain red/green
                "-  removed: one".to_string(),
                "-  removed: two".to_string(),
                "+  added: line".to_string(),
            ],
            &[
                EnvChange::Added("NEW_VAR".to_string()),
                EnvChange::Removed("OLD_VAR".to_string()),
                EnvChange::Changed("API_KEY".to_string()),
            ],
        );
        Reporter::stack_details(
            "https://portainer.example.com",
            "docker-compose.yml",
            1234,
            Some((".env".to_string(), 5)),
            1,
        );
    }
}
