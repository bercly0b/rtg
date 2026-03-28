use crate::usecases::guided_auth::AuthBackendError;
use crate::usecases::list_chats::ListChatsSourceError;
use crate::usecases::load_messages::MessagesSourceError;
use crate::usecases::send_message::SendMessageSourceError;

use super::super::tdlib_client::TdLibError;

/// Maps TDLib initialization error to AuthBackendError.
pub(super) fn map_init_error(error: TdLibError) -> AuthBackendError {
    AuthBackendError::Transient {
        code: "AUTH_BACKEND_UNAVAILABLE",
        message: format!("TDLib initialization failed: {error}"),
    }
}

/// Maps TDLib error to AuthBackendError.
pub(super) fn map_tdlib_error(error: TdLibError) -> AuthBackendError {
    match error {
        TdLibError::Timeout { .. } => AuthBackendError::Timeout,
        TdLibError::Init { message } | TdLibError::Request { message, .. } => {
            AuthBackendError::Transient {
                code: "AUTH_BACKEND_UNAVAILABLE",
                message,
            }
        }
        TdLibError::Shutdown { message } => AuthBackendError::Transient {
            code: "AUTH_BACKEND_CLOSED",
            message,
        },
    }
}

/// Maps phone number request error.
pub(super) fn map_request_code_error(error: TdLibError) -> AuthBackendError {
    let TdLibError::Request { message, .. } = error else {
        return map_tdlib_error(error);
    };

    let msg_lower = message.to_ascii_lowercase();

    if msg_lower.contains("phone") && msg_lower.contains("invalid") {
        return AuthBackendError::InvalidPhone;
    }

    if let Some(seconds) = parse_flood_wait_seconds(&msg_lower) {
        return AuthBackendError::FloodWait { seconds };
    }

    AuthBackendError::Transient {
        code: "AUTH_REQUEST_CODE_FAILED",
        message,
    }
}

/// Maps sign-in error.
pub(super) fn map_sign_in_error(error: TdLibError) -> AuthBackendError {
    let TdLibError::Request { message, .. } = error else {
        return map_tdlib_error(error);
    };

    let msg_lower = message.to_ascii_lowercase();

    if msg_lower.contains("code")
        && (msg_lower.contains("invalid") || msg_lower.contains("expired"))
    {
        return AuthBackendError::InvalidCode;
    }

    if let Some(seconds) = parse_flood_wait_seconds(&msg_lower) {
        return AuthBackendError::FloodWait { seconds };
    }

    AuthBackendError::Transient {
        code: "AUTH_SIGN_IN_FAILED",
        message,
    }
}

/// Maps password verification error.
pub(super) fn map_password_error(error: TdLibError) -> AuthBackendError {
    let TdLibError::Request { message, .. } = error else {
        return map_tdlib_error(error);
    };

    let msg_lower = message.to_ascii_lowercase();

    if msg_lower.contains("password") && msg_lower.contains("invalid") {
        return AuthBackendError::WrongPassword;
    }

    if let Some(seconds) = parse_flood_wait_seconds(&msg_lower) {
        return AuthBackendError::FloodWait { seconds };
    }

    AuthBackendError::Transient {
        code: "AUTH_PASSWORD_VERIFY_FAILED",
        message,
    }
}

/// Extracts flood wait seconds from error message.
pub(super) fn parse_flood_wait_seconds(message: &str) -> Option<u32> {
    let msg_lower = message.to_ascii_lowercase();
    if !msg_lower.contains("flood") {
        return None;
    }

    message
        .split(|ch: char| !ch.is_ascii_digit())
        .find_map(|part| {
            (!part.is_empty())
                .then(|| part.parse::<u32>().ok())
                .flatten()
        })
}

/// Maps TDLib error to ListChatsSourceError.
pub(super) fn map_list_chats_error(error: TdLibError) -> ListChatsSourceError {
    let msg = match &error {
        TdLibError::Request { message, .. } => message.to_ascii_lowercase(),
        _ => String::new(),
    };

    if msg.contains("unauthorized") || msg.contains("auth") {
        return ListChatsSourceError::Unauthorized;
    }

    ListChatsSourceError::Unavailable
}

/// Maps TDLib error to MessagesSourceError.
pub(super) fn map_messages_error(error: TdLibError) -> MessagesSourceError {
    let msg = match &error {
        TdLibError::Request { message, .. } => message.to_ascii_lowercase(),
        _ => String::new(),
    };

    if msg.contains("unauthorized") || msg.contains("auth") {
        return MessagesSourceError::Unauthorized;
    }

    if msg.contains("chat") && msg.contains("not found") {
        return MessagesSourceError::ChatNotFound;
    }

    MessagesSourceError::Unavailable
}

/// Maps TDLib error to SendMessageSourceError.
pub(super) fn map_send_message_error(error: TdLibError) -> SendMessageSourceError {
    let msg = match &error {
        TdLibError::Request { message, .. } => message.to_ascii_lowercase(),
        _ => String::new(),
    };

    if msg.contains("unauthorized") || msg.contains("auth") {
        return SendMessageSourceError::Unauthorized;
    }

    if msg.contains("chat") && msg.contains("not found") {
        return SendMessageSourceError::ChatNotFound;
    }

    SendMessageSourceError::Unavailable
}
