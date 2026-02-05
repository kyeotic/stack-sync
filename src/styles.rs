use owo_colors::{OwoColorize, Style};
use std::fmt::Display;

pub trait AppStyles: OwoColorize + Sized {
    fn updated(self) -> owo_colors::Styled<Self> {
        self.style(Style::new().green().bold())
    }

    fn up_to_date(self) -> owo_colors::Styled<Self> {
        self.style(Style::new().blue().bold())
    }

    fn would_update(self) -> owo_colors::Styled<Self> {
        self.style(Style::new().yellow().bold())
    }
}

impl<T> AppStyles for T where T: Display {}
