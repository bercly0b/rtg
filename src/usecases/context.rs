use crate::{infra::config::AppConfig, telegram::TelegramAdapter};

#[derive(Debug, Clone)]
pub struct AppContext {
    pub config: AppConfig,
    pub telegram: TelegramAdapter,
}

impl AppContext {
    pub fn new(config: AppConfig) -> Self {
        Self {
            config,
            telegram: TelegramAdapter::stub(),
        }
    }
}
