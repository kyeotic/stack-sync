use console::{Alignment, pad_str};
use owo_colors::{OwoColorize, Style};
use std::fmt::Display;

impl<T: Display> AnsiPadding for T {}

/// Application-specific styles with automatic terminal color support detection
pub trait AppStyles: OwoColorize + Sized + Display {
    fn updated(&self) -> String {
        self.style_if_supported(Style::new().green().bold())
    }

    fn up_to_date(&self) -> String {
        self.style_if_supported(Style::new().cyan().bold())
    }

    fn waiting(&self) -> String {
        self.style_if_supported(Style::new().blue().bold())
    }

    fn would_update(&self) -> String {
        self.style_if_supported(Style::new().yellow().bold())
    }

    fn field_label(&self) -> String {
        self.style_if_supported(Style::new().bold())
    }

    /// Applies a style unless NO_COLOR env var is set
    fn style_if_supported(&self, style: Style) -> String {
        // The built in self.if_supports_color breaks in bacon
        if std::env::var("NO_COLOR").is_ok() {
            self.to_string()
        } else {
            self.style(style).to_string()
        }
    }
}

impl<T> AppStyles for T where T: Display {}

/// ANSI-aware padding that ignores escape sequences when calculating width
pub trait AnsiPadding: Display {
    fn align_right(self, width: usize) -> String
    where
        Self: Sized,
    {
        pad_str(&self.to_string(), width, Alignment::Right, None).into_owned()
    }

    // fn align_left(self, width: usize) -> String
    // where
    //     Self: Sized,
    // {
    //     pad_str(&self.to_string(), width, Alignment::Left, None).into_owned()
    // }
}
