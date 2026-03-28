//! Style definitions for the UI components.
//!
//! Styles are organized by domain area into sub-modules.
//! This module re-exports all public style functions so that consumers
//! can continue using `styles::function_name()` without changes.

mod chat_info_popup;
mod chat_list;
mod command_popup;
mod help_popup;
mod input;
mod messages;
mod panel;

pub use chat_info_popup::*;
pub use chat_list::*;
pub use command_popup::*;
pub use help_popup::*;
pub use input::*;
pub use messages::*;
pub use panel::*;

use ratatui::style::Color;

/// Palette of distinguishable colors for sender names on dark backgrounds.
///
/// Intentionally excludes Cyan (used for media indicators like `[Photo]`)
/// and Green (used for outgoing "You:" sender).
const SENDER_COLOR_PALETTE: &[Color] = &[
    Color::LightBlue,
    Color::Magenta,
    Color::Yellow,
    Color::LightRed,
    Color::LightMagenta,
    Color::Blue,
    Color::LightYellow,
    Color::Red,
];

/// Deterministic hash of a name to a palette index.
///
/// Uses FNV-1a-inspired hashing for stable, well-distributed results.
fn name_to_color_index(name: &str) -> usize {
    let mut hash: u32 = 2_166_136_261;
    for byte in name.as_bytes() {
        hash ^= u32::from(*byte);
        hash = hash.wrapping_mul(16_777_619);
    }
    (hash as usize) % SENDER_COLOR_PALETTE.len()
}

#[cfg(test)]
mod tests;
