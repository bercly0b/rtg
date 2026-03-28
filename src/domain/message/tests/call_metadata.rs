use super::*;

#[test]
fn call_metadata_outgoing_connected() {
    let info = CallInfo {
        is_video: false,
        duration: 83,
        discard_reason: CallDiscardReason::HungUp,
    };
    assert_eq!(
        build_call_metadata_display(&info, true),
        "direction=outgoing, duration=1:23"
    );
}

#[test]
fn call_metadata_incoming_connected() {
    let info = CallInfo {
        is_video: false,
        duration: 5,
        discard_reason: CallDiscardReason::HungUp,
    };
    assert_eq!(
        build_call_metadata_display(&info, false),
        "direction=incoming, duration=0:05"
    );
}

#[test]
fn call_metadata_missed_incoming() {
    let info = CallInfo {
        is_video: false,
        duration: 0,
        discard_reason: CallDiscardReason::Missed,
    };
    assert_eq!(build_call_metadata_display(&info, false), "status=missed");
}

#[test]
fn call_metadata_missed_outgoing_shows_cancelled() {
    let info = CallInfo {
        is_video: false,
        duration: 0,
        discard_reason: CallDiscardReason::Missed,
    };
    assert_eq!(build_call_metadata_display(&info, true), "status=cancelled");
}

#[test]
fn call_metadata_declined() {
    let info = CallInfo {
        is_video: false,
        duration: 0,
        discard_reason: CallDiscardReason::Declined,
    };
    assert_eq!(build_call_metadata_display(&info, false), "status=declined");
}

#[test]
fn call_metadata_video_outgoing_connected() {
    let info = CallInfo {
        is_video: true,
        duration: 60,
        discard_reason: CallDiscardReason::HungUp,
    };
    assert_eq!(
        build_call_metadata_display(&info, true),
        "direction=outgoing, duration=1:00"
    );
}

#[test]
fn call_metadata_video_missed() {
    let info = CallInfo {
        is_video: true,
        duration: 0,
        discard_reason: CallDiscardReason::Missed,
    };
    assert_eq!(build_call_metadata_display(&info, false), "status=missed");
}

#[test]
fn call_metadata_disconnected_with_duration() {
    let info = CallInfo {
        is_video: false,
        duration: 30,
        discard_reason: CallDiscardReason::Disconnected,
    };
    assert_eq!(
        build_call_metadata_display(&info, true),
        "status=disconnected, direction=outgoing, duration=0:30"
    );
}

#[test]
fn call_metadata_hungup_zero_duration_incoming_shows_missed() {
    let info = CallInfo {
        is_video: false,
        duration: 0,
        discard_reason: CallDiscardReason::HungUp,
    };
    assert_eq!(build_call_metadata_display(&info, false), "status=missed");
}

#[test]
fn call_metadata_hungup_zero_duration_outgoing_shows_cancelled() {
    let info = CallInfo {
        is_video: false,
        duration: 0,
        discard_reason: CallDiscardReason::HungUp,
    };
    assert_eq!(build_call_metadata_display(&info, true), "status=cancelled");
}
