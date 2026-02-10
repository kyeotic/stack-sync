use owo_colors::{OwoColorize, Style};
use std::fmt::Display;

use crate::styles::{AnsiPadding, AppStyles};

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
    use super::Reporter;

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
        Reporter::stack_details(
            "https://portainer.example.com",
            "docker-compose.yml",
            1234,
            Some((".env".to_string(), 5)),
            1,
        );
    }
}
