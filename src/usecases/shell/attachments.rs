use crate::{
    domain::{
        attachment::OutgoingAttachment, chat::ChatType, events::KeyInput,
        file_picker_state::FileEntry, message::ReplyInfo, open_defaults::DEFAULT_OPEN,
    },
    usecases::{
        background::{BackgroundError, TaskDispatcher, ATTACHMENT_TOO_LARGE},
        send_attachment::attachment_kind,
    },
};

use super::ShellCtx;

/// What the file picker lists, and the runs of the file list command it has
/// started.
pub(super) struct Attachments {
    list_cmd: String,
    dirs: Vec<String>,
    last_listing_id: u64,
}

impl Attachments {
    pub(super) fn new(list_cmd: String, dirs: Vec<String>) -> Self {
        Self {
            list_cmd,
            dirs,
            last_listing_id: 0,
        }
    }
}

pub(super) fn open_file_picker<D: TaskDispatcher>(
    ctx: &mut ShellCtx<'_, D>,
    attachments: &mut Attachments,
) {
    if ctx.state.open_chat().chat_type() == ChatType::Channel {
        ctx.state.set_notification("Channel is read-only");
        return;
    }
    if ctx.state.open_topic_is_closed() {
        ctx.state.set_notification("Topic is closed");
        return;
    }

    attachments.last_listing_id += 1;
    let home = dirs::home_dir().and_then(|home| home.to_str().map(str::to_owned));
    ctx.state
        .open_file_picker(attachments.last_listing_id, home);
    ctx.dispatcher.dispatch_list_files(
        attachments.last_listing_id,
        attachments.list_cmd.clone(),
        attachments.dirs.clone(),
    );
}

pub(super) fn handle_file_picker_key<D: TaskDispatcher>(ctx: &mut ShellCtx<'_, D>, key: &KeyInput) {
    let Some(picker) = ctx.state.file_picker_mut() else {
        return;
    };
    match (key.key.as_str(), key.ctrl) {
        ("j", true) | ("down", _) => picker.select_next(),
        ("k", true) | ("up", _) => picker.select_previous(),
        ("l", true) => {
            if let Some(path) = picker.selected().map(|entry| entry.path.clone()) {
                open_selected_file(ctx, path);
            }
        }
        ("backspace", _) => picker.delete_char_before(),
        ("esc", _) => close_file_picker(ctx),
        ("enter", _) => {
            if let Some(entry) = picker.selected().cloned() {
                close_file_picker(ctx);
                send_attachment(ctx, entry);
            }
        }
        (ch, false) if ch.chars().count() == 1 => {
            if let Some(ch) = ch.chars().next() {
                picker.insert_char(ch);
            }
        }
        _ => {}
    }
}

/// Opens the file for a look before it is sent. Always with the platform
/// opener: choosing an `[open]` handler needs a MIME type, which a local
/// file does not come with.
fn open_selected_file<D: TaskDispatcher>(ctx: &mut ShellCtx<'_, D>, path: String) {
    ctx.dispatcher
        .dispatch_open_file(DEFAULT_OPEN.to_owned(), path);
}

fn close_file_picker<D: TaskDispatcher>(ctx: &mut ShellCtx<'_, D>) {
    ctx.state.close_file_picker();
    ctx.dispatcher.dispatch_stop_file_listing();
}

/// Sends the file as an attachment: the text in the input becomes its
/// caption, and the message shows up at once as being sent.
fn send_attachment<D: TaskDispatcher>(ctx: &mut ShellCtx<'_, D>, entry: FileEntry) {
    let Some(chat_id) = ctx.state.open_chat().chat_id() else {
        return;
    };

    let kind = attachment_kind(&entry.path, entry.size);
    let caption = ctx.state.message_input().text().trim().to_owned();
    let reply_to = ctx.state.message_input_mut().take_reply_to();
    let pending_reply_info = reply_to.as_ref().map(|reply| ReplyInfo {
        sender_name: reply.sender_name.clone(),
        is_outgoing: reply.sender_name == "You",
        text: reply.text.clone(),
    });

    ctx.state.message_input_mut().clear();
    let pending_id = ctx.state.open_chat_mut().add_pending_message(
        caption.clone(),
        kind.media(),
        pending_reply_info,
    );
    let topic_id = ctx.state.open_chat().topic_id();
    ctx.dispatcher.dispatch_send_attachment(
        chat_id,
        topic_id,
        OutgoingAttachment {
            pending_id,
            path: entry.path,
            size_bytes: entry.size,
            kind,
            caption,
            reply_to,
        },
    );
}

pub(super) fn handle_files_listed<D: TaskDispatcher>(
    ctx: &mut ShellCtx<'_, D>,
    listing_id: u64,
    entries: Vec<FileEntry>,
) {
    if let Some(picker) = current_picker(ctx, listing_id) {
        picker.add_entries(entries);
    }
}

pub(super) fn handle_file_listing_finished<D: TaskDispatcher>(
    ctx: &mut ShellCtx<'_, D>,
    listing_id: u64,
    error: Option<String>,
) {
    if let Some(picker) = current_picker(ctx, listing_id) {
        picker.finish(error);
    }
}

/// The open file picker, unless `listing_id` belongs to an earlier one.
fn current_picker<'a, D: TaskDispatcher>(
    ctx: &'a mut ShellCtx<'_, D>,
    listing_id: u64,
) -> Option<&'a mut crate::domain::file_picker_state::FilePickerState> {
    ctx.state
        .file_picker_mut()
        .filter(|picker| picker.listing_id() == listing_id)
}

pub(super) fn handle_attachment_sent<D: TaskDispatcher>(
    ctx: &mut ShellCtx<'_, D>,
    chat_id: i64,
    topic_id: Option<i32>,
    attachment: OutgoingAttachment,
    result: Result<(), BackgroundError>,
) {
    let Err(error) = result else {
        tracing::debug!(chat_id, "background: attachment sent successfully");
        return;
    };
    tracing::warn!(
        chat_id,
        code = error.code,
        "background: send attachment failed"
    );

    ctx.state
        .set_notification(if error.code == ATTACHMENT_TOO_LARGE {
            "File is too large to send"
        } else {
            "Failed to send the file"
        });

    // Pending ids and the reply target mean something only in the
    // conversation the attachment was sent from.
    let still_open = ctx.state.open_chat().chat_id() == Some(chat_id)
        && ctx.state.open_chat().topic_id() == topic_id;
    if still_open {
        ctx.state
            .open_chat_mut()
            .remove_message(attachment.pending_id);
    }
    let reply_to = attachment.reply_to.filter(|_| still_open);
    // Without a caption or a reply there is nothing to put back, and a
    // draft typed meanwhile must survive.
    if !attachment.caption.is_empty() || reply_to.is_some() {
        ctx.state
            .message_input_mut()
            .restore_failed_send(&attachment.caption, reply_to);
    }
}
