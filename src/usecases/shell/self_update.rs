use crate::{
    domain::update_check::SelfUpdateProgress,
    usecases::{background::TaskDispatcher, self_update::SelfUpdated},
};

use super::ShellCtx;

/// Starts `rtg update` in the background. While it runs, and after it has
/// installed a release, the key does nothing: running it again would reinstall
/// the same release, since the running RTG keeps its version until it
/// restarts.
pub(super) fn start_self_update<D: TaskDispatcher>(shell: &mut ShellCtx<'_, D>) {
    if shell.state.self_update().is_some() {
        return;
    }

    shell
        .state
        .set_self_update(Some(SelfUpdateProgress::Running));
    shell.dispatcher.dispatch_self_update();
}

pub(super) fn handle_self_update_finished<D: TaskDispatcher>(
    shell: &mut ShellCtx<'_, D>,
    result: Result<SelfUpdated, String>,
) {
    match result {
        Ok(SelfUpdated::To { version }) => {
            tracing::info!(version, "self-update: installed");
            shell
                .state
                .set_self_update(Some(SelfUpdateProgress::Installed { version }));
        }
        Ok(SelfUpdated::AlreadyLatest) => {
            shell.state.set_self_update(None);
            shell.state.set_notification(concat!(
                "rtg ",
                env!("CARGO_PKG_VERSION"),
                " is the latest release"
            ));
        }
        Err(message) => {
            shell.state.set_self_update(None);
            shell.state.set_notification(message);
        }
    }
}
