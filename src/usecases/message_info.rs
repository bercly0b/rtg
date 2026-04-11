use crate::domain::message_info_state::MessageInfo;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MessageInfoQuery {
    pub chat_id: i64,
    pub message_id: i64,
    pub is_outgoing: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MessageInfoError {
    Unavailable,
}

pub trait MessageInfoSource: Send + Sync {
    fn resolve_message_info(
        &self,
        query: &MessageInfoQuery,
    ) -> Result<MessageInfo, MessageInfoError>;
}

impl<T: MessageInfoSource> MessageInfoSource for std::sync::Arc<T> {
    fn resolve_message_info(
        &self,
        query: &MessageInfoQuery,
    ) -> Result<MessageInfo, MessageInfoError> {
        (**self).resolve_message_info(query)
    }
}
