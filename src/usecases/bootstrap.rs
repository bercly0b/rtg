use std::path::Path;

use crate::{
    infra::{self, error::AppError},
    usecases::context::AppContext,
};

pub fn bootstrap(config_path: Option<&Path>) -> Result<AppContext, AppError> {
    let config = infra::config::load(config_path)?;
    infra::logging::init(&config.logging)?;

    Ok(AppContext::new(config))
}
