use crate::usecases::{
    list_chats::ListChatsError, load_messages::LoadMessagesError, send_message::SendMessageError,
};

pub(super) fn map_list_chats_error(error: &ListChatsError) -> &'static str {
    match error {
        ListChatsError::Unauthorized => "CHAT_LIST_UNAUTHORIZED",
        ListChatsError::TemporarilyUnavailable => "CHAT_LIST_UNAVAILABLE",
        ListChatsError::DataContractViolation => "CHAT_LIST_DATA_ERROR",
    }
}

pub(super) fn map_load_messages_error(error: &LoadMessagesError) -> &'static str {
    match error {
        LoadMessagesError::Unauthorized => "MESSAGES_UNAUTHORIZED",
        LoadMessagesError::TemporarilyUnavailable => "MESSAGES_UNAVAILABLE",
        LoadMessagesError::ChatNotFound => "MESSAGES_CHAT_NOT_FOUND",
    }
}

pub(super) fn map_send_message_error(error: &SendMessageError) -> &'static str {
    match error {
        SendMessageError::EmptyMessage => "SEND_EMPTY_MESSAGE",
        SendMessageError::Unauthorized => "SEND_UNAUTHORIZED",
        SendMessageError::ChatNotFound => "SEND_CHAT_NOT_FOUND",
        SendMessageError::TemporarilyUnavailable => "SEND_UNAVAILABLE",
    }
}
