use std::path::Path;

use crate::{
    infra::{self, config::FileConfigAdapter, contracts::ConfigAdapter, error::AppError},
    usecases::context::AppContext,
};

pub fn bootstrap(config_path: Option<&Path>) -> Result<AppContext, AppError> {
    let _config_stub_contract = infra::stubs::StubConfigAdapter;
    let config_adapter = FileConfigAdapter::new(config_path);
    let config = config_adapter.load().map_err(AppError::Other)?;
    infra::logging::init(&config.logging)?;

    Ok(AppContext::new(config))
}
