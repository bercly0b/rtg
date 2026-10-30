use crate::usecases::edit_message::EditMessageSourceError;
use crate::usecases::guided_auth::AuthBackendError;
use crate::usecases::list_chats::ListChatsSourceError;
use crate::usecases::list_forum_topics::ListForumTopicsSourceError;
use crate::usecases::load_messages::MessagesSourceError;
use crate::usecases::send_message::SendMessageSourceError;

use super::super::tdlib_client::TdLibError;

/// TDLib error code for a request that needs an authorized session.
const TDLIB_ERROR_UNAUTHORIZED: i32 = 401;

/// TDLib error code for a rate-limited request: TDLib turns a long Telegram
/// `FLOOD_WAIT_N` into "Too Many Requests: retry after N".
const TDLIB_ERROR_TOO_MANY_REQUESTS: i32 = 429;

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
        TdLibError::Request { .. } => AuthBackendError::Transient {
            code: "AUTH_BACKEND_UNAVAILABLE",
            message: error.to_string(),
        },
        TdLibError::Init { message }
        | TdLibError::Internal { message }
        | TdLibError::Decode { message, .. } => AuthBackendError::Transient {
            code: "AUTH_BACKEND_UNAVAILABLE",
            message,
        },
        TdLibError::Shutdown { message } => AuthBackendError::Transient {
            code: "AUTH_BACKEND_CLOSED",
            message,
        },
    }
}

/// Maps phone number request error.
pub(super) fn map_request_code_error(error: TdLibError) -> AuthBackendError {
    let TdLibError::Request { code, message } = &error else {
        return map_tdlib_error(error);
    };

    let msg_lower = message.to_ascii_lowercase();

    if msg_lower.contains("phone") && msg_lower.contains("invalid") {
        return AuthBackendError::InvalidPhone;
    }

    if let Some(seconds) = parse_flood_wait_seconds(*code, &msg_lower) {
        return AuthBackendError::FloodWait { seconds };
    }

    AuthBackendError::Transient {
        code: "AUTH_REQUEST_CODE_FAILED",
        message: error.to_string(),
    }
}

/// Maps sign-in error.
pub(super) fn map_sign_in_error(error: TdLibError) -> AuthBackendError {
    let TdLibError::Request { code, message } = &error else {
        return map_tdlib_error(error);
    };

    let msg_lower = message.to_ascii_lowercase();

    if msg_lower.contains("code")
        && (msg_lower.contains("invalid") || msg_lower.contains("expired"))
    {
        return AuthBackendError::InvalidCode;
    }

    if let Some(seconds) = parse_flood_wait_seconds(*code, &msg_lower) {
        return AuthBackendError::FloodWait { seconds };
    }

    AuthBackendError::Transient {
        code: "AUTH_SIGN_IN_FAILED",
        message: error.to_string(),
    }
}

/// Maps password verification error.
pub(super) fn map_password_error(error: TdLibError) -> AuthBackendError {
    let TdLibError::Request { code, message } = &error else {
        return map_tdlib_error(error);
    };

    let msg_lower = message.to_ascii_lowercase();

    if msg_lower.contains("password") && msg_lower.contains("invalid") {
        return AuthBackendError::WrongPassword;
    }

    if let Some(seconds) = parse_flood_wait_seconds(*code, &msg_lower) {
        return AuthBackendError::FloodWait { seconds };
    }

    AuthBackendError::Transient {
        code: "AUTH_PASSWORD_VERIFY_FAILED",
        message: error.to_string(),
    }
}

/// Extracts the wait time of a rate-limited request.
///
/// Code 429 is how TDLib reports it; a `flood` message also counts, for an
/// MTProto error relayed as is. `msg_lower` must already be lowercased.
pub(super) fn parse_flood_wait_seconds(code: i32, msg_lower: &str) -> Option<u32> {
    if code != TDLIB_ERROR_TOO_MANY_REQUESTS && !msg_lower.contains("flood") {
        return None;
    }

    msg_lower
        .split(|ch: char| !ch.is_ascii_digit())
        .find_map(|part| {
            (!part.is_empty())
                .then(|| part.parse::<u32>().ok())
                .flatten()
        })
}

/// TDLib answers 401 to any request that needs an authorized session; the
/// message check is a fallback for an error that comes without that code.
fn is_unauthorized(code: i32, msg_lower: &str) -> bool {
    code == TDLIB_ERROR_UNAUTHORIZED || msg_lower.contains("unauthorized")
}

/// Maps TDLib error to ListChatsSourceError.
pub(super) fn map_list_chats_error(error: TdLibError) -> ListChatsSourceError {
    let TdLibError::Request { code, message } = &error else {
        return ListChatsSourceError::Unavailable;
    };
    let msg = message.to_ascii_lowercase();

    if is_unauthorized(*code, &msg) {
        return ListChatsSourceError::Unauthorized;
    }

    ListChatsSourceError::Unavailable
}

/// Maps TDLib error to MessagesSourceError.
pub(super) fn map_messages_error(error: TdLibError) -> MessagesSourceError {
    let TdLibError::Request { code, message } = &error else {
        return MessagesSourceError::Unavailable;
    };
    let msg = message.to_ascii_lowercase();

    if is_unauthorized(*code, &msg) {
        return MessagesSourceError::Unauthorized;
    }

    if msg.contains("chat") && msg.contains("not found") {
        return MessagesSourceError::ChatNotFound;
    }

    MessagesSourceError::Unavailable
}

/// Maps TDLib error to ListForumTopicsSourceError.
pub(super) fn map_forum_topics_error(error: TdLibError) -> ListForumTopicsSourceError {
    let TdLibError::Request { code, message } = &error else {
        return ListForumTopicsSourceError::Unavailable;
    };
    let msg = message.to_ascii_lowercase();

    if is_unauthorized(*code, &msg) {
        return ListForumTopicsSourceError::Unauthorized;
    }

    if msg.contains("chat") && msg.contains("not found") {
        return ListForumTopicsSourceError::ChatNotFound;
    }

    // TDLib returns "The chat is not a forum" / "The supergroup must be a
    // forum" / "CHANNEL_FORUM_MISSING" when our cached `is_forum` flag is
    // stale. Surface this as a hard data error so the UI shows an error
    // state instead of looping in Loading.
    if msg.contains("not a forum")
        || msg.contains("must be a forum")
        || msg.contains("forum_missing")
    {
        return ListForumTopicsSourceError::InvalidData;
    }

    ListForumTopicsSourceError::Unavailable
}

/// Maps TDLib error to SendMessageSourceError.
pub(super) fn map_send_message_error(error: TdLibError) -> SendMessageSourceError {
    let TdLibError::Request { code, message } = &error else {
        return SendMessageSourceError::Unavailable;
    };
    let msg = message.to_ascii_lowercase();

    if is_unauthorized(*code, &msg) {
        return SendMessageSourceError::Unauthorized;
    }

    if msg.contains("chat") && msg.contains("not found") {
        return SendMessageSourceError::ChatNotFound;
    }

    SendMessageSourceError::Unavailable
}

pub(super) fn map_edit_message_error(error: TdLibError) -> EditMessageSourceError {
    let TdLibError::Request { code, message } = &error else {
        return EditMessageSourceError::Unavailable;
    };
    let msg = message.to_ascii_lowercase();

    if is_unauthorized(*code, &msg) {
        return EditMessageSourceError::Unauthorized;
    }

    if msg.contains("message") && msg.contains("not found") {
        return EditMessageSourceError::MessageNotFound;
    }

    EditMessageSourceError::Unavailable
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(code: i32, message: &str) -> TdLibError {
        TdLibError::Request {
            code,
            message: message.to_owned(),
        }
    }

    fn assert_classified<E: PartialEq + std::fmt::Debug>(
        map: fn(TdLibError) -> E,
        cases: &[(i32, &str, E)],
    ) {
        for (code, message, expected) in cases {
            assert_eq!(
                &map(request(*code, message)),
                expected,
                "code {code}, message {message:?}"
            );
        }
    }

    #[test]
    fn list_chats_errors_are_classified_by_code_then_message() {
        use ListChatsSourceError::{Unauthorized, Unavailable};
        assert_classified(
            map_list_chats_error,
            &[
                (401, "Unauthorized", Unauthorized),
                (401, "SESSION_REVOKED", Unauthorized),
                (400, "Unauthorized", Unauthorized),
                (403, "MESSAGE_AUTHOR_REQUIRED", Unavailable),
                (400, "AUTH_BYTES_INVALID", Unavailable),
                (500, "Internal Server Error", Unavailable),
            ],
        );
    }

    #[test]
    fn messages_errors_are_classified_by_code_then_message() {
        use MessagesSourceError::{ChatNotFound, Unauthorized, Unavailable};
        assert_classified(
            map_messages_error,
            &[
                (401, "Unauthorized", Unauthorized),
                (401, "SESSION_REVOKED", Unauthorized),
                (403, "MESSAGE_AUTHOR_REQUIRED", Unavailable),
                (400, "Chat not found", ChatNotFound),
                (500, "Internal Server Error", Unavailable),
            ],
        );
    }

    #[test]
    fn forum_topics_errors_are_classified_by_code_then_message() {
        use ListForumTopicsSourceError::{ChatNotFound, InvalidData, Unauthorized, Unavailable};
        assert_classified(
            map_forum_topics_error,
            &[
                (401, "Unauthorized", Unauthorized),
                (401, "SESSION_REVOKED", Unauthorized),
                (403, "MESSAGE_AUTHOR_REQUIRED", Unavailable),
                (400, "Chat not found", ChatNotFound),
                (400, "CHANNEL_FORUM_MISSING", InvalidData),
            ],
        );
    }

    #[test]
    fn send_message_errors_are_classified_by_code_then_message() {
        use SendMessageSourceError::{ChatNotFound, Unauthorized, Unavailable};
        assert_classified(
            map_send_message_error,
            &[
                (401, "Unauthorized", Unauthorized),
                (401, "SESSION_REVOKED", Unauthorized),
                (403, "MESSAGE_AUTHOR_REQUIRED", Unavailable),
                (400, "Chat not found", ChatNotFound),
                (403, "CHAT_WRITE_FORBIDDEN", Unavailable),
            ],
        );
    }

    #[test]
    fn edit_message_errors_are_classified_by_code_then_message() {
        use EditMessageSourceError::{MessageNotFound, Unauthorized, Unavailable};
        assert_classified(
            map_edit_message_error,
            &[
                (401, "Unauthorized", Unauthorized),
                (401, "SESSION_REVOKED", Unauthorized),
                (403, "MESSAGE_AUTHOR_REQUIRED", Unavailable),
                (400, "MESSAGE_AUTHOR_REQUIRED", Unavailable),
                (400, "Message not found", MessageNotFound),
                (400, "Message can't be edited", Unavailable),
            ],
        );
    }

    #[test]
    fn request_code_errors_are_classified_by_code_then_message() {
        assert_classified(
            map_request_code_error,
            &[
                (400, "PHONE_NUMBER_INVALID", AuthBackendError::InvalidPhone),
                (
                    429,
                    "Too Many Requests: retry after 300",
                    AuthBackendError::FloodWait { seconds: 300 },
                ),
                (
                    420,
                    "FLOOD_WAIT_67",
                    AuthBackendError::FloodWait { seconds: 67 },
                ),
                (
                    400,
                    "PHONE_NUMBER_BANNED",
                    AuthBackendError::Transient {
                        code: "AUTH_REQUEST_CODE_FAILED",
                        message: "TDLib request error 400: PHONE_NUMBER_BANNED".to_owned(),
                    },
                ),
            ],
        );
    }

    #[test]
    fn sign_in_errors_are_classified_by_code_then_message() {
        assert_classified(
            map_sign_in_error,
            &[
                (400, "PHONE_CODE_INVALID", AuthBackendError::InvalidCode),
                (400, "PHONE_CODE_EXPIRED", AuthBackendError::InvalidCode),
                (
                    429,
                    "Too Many Requests: retry after 60",
                    AuthBackendError::FloodWait { seconds: 60 },
                ),
                (
                    400,
                    "SESSION_PASSWORD_NEEDED",
                    AuthBackendError::Transient {
                        code: "AUTH_SIGN_IN_FAILED",
                        message: "TDLib request error 400: SESSION_PASSWORD_NEEDED".to_owned(),
                    },
                ),
            ],
        );
    }

    #[test]
    fn password_errors_are_classified_by_code_then_message() {
        assert_classified(
            map_password_error,
            &[
                (
                    400,
                    "PASSWORD_HASH_INVALID",
                    AuthBackendError::WrongPassword,
                ),
                (
                    429,
                    "Too Many Requests: retry after 120",
                    AuthBackendError::FloodWait { seconds: 120 },
                ),
                (
                    500,
                    "Internal Server Error",
                    AuthBackendError::Transient {
                        code: "AUTH_PASSWORD_VERIFY_FAILED",
                        message: "TDLib request error 500: Internal Server Error".to_owned(),
                    },
                ),
            ],
        );
    }

    #[test]
    fn errors_that_never_reached_tdlib_are_unavailable() {
        let timeout = || TdLibError::Timeout {
            message: "waiting for response".to_owned(),
        };
        assert_eq!(
            map_list_chats_error(timeout()),
            ListChatsSourceError::Unavailable
        );
        assert_eq!(
            map_messages_error(timeout()),
            MessagesSourceError::Unavailable
        );
        assert_eq!(
            map_forum_topics_error(timeout()),
            ListForumTopicsSourceError::Unavailable
        );
        assert_eq!(
            map_send_message_error(timeout()),
            SendMessageSourceError::Unavailable
        );
        assert_eq!(
            map_edit_message_error(timeout()),
            EditMessageSourceError::Unavailable
        );
    }

    #[test]
    fn internal_error_maps_to_unavailable_auth_backend() {
        let error = TdLibError::Internal {
            message: "auth state receiver lock poisoned".to_owned(),
        };
        assert_eq!(
            map_tdlib_error(error),
            AuthBackendError::Transient {
                code: "AUTH_BACKEND_UNAVAILABLE",
                message: "auth state receiver lock poisoned".to_owned(),
            }
        );
    }

    #[test]
    fn forum_topics_error_chat_not_found_is_mapped() {
        let err = request(400, "Chat not found");
        assert_eq!(
            map_forum_topics_error(err),
            ListForumTopicsSourceError::ChatNotFound
        );
    }

    #[test]
    fn forum_topics_error_not_a_forum_is_invalid_data() {
        let err = request(400, "The chat is not a forum");
        assert_eq!(
            map_forum_topics_error(err),
            ListForumTopicsSourceError::InvalidData
        );
    }

    #[test]
    fn forum_topics_error_must_be_a_forum_is_invalid_data() {
        let err = request(400, "The supergroup must be a forum");
        assert_eq!(
            map_forum_topics_error(err),
            ListForumTopicsSourceError::InvalidData
        );
    }

    #[test]
    fn forum_topics_error_unknown_falls_back_to_unavailable() {
        let err = request(500, "Internal Server Error");
        assert_eq!(
            map_forum_topics_error(err),
            ListForumTopicsSourceError::Unavailable
        );
    }
}
