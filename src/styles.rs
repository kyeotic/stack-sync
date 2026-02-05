use owo_colors::{OwoColorize, Style};
use std::fmt::Display;

pub trait AppStyles: OwoColorize + Sized + ToString {
    fn updated(&self) -> owo_colors::Styled<&Self> {
        self.style(Style::new().green().bold())
    }

    fn up_to_date(&self) -> owo_colors::Styled<&Self> {
        self.style(Style::new().cyan().bold())
    }

    fn would_update(&self) -> owo_colors::Styled<&Self> {
        self.style(Style::new().yellow().bold())
    }

    fn dry_run(&self) -> String {
        self.style_preserving_indent(Style::new().blue().on_white().bold())
    }

    /// Applies a style only to content after leading whitespace
    fn style_preserving_indent(&self, style: Style) -> String {
        let s = self.to_string();
        let trimmed = s.trim_start();
        let leading_ws = &s[..s.len() - trimmed.len()];
        format!("{}{}", leading_ws, trimmed.style(style))
    }
}

impl<T> AppStyles for T where T: Display {}
