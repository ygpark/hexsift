//! Global color context for managing color output settings

use crate::cli::ColorChoice;
use std::sync::OnceLock;

static COLOR_CONTEXT: OnceLock<ColorChoice> = OnceLock::new();

/// Set the global color choice
pub fn set_color_choice(color: ColorChoice) {
    COLOR_CONTEXT.set(color).ok();
}

/// Get the current color choice (defaults to Auto if not set)
pub fn get_color_choice() -> &'static ColorChoice {
    COLOR_CONTEXT.get().unwrap_or(&ColorChoice::Auto)
}
