use owo_colors::{OwoColorize, Style};
use std::fmt::Display;

use crate::styles::{AnsiPadding, AppStyles};

pub struct Reporter;

impl Reporter {
    fn bold(text: &str) -> String {
        text.style_if_supported(Style::new().bold())
    }

    // --- action labels ---

    pub fn would_update(name: &str, id: impl Display) {
        println!(
            " {} {} {}",
            "Would Update".would_update().align_right(12),
            Self::bold(name),
            format!("(id: {})", id).dimmed()
        );
    }

    pub fn would_create(name: &str) {
        println!(
            " {} {}",
            "Would Create".would_update().align_right(12),
            Self::bold(name)
        );
    }

    pub fn updating(name: &str) {
        println!(
            " {} {}...",
            "Updating".waiting().align_right(12),
            Self::bold(name)
        );
    }

    pub fn updated(name: &str, id: impl Display) {
        println!(
            " {} {} {}",
            "Updated".updated().align_right(12),
            Self::bold(name),
            format!("(id: {})", id).dimmed()
        );
    }

    pub fn creating(name: &str) {
        println!(
            " {} {}...",
            "Creating".waiting().align_right(12),
            Self::bold(name)
        );
    }

    pub fn created(name: &str, id: impl Display) {
        println!(
            " {} {} {}",
            "Created".updated().align_right(12),
            Self::bold(name),
            format!("(id: {})", id).dimmed()
        );
    }

    pub fn up_to_date(name: &str) {
        println!(
            " {} {}",
            "Up-to-Date".up_to_date().align_right(12),
            Self::bold(name)
        );
    }

    pub fn would_redeploy(name: &str) {
        println!(
            " {} {}",
            "Would Redep.".would_update().align_right(12),
            Self::bold(name)
        );
    }

    pub fn redeploying(name: &str) {
        println!(
            " {} {}...",
            "Redeploying".waiting().align_right(12),
            Self::bold(name)
        );
    }

    pub fn redeployed(name: &str, id: impl Display) {
        println!(
            " {} {} {}",
            "Redeployed".updated().align_right(12),
            Self::bold(name),
            format!("(id: {})", id).dimmed()
        );
    }

    pub fn not_found(name: &str) {
        println!(
            " {} {}",
            "Not Found".would_update().align_right(12),
            Self::bold(name)
        );
    }

    // --- detail block ---

    const FIELD_LABEL_WIDTH: usize = 14;

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
