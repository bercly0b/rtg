use crate::usecases::background::TaskDispatcher;

use super::ShellCtx;

/// Edits the chat search query and moves the selection to the next chat
/// whose title contains it.
pub(super) fn handle_chat_search_key<D: TaskDispatcher>(ctx: &mut ShellCtx<'_, D>, key: &str) {
    match key {
        "esc" | "enter" => ctx.state.close_chat_search(),
        "backspace" => {
            let Some(search) = ctx.state.chat_search_mut() else {
                return;
            };
            search.delete_char_before();
            let q = search.query().to_owned();
            if !q.is_empty() {
                ctx.state.chat_list_mut().select_by_query(&q);
            }
        }
        k if k.chars().count() == 1 => {
            let (Some(ch), Some(search)) = (k.chars().next(), ctx.state.chat_search_mut()) else {
                return;
            };
            search.insert_char(ch);
            let q = search.query().to_owned();
            ctx.state.chat_list_mut().select_by_query(&q);
        }
        _ => {}
    }
}
