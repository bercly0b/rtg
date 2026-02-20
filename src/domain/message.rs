#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Message {
    pub id: i32,
    pub sender_name: String,
    pub text: String,
    pub timestamp_ms: i64,
    pub is_outgoing: bool,
}
