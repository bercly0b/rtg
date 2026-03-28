use crate::telegram::tdlib_client::TdLibError;
use crate::usecases::guided_auth::AuthBackendError;
use crate::usecases::list_chats::ListChatsSourceError;

use super::error_mapping::{
    map_list_chats_error, map_password_error, map_request_code_error, map_sign_in_error,
    map_tdlib_error, parse_flood_wait_seconds,
};
use super::messages::reply_sender_name_for_message;

#[test]
fn parse_flood_wait_extracts_seconds() {
    assert_eq!(parse_flood_wait_seconds("flood_wait_67"), Some(67));
    assert_eq!(parse_flood_wait_seconds("FLOOD_WAIT_120"), Some(120));
    assert_eq!(parse_flood_wait_seconds("no flood here"), None);
    assert_eq!(parse_flood_wait_seconds("other error"), None);
}

#[test]
fn map_request_code_error_detects_invalid_phone() {
    let error = TdLibError::Request {
        code: 400,
        message: "PHONE_NUMBER_INVALID".to_owned(),
    };
    assert_eq!(
        map_request_code_error(error),
        AuthBackendError::InvalidPhone
    );
}

#[test]
fn map_sign_in_error_detects_invalid_code() {
    let error = TdLibError::Request {
        code: 400,
        message: "PHONE_CODE_INVALID".to_owned(),
    };
    assert_eq!(map_sign_in_error(error), AuthBackendError::InvalidCode);
}

#[test]
fn map_password_error_detects_wrong_password() {
    let error = TdLibError::Request {
        code: 400,
        message: "PASSWORD_HASH_INVALID".to_owned(),
    };
    assert_eq!(map_password_error(error), AuthBackendError::WrongPassword);
}

#[test]
fn map_flood_wait_in_request_code() {
    let error = TdLibError::Request {
        code: 429,
        message: "FLOOD_WAIT_300".to_owned(),
    };
    assert_eq!(
        map_request_code_error(error),
        AuthBackendError::FloodWait { seconds: 300 }
    );
}

#[test]
fn map_list_chats_error_returns_unavailable_for_generic_error() {
    let error = TdLibError::Request {
        code: 500,
        message: "Internal Server Error".to_owned(),
    };
    assert_eq!(
        map_list_chats_error(error),
        ListChatsSourceError::Unavailable,
    );
}

#[test]
fn map_list_chats_error_returns_unauthorized_for_auth_error() {
    let error = TdLibError::Request {
        code: 401,
        message: "Unauthorized".to_owned(),
    };
    assert_eq!(
        map_list_chats_error(error),
        ListChatsSourceError::Unauthorized,
    );
}

#[test]
fn map_tdlib_error_maps_request_to_transient() {
    let error = TdLibError::Request {
        code: 400,
        message: "BAD_REQUEST".to_owned(),
    };
    assert_eq!(
        map_tdlib_error(error),
        AuthBackendError::Transient {
            code: "AUTH_BACKEND_UNAVAILABLE",
            message: "BAD_REQUEST".to_owned(),
        }
    );
}

#[test]
fn reply_sender_name_for_outgoing_message_is_you() {
    let message = crate::domain::message::Message {
        id: 1,
        sender_name: "My Real Name".to_owned(),
        text: "hello".to_owned(),
        timestamp_ms: 0,
        is_outgoing: true,
        media: crate::domain::message::MessageMedia::None,
        status: crate::domain::message::MessageStatus::Delivered,
        file_info: None,
        call_info: None,
        reply_to: None,
        reaction_count: 0,
        links: Vec::new(),
        is_edited: false,
    };

    assert_eq!(reply_sender_name_for_message(&message), "You");
}
