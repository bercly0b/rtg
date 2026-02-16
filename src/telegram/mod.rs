//! Telegram integration layer: API clients and event mapping.

#[derive(Debug, Clone)]
pub struct TelegramAdapter;

impl TelegramAdapter {
    pub fn stub() -> Self {
        Self
    }
}

/// Returns the telegram module name for smoke checks.
pub fn module_name() -> &'static str {
    "telegram"
}
