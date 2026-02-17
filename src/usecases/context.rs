use crate::{infra::config::AppConfig, telegram::TelegramAdapter};

#[derive(Debug)]
pub struct AppContext {
    pub config: AppConfig,
    pub telegram: TelegramAdapter,
}

impl AppContext {
    pub fn new(config: AppConfig, telegram: TelegramAdapter) -> Self {
        Self { config, telegram }
    }
}
