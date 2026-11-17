use crate::{domain::info_picker_state::info_entries, usecases::background::TaskDispatcher};

use super::{popups, ShellCtx};

/// Shows the chat info popup for the selected message: about its sender
/// straight away, or through the info picker when it holds a mention.
pub(super) fn show_message_chat_info<D: TaskDispatcher>(ctx: &mut ShellCtx<'_, D>) {
    let Some(msg) = ctx.state.open_chat().selected_message() else {
        return;
    };
    let mut entries = info_entries(msg);
    if entries.len() > 1 {
        ctx.state.open_info_picker(entries);
    } else if let Some(entry) = entries.pop() {
        popups::open_chat_info_popup(ctx, entry.subject, entry.label);
    }
}

pub(super) fn handle_info_picker_key<D: TaskDispatcher>(ctx: &mut ShellCtx<'_, D>, key: &str) {
    match key {
        "j" => {
            if let Some(picker) = ctx.state.info_picker_mut() {
                picker.select_next();
            }
        }
        "k" => {
            if let Some(picker) = ctx.state.info_picker_mut() {
                picker.select_previous();
            }
        }
        "enter" | "I" => {
            let entry = ctx
                .state
                .info_picker()
                .and_then(|picker| picker.selected().cloned());
            ctx.state.close_info_picker();
            if let Some(entry) = entry {
                popups::open_chat_info_popup(ctx, entry.subject, entry.label);
            }
        }
        "esc" | "q" => ctx.state.close_info_picker(),
        _ => {}
    }
}
