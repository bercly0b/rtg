//! External command management: spawning user-configured commands (voice
//! recording, media playback), streaming their output, and process control.
//!
//! The command runs in a background thread. Its output is streamed through
//! an mpsc channel that the UI event source polls to update the command
//! popup in real time.

mod command;
mod file_list;
pub(crate) mod handle;
mod output;

pub use command::start_command;
pub use file_list::{start_file_list, FileListEvent, FileListHandle};
pub use handle::CommandHandle;

#[cfg(test)]
mod tests;
