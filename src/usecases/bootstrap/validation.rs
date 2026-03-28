use crate::{
    infra::{config::TelegramConfig, error::AppError, secrets::sanitize_error_code},
    usecases::guided_auth::AuthBackendError,
};

pub(super) fn map_telegram_bootstrap_error(error: AuthBackendError) -> AppError {
    let backend_code = match error {
        AuthBackendError::InvalidPhone => "AUTH_INVALID_PHONE".to_owned(),
        AuthBackendError::InvalidCode => "AUTH_INVALID_CODE".to_owned(),
        AuthBackendError::WrongPassword => "AUTH_WRONG_2FA".to_owned(),
        AuthBackendError::Timeout => "AUTH_TIMEOUT".to_owned(),
        AuthBackendError::FloodWait { .. } => "AUTH_FLOOD_WAIT".to_owned(),
        AuthBackendError::Transient { code, .. } => sanitize_error_code(code),
    };

    AppError::ConfigValidation {
        code: "TELEGRAM_BOOTSTRAP_FAILED",
        details: format!(
            "telegram client initialization failed [{backend_code}]; check telegram.api_id, telegram.api_hash, and network access"
        ),
    }
}

pub(super) fn validate_telegram_config(config: &TelegramConfig) -> Result<(), AppError> {
    let api_hash_is_default = config.api_hash == TelegramConfig::default().api_hash;
    let api_hash_missing = config.api_hash.trim().is_empty() || api_hash_is_default;
    let api_id_missing = config.api_id <= 0;

    let partially_configured = (config.api_id > 0 && api_hash_missing)
        || (config.api_hash.trim() != "" && !api_hash_is_default && api_id_missing);

    if partially_configured {
        return Err(AppError::ConfigValidation {
            code: "TELEGRAM_CONFIG_INVALID",
            details:
                "telegram.api_id and telegram.api_hash must both be set for real backend bootstrap"
                    .to_owned(),
        });
    }

    Ok(())
}
