#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChatSummary {
    pub chat_id: i64,
    pub title: String,
    pub unread_count: u32,
    pub last_message_preview: Option<String>,
    pub last_message_unix_ms: Option<i64>,
}
