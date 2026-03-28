use super::file_info::format_duration;

/// Reason why a call was ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CallDiscardReason {
    /// Normal hang-up or unknown reason.
    HungUp,
    /// The call was missed (incoming) or cancelled (outgoing).
    Missed,
    /// The other party declined the call.
    Declined,
    /// The users were disconnected during the call.
    Disconnected,
}

/// Metadata for a call message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CallInfo {
    /// Whether this was a video call.
    pub is_video: bool,
    /// Call duration in seconds (0 if the call didn't connect).
    pub duration: i32,
    /// Why the call ended.
    pub discard_reason: CallDiscardReason,
}

/// Builds a display string for call metadata.
///
/// Uses `is_outgoing` from the message to determine direction.
///
/// Format follows the same `key=value` convention as file metadata:
/// `"direction=outgoing, duration=1:23"`, `"status=missed"`.
pub fn build_call_metadata_display(info: &CallInfo, is_outgoing: bool) -> String {
    let mut parts = Vec::new();

    let status = match info.discard_reason {
        CallDiscardReason::Missed => Some(if is_outgoing { "cancelled" } else { "missed" }),
        CallDiscardReason::Declined => Some("declined"),
        CallDiscardReason::Disconnected => Some("disconnected"),
        CallDiscardReason::HungUp if info.duration == 0 => {
            Some(if is_outgoing { "cancelled" } else { "missed" })
        }
        CallDiscardReason::HungUp => None,
    };

    if let Some(s) = status {
        parts.push(format!("status={s}"));
    }

    if info.duration > 0 {
        let dir = if is_outgoing { "outgoing" } else { "incoming" };
        parts.push(format!("direction={dir}"));
        parts.push(format!("duration={}", format_duration(info.duration)));
    }

    parts.join(", ")
}
