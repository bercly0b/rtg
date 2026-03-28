use crate::usecases::background::error_mapping::*;

#[test]
fn map_list_chats_error_unauthorized() {
    use crate::usecases::list_chats::ListChatsError;
    assert_eq!(
        map_list_chats_error(&ListChatsError::Unauthorized),
        "CHAT_LIST_UNAUTHORIZED"
    );
}

#[test]
fn map_list_chats_error_unavailable() {
    use crate::usecases::list_chats::ListChatsError;
    assert_eq!(
        map_list_chats_error(&ListChatsError::TemporarilyUnavailable),
        "CHAT_LIST_UNAVAILABLE"
    );
}

#[test]
fn map_list_chats_error_data_contract() {
    use crate::usecases::list_chats::ListChatsError;
    assert_eq!(
        map_list_chats_error(&ListChatsError::DataContractViolation),
        "CHAT_LIST_DATA_ERROR"
    );
}

#[test]
fn map_load_messages_error_unauthorized() {
    use crate::usecases::load_messages::LoadMessagesError;
    assert_eq!(
        map_load_messages_error(&LoadMessagesError::Unauthorized),
        "MESSAGES_UNAUTHORIZED"
    );
}

#[test]
fn map_load_messages_error_unavailable() {
    use crate::usecases::load_messages::LoadMessagesError;
    assert_eq!(
        map_load_messages_error(&LoadMessagesError::TemporarilyUnavailable),
        "MESSAGES_UNAVAILABLE"
    );
}

#[test]
fn map_load_messages_error_chat_not_found() {
    use crate::usecases::load_messages::LoadMessagesError;
    assert_eq!(
        map_load_messages_error(&LoadMessagesError::ChatNotFound),
        "MESSAGES_CHAT_NOT_FOUND"
    );
}

#[test]
fn map_send_message_error_empty() {
    use crate::usecases::send_message::SendMessageError;
    assert_eq!(
        map_send_message_error(&SendMessageError::EmptyMessage),
        "SEND_EMPTY_MESSAGE"
    );
}

#[test]
fn map_send_message_error_unauthorized() {
    use crate::usecases::send_message::SendMessageError;
    assert_eq!(
        map_send_message_error(&SendMessageError::Unauthorized),
        "SEND_UNAUTHORIZED"
    );
}

#[test]
fn map_send_message_error_chat_not_found() {
    use crate::usecases::send_message::SendMessageError;
    assert_eq!(
        map_send_message_error(&SendMessageError::ChatNotFound),
        "SEND_CHAT_NOT_FOUND"
    );
}

#[test]
fn map_send_message_error_unavailable() {
    use crate::usecases::send_message::SendMessageError;
    assert_eq!(
        map_send_message_error(&SendMessageError::TemporarilyUnavailable),
        "SEND_UNAVAILABLE"
    );
}
