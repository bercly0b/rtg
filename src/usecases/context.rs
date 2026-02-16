use crate::{infra::config::AppConfig, telegram::TelegramAdapter};

#[derive(Debug, Clone)]
pub struct AppContext {
    pub config: AppConfig,
    pub telegram: TelegramAdapter,
    pub cache: CacheAdapter,
}

impl AppContext {
    pub fn new(config: AppConfig) -> Self {
        Self {
            config,
            telegram: TelegramAdapter::stub(),
            cache: CacheAdapter::stub(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct CacheAdapter;

impl CacheAdapter {
    pub fn stub() -> Self {
        Self
    }
}
