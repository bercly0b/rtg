use anyhow::Result;

use crate::{
    domain::message::{open_targets, LinkTarget},
    infra::contracts::ExternalOpener,
    usecases::background::TaskDispatcher,
};

use super::{
    chat_open::{self, OpenChatSync},
    ShellCtx,
};

/// Opens what the selected message points at: its only link or mention
/// straight away, or the link picker when it holds several.
pub(super) fn open_message_link<D: TaskDispatcher>(
    ctx: &mut ShellCtx<'_, D>,
    sync: &mut OpenChatSync,
    opener: &dyn ExternalOpener,
) -> Result<()> {
    let Some(msg) = ctx.state.open_chat().selected_message() else {
        return Ok(());
    };
    let mut targets = open_targets(&msg.display_content(), &msg.links);
    if targets.len() > 1 {
        ctx.state.open_link_picker(targets);
        return Ok(());
    }
    match targets.pop() {
        Some(target) => open_target(ctx, sync, opener, target),
        None => Ok(()),
    }
}

pub(super) fn handle_link_picker_key<D: TaskDispatcher>(
    ctx: &mut ShellCtx<'_, D>,
    sync: &mut OpenChatSync,
    opener: &dyn ExternalOpener,
    key: &str,
) -> Result<()> {
    match key {
        "j" => {
            if let Some(picker) = ctx.state.link_picker_mut() {
                picker.select_next();
            }
        }
        "k" => {
            if let Some(picker) = ctx.state.link_picker_mut() {
                picker.select_previous();
            }
        }
        "enter" | "o" => {
            let target = ctx
                .state
                .link_picker()
                .and_then(|picker| picker.selected().cloned());
            ctx.state.close_link_picker();
            if let Some(target) = target {
                open_target(ctx, sync, opener, target)?;
            }
        }
        "esc" | "q" => ctx.state.close_link_picker(),
        _ => {}
    }
    Ok(())
}

fn open_target<D: TaskDispatcher>(
    ctx: &mut ShellCtx<'_, D>,
    sync: &mut OpenChatSync,
    opener: &dyn ExternalOpener,
    target: LinkTarget,
) -> Result<()> {
    match target {
        LinkTarget::Url(url) => opener.open(&url),
        LinkTarget::Mention(username) => {
            chat_open::open_mention(ctx, sync, username);
            Ok(())
        }
    }
}
