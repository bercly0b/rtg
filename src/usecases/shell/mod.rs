mod background_results;
mod chat_list;
mod chat_open;
mod chat_search;
mod chat_updates;
mod command_popup;
mod file_updates;
mod forum;
mod key_dispatch;
mod message_actions;
mod message_input;
mod popups;
mod self_update;
mod voice;

use std::{collections::HashMap, sync::Arc};

use anyhow::Result;

use crate::{
    domain::{
        chat_list_state::ChatListUiState,
        keymap::{KeyContext, Keymap, ResolveResult},
        message_cache::DEFAULT_MIN_DISPLAY_MESSAGES,
        shell_state::{ActivePane, ShellState},
    },
    infra::contracts::{ExternalOpener, StorageAdapter},
};

// Re-exported for test modules that rely on `use super::*`.
#[cfg(test)]
use crate::{
    domain::{chat::ChatType, events::ChatUpdate, open_chat_state::MessageSource},
    usecases::chat_subtitle::ChatSubtitleQuery,
};

use super::{
    background::TaskDispatcher,
    contracts::{AppEvent, ShellEffect, ShellOrchestrator},
    load_messages::CachedMessagesSource,
};
use chat_list::ChatListLoads;
use chat_open::OpenChatSync;
use command_popup::MediaCommands;
use file_updates::Downloads;

/// Default limit for cached message preloading.
const DEFAULT_CACHED_MESSAGES_LIMIT: usize = 50;

/// Orchestrator settings that come from the app config.
pub struct OrchestratorConfig {
    /// Minimum number of cached messages required to display them immediately.
    /// If the cache holds fewer messages, the UI shows Loading instead of a
    /// sparse preview (eliminates the "1 message flash" artifact).
    pub min_display_messages: usize,
    /// Voice recording command template.
    pub voice_record_cmd: String,
    /// MIME-type → command mappings for opening message files.
    pub open_handlers: HashMap<String, String>,
    /// Maximum file size (in bytes) for auto-download.
    pub max_auto_download_bytes: u64,
    /// Action name → key pattern overrides of the default keymap.
    pub key_overrides: HashMap<String, String>,
}

impl Default for OrchestratorConfig {
    fn default() -> Self {
        Self {
            min_display_messages: DEFAULT_MIN_DISPLAY_MESSAGES,
            voice_record_cmd: super::voice_recording::DEFAULT_RECORD_CMD.to_owned(),
            open_handlers: HashMap::new(),
            max_auto_download_bytes: 10_000_000,
            key_overrides: HashMap::new(),
        }
    }
}

/// What every handler gets: the state and the outputs to background tasks
/// and to the shell loop.
///
/// Handlers take the orchestrator zones they touch (`ChatListLoads`,
/// `OpenChatSync`, `Downloads`, `MediaCommands`) as separate arguments, so a
/// signature states what a handler can change.
struct ShellCtx<'a, D: TaskDispatcher> {
    pub state: &'a mut ShellState,
    pub dispatcher: &'a D,
    pub effects: &'a mut Vec<ShellEffect>,
}

/// The whole orchestrator, borrowed. Only routers (`handle_event`,
/// `key_dispatch`, `background_results`, `chat_updates`) take it, to hand
/// each handler the zones it needs.
struct OrchestratorCtx<'a, D: TaskDispatcher> {
    pub shell: ShellCtx<'a, D>,
    pub chat_list_loads: &'a mut ChatListLoads,
    pub open_chat_sync: &'a mut OpenChatSync,
    pub downloads: &'a mut Downloads,
    pub media: &'a mut MediaCommands,
    pub opener: &'a dyn ExternalOpener,
}

pub struct DefaultShellOrchestrator<S, O, D>
where
    S: StorageAdapter,
    O: ExternalOpener,
    D: TaskDispatcher,
{
    state: ShellState,
    storage: S,
    opener: O,
    dispatcher: D,
    /// Effects queued for the shell loop, drained by `take_effects`.
    effects: Vec<ShellEffect>,
    chat_list_loads: ChatListLoads,
    open_chat_sync: OpenChatSync,
    downloads: Downloads,
    media: MediaCommands,
    /// When `true`, the orchestrator was initialised with cached data and needs
    /// a background refresh on the first Tick to pick up server-side changes.
    initial_refresh_needed: bool,
    keymap: Keymap,
}

impl<S, O, D> DefaultShellOrchestrator<S, O, D>
where
    S: StorageAdapter,
    O: ExternalOpener,
    D: TaskDispatcher,
{
    #[cfg_attr(not(test), allow(dead_code, reason = "used only by tests"))]
    pub fn new(storage: S, opener: O, dispatcher: D) -> Self {
        Self::new_with_initial_state(
            storage,
            opener,
            dispatcher,
            ShellState::default(),
            None,
            OrchestratorConfig::default(),
        )
    }

    /// Creates an orchestrator pre-populated with an initial state.
    ///
    /// When the initial state already has a `Ready` chat list (e.g. from
    /// TDLib cache), `initial_refresh_needed` is set so the first Tick
    /// triggers a background refresh to pick up server-side changes.
    ///
    /// `cache_source` provides synchronous access to TDLib's local cache
    /// for instant message display when opening chats.
    pub fn new_with_initial_state(
        storage: S,
        opener: O,
        dispatcher: D,
        initial_state: ShellState,
        cache_source: Option<Arc<dyn CachedMessagesSource>>,
        config: OrchestratorConfig,
    ) -> Self {
        let initial_refresh_needed = initial_state.chat_list().ui_state() == ChatListUiState::Ready;
        Self {
            state: initial_state,
            storage,
            opener,
            dispatcher,
            effects: Vec::new(),
            chat_list_loads: ChatListLoads::default(),
            open_chat_sync: OpenChatSync::new(cache_source, config.min_display_messages),
            downloads: Downloads::new(config.max_auto_download_bytes),
            media: MediaCommands::new(config.voice_record_cmd, config.open_handlers),
            initial_refresh_needed,
            keymap: if config.key_overrides.is_empty() {
                Keymap::default()
            } else {
                Keymap::with_overrides(&config.key_overrides)
            },
        }
    }

    fn as_ctx(&mut self) -> OrchestratorCtx<'_, D> {
        OrchestratorCtx {
            shell: ShellCtx {
                state: &mut self.state,
                dispatcher: &self.dispatcher,
                effects: &mut self.effects,
            },
            chat_list_loads: &mut self.chat_list_loads,
            open_chat_sync: &mut self.open_chat_sync,
            downloads: &mut self.downloads,
            media: &mut self.media,
            opener: &self.opener,
        }
    }

    fn handle_chat_list_action(&mut self, action: crate::domain::keymap::Action) -> Result<()> {
        let save_action = key_dispatch::dispatch_chat_list_action(&mut self.as_ctx(), action)?;
        if save_action {
            self.storage.save_last_action("open_chat")?;
        }
        Ok(())
    }

    fn handle_messages_action(&mut self, action: crate::domain::keymap::Action) -> Result<()> {
        key_dispatch::dispatch_messages_action(&mut self.as_ctx(), action)
    }

    fn handle_forum_topic_list_action(
        &mut self,
        action: crate::domain::keymap::Action,
    ) -> Result<()> {
        key_dispatch::dispatch_forum_topic_list_action(&mut self.as_ctx(), action)
    }

    #[cfg(test)]
    fn send_voice_recording(&mut self) {
        let ctx = &mut self.as_ctx();
        voice::send_voice_recording(&mut ctx.shell, ctx.media);
    }

    #[cfg(test)]
    fn discard_voice_recording(&mut self) {
        voice::discard_voice_recording(&mut self.media);
    }
}

impl<S, O, D> ShellOrchestrator for DefaultShellOrchestrator<S, O, D>
where
    S: StorageAdapter,
    O: ExternalOpener,
    D: TaskDispatcher,
{
    fn state(&self) -> &ShellState {
        &self.state
    }

    fn state_mut(&mut self) -> &mut ShellState {
        &mut self.state
    }

    fn keymap(&self) -> &Keymap {
        &self.keymap
    }

    fn handle_event(&mut self, event: AppEvent) -> Result<()> {
        match event {
            AppEvent::Tick => {
                let refresh_chat_list =
                    if self.state.chat_list().ui_state() == ChatListUiState::Loading {
                        true
                    } else {
                        std::mem::take(&mut self.initial_refresh_needed)
                    };
                if refresh_chat_list {
                    let ctx = &mut self.as_ctx();
                    chat_list::dispatch_chat_list_refresh(
                        &mut ctx.shell,
                        ctx.chat_list_loads,
                        false,
                    );
                }
                self.state.open_chat_mut().typing_state_mut().expire_stale();
                self.storage.save_last_action("tick")?;
            }
            AppEvent::QuitRequested => {
                let ctx = &mut self.as_ctx();
                chat_open::close_tdlib_chat(&mut ctx.shell, ctx.open_chat_sync);
                self.state.stop();
            }
            AppEvent::CommandOutputLine { text, replace_last } => {
                if let Some(popup) = self.state.command_popup_mut() {
                    if replace_last {
                        popup.replace_last_line(text);
                    } else {
                        popup.push_line(text);
                    }
                }
            }
            AppEvent::CommandExited { success } => {
                tracing::info!(success, "external command exited");
                let ctx = &mut self.as_ctx();
                command_popup::handle_command_exited(&mut ctx.shell, ctx.media, success);
            }
            AppEvent::InputKey(key) => {
                if key_dispatch::dispatch_modal_key(&mut self.as_ctx(), &key.key) {
                    return Ok(());
                }

                let context = match self.state.active_pane() {
                    ActivePane::ChatList if self.state.forum_topic_list().is_some() => {
                        KeyContext::ForumTopicList
                    }
                    ActivePane::ChatList => KeyContext::ChatList,
                    ActivePane::Messages => KeyContext::Messages,
                    ActivePane::MessageInput => unreachable!(),
                };

                match self.keymap.resolve(&key.key, key.ctrl, context) {
                    ResolveResult::Action(action) => match context {
                        KeyContext::ChatList => self.handle_chat_list_action(action)?,
                        KeyContext::ForumTopicList => {
                            self.handle_forum_topic_list_action(action)?
                        }
                        KeyContext::Messages => self.handle_messages_action(action)?,
                        KeyContext::Global => {}
                    },
                    ResolveResult::Pending | ResolveResult::Unmatched => {}
                }
            }
            AppEvent::ConnectivityChanged(status) => {
                self.state.set_connectivity_status(status);
            }
            AppEvent::ChatUpdateReceived { updates } => {
                tracing::debug!(count = updates.len(), "orchestrator received chat updates");
                chat_updates::handle_chat_updates(&mut self.as_ctx(), updates);
            }
            AppEvent::BackgroundTasksCompleted(results) => {
                for result in results {
                    background_results::handle_background_result(&mut self.as_ctx(), result);
                }
            }
        }

        Ok(())
    }

    fn take_effects(&mut self) -> Vec<ShellEffect> {
        std::mem::take(&mut self.effects)
    }
}

#[cfg(test)]
mod tests;
