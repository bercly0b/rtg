use crate::{domain::shell_state::ActivePane, usecases::background::TaskDispatcher};

use super::OrchestratorCtx;

pub(super) fn handle_message_input_key<D: TaskDispatcher>(
    ctx: &mut OrchestratorCtx<'_, D>,
    key: &str,
) {
    match key {
        "esc" => {
            if ctx.state.message_input().editing().is_some() {
                ctx.state.message_input_mut().clear();
            }
            ctx.state.message_input_mut().clear_reply_to();
            ctx.state.message_input_mut().clear_editing();
            ctx.state.set_active_pane(ActivePane::Messages);
        }
        "enter" => {
            if ctx.state.message_input().editing().is_some() {
                try_edit_message(ctx);
            } else {
                try_send_message(ctx);
            }
        }
        "backspace" => ctx.state.message_input_mut().delete_char_before(),
        "delete" => ctx.state.message_input_mut().delete_char_at(),
        "left" => ctx.state.message_input_mut().move_cursor_left(),
        "right" => ctx.state.message_input_mut().move_cursor_right(),
        "home" => ctx.state.message_input_mut().move_cursor_home(),
        "end" => ctx.state.message_input_mut().move_cursor_end(),
        // Single character input
        ch if ch.chars().count() == 1 => {
            if let Some(c) = ch.chars().next() {
                ctx.state.message_input_mut().insert_char(c);
            }
        }
        _ => {}
    }
}

fn try_edit_message<D: TaskDispatcher>(ctx: &mut OrchestratorCtx<'_, D>) {
    let text = ctx.state.message_input().text().to_string();
    let trimmed = text.trim();

    if trimmed.is_empty() {
        return;
    }

    let edit_ctx = ctx.state.message_input_mut().take_editing().unwrap();

    if trimmed == edit_ctx.original_text.trim() {
        ctx.state.message_input_mut().clear();
        ctx.state.set_active_pane(ActivePane::Messages);
        return;
    }

    ctx.state.message_input_mut().clear();
    ctx.state.set_active_pane(ActivePane::Messages);

    ctx.state
        .open_chat_mut()
        .update_message_text(edit_ctx.message_id, trimmed.to_owned());

    ctx.dispatcher
        .dispatch_edit_message(edit_ctx.chat_id, edit_ctx.message_id, text);
}

fn try_send_message<D: TaskDispatcher>(ctx: &mut OrchestratorCtx<'_, D>) {
    let text = ctx.state.message_input().text().to_string();
    let trimmed = text.trim();

    // Validate locally -- empty/whitespace messages are rejected immediately
    if trimmed.is_empty() {
        return;
    }

    let Some(chat_id) = ctx.state.open_chat().chat_id() else {
        return;
    };

    tracing::debug!(chat_id, "dispatching send message to background");

    // Extract reply context before clearing input
    let reply_context = ctx.state.message_input_mut().take_reply_to();
    let reply_to_message_id = reply_context.as_ref().map(|r| r.message_id);
    let pending_reply_info = reply_context.map(|r| crate::domain::message::ReplyInfo {
        sender_name: r.sender_name.clone(),
        is_outgoing: r.sender_name == "You",
        text: r.text,
    });

    // Optimistically clear the input and show the message immediately
    ctx.state.message_input_mut().clear();
    ctx.state.open_chat_mut().add_pending_message(
        trimmed.to_owned(),
        crate::domain::message::MessageMedia::None,
        pending_reply_info,
    );
    ctx.dispatcher
        .dispatch_send_message(chat_id, text.clone(), reply_to_message_id);
}
