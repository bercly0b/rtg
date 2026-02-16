use std::path::Path;

use crate::{
    infra::{self, config::FileConfigAdapter, contracts::ConfigAdapter, error::AppError},
    usecases::context::AppContext,
};

pub fn bootstrap(config_path: Option<&Path>) -> Result<AppContext, AppError> {
    let context = build_context(config_path)?;
    infra::logging::init(&context.config.logging)?;

    Ok(context)
}

fn build_context(config_path: Option<&Path>) -> Result<AppContext, AppError> {
    let _config_stub_contract = infra::stubs::StubConfigAdapter;
    let config_adapter = FileConfigAdapter::new(config_path);
    let config = config_adapter.load().map_err(AppError::Other)?;

    Ok(AppContext::new(config))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_context_with_default_config_when_file_is_missing() {
        let context = build_context(Some(Path::new("./missing-config.toml")))
            .expect("context should build from defaults");

        assert_eq!(context.config, crate::infra::config::AppConfig::default());
    }
}
