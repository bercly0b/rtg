use std::sync::mpsc;

use crate::domain::events::{ChatUpdate, ConnectivityStatus};

use super::super::{
    ChannelChatUpdatesSignalSource, ChannelConnectivityStatusSource, ChatUpdatesSignalSource,
    ConnectivityStatusSource,
};

#[test]
fn connectivity_source_returns_latest_status_in_burst() {
    let mut source = ChannelConnectivityStatusSource::from_values(vec![
        ConnectivityStatus::Connecting,
        ConnectivityStatus::Disconnected,
        ConnectivityStatus::Connected,
    ]);

    assert_eq!(source.next_status(), Some(ConnectivityStatus::Connected));
    assert_eq!(source.next_status(), None);
}

#[test]
fn connectivity_source_is_non_blocking_when_channel_is_empty() {
    let (_tx, rx) = mpsc::channel::<ConnectivityStatus>();
    let mut source = ChannelConnectivityStatusSource::new(rx);

    assert_eq!(source.next_status(), None);
}

#[test]
fn chat_updates_source_drains_burst_into_single_batch() {
    let mut source = ChannelChatUpdatesSignalSource::from_updates(vec![
        ChatUpdate::ChatMetadataChanged { chat_id: 1 },
        ChatUpdate::ChatMetadataChanged { chat_id: 2 },
        ChatUpdate::ChatMetadataChanged { chat_id: 3 },
    ]);

    let updates = source.pending_updates();
    assert_eq!(updates.as_ref().map(|u| u.len()), Some(3));
    assert_eq!(source.pending_updates(), None);
}
