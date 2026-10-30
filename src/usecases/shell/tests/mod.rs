mod chat_info;
mod chat_list;
mod chat_open;
mod chat_updates;
mod dispatcher;
mod factories;
mod fixtures;
mod forum;
mod help_popup;
mod lifecycle;
mod message_actions;
mod message_cache;
mod message_info;
mod message_input;
mod message_pagination;
mod playback;
mod reaction_picker;
mod voice;

use std::cell::RefCell;

use super::*;
use crate::{
    domain::{
        chat::ChatSummary,
        chat_list_state::ChatListUiState,
        events::{AppEvent, BackgroundError, BackgroundTaskResult, ConnectivityStatus, KeyInput},
        message::Message,
        message_input_state::{OutgoingMessage, ReplyContext},
        open_chat_state::OpenChatUiState,
        shell_state::ShellState,
    },
    infra::{contracts::ExternalOpener, stubs::StubStorageAdapter},
};

use dispatcher::*;
use factories::*;
use fixtures::*;
