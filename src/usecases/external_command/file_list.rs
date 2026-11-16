//! Runs the file list command of the file picker: reads the paths it prints
//! and finds out the size and modification time of each file.

use std::{
    io::{BufRead, BufReader, Read},
    path::Path,
    process::{Child, Command, Stdio},
    sync::{mpsc, Arc},
    thread,
    time::{Duration, UNIX_EPOCH},
};

#[cfg(unix)]
use std::os::unix::process::CommandExt;

use crate::domain::{
    command_template::{self, DIRS_PLACEHOLDER},
    file_picker_state::FileEntry,
};

use super::handle::SharedChild;

/// How long files pile up before they are reported together: long enough to
/// keep a fast command from flooding the UI with one-file updates, short
/// enough that the list seems to fill at once.
const BATCH_WINDOW: Duration = Duration::from_millis(20);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileListEvent {
    /// Files the command has named since the previous event.
    Entries(Vec<FileEntry>),
    /// The command is done. `error` says why there is nothing to show; a
    /// command that failed after naming files ends without one.
    Finished { error: Option<String> },
}

/// A running file list command; dropping the handle kills it.
pub struct FileListHandle {
    child: Arc<SharedChild>,
}

impl Drop for FileListHandle {
    fn drop(&mut self) {
        self.child.kill();
    }
}

/// Starts `template` over `dirs` and reports what it lists through
/// `on_event`, always ending with [`FileListEvent::Finished`].
///
/// `~` in `dirs` stands for the home directory, and directories that do not
/// exist are left out. Returns `None` when the command did not start;
/// `on_event` has got the reason by then.
pub fn start_file_list(
    template: &str,
    dirs: &[String],
    mut on_event: impl FnMut(FileListEvent) + Send + 'static,
) -> Option<FileListHandle> {
    let mut fail = |error: String| {
        on_event(FileListEvent::Finished { error: Some(error) });
        None
    };

    let dirs = existing_dirs(dirs, dirs::home_dir().as_deref());
    let argv = match command_template::resolve_list_argv(template, &dirs) {
        Ok(argv) => argv,
        Err(error) => return fail(format!("[attach] list_cmd: {error}")),
    };
    if dirs.is_empty() && template.contains(DIRS_PLACEHOLDER) {
        return fail("None of the [attach] dirs exist".to_owned());
    }

    let mut child = match spawn(&argv) {
        Ok(child) => child,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return fail(format!("command not found: {}", argv[0]));
        }
        Err(error) => return fail(format!("failed to start {}: {error}", argv[0])),
    };
    let stdout = child.stdout.take().expect("stdout is piped");
    let stderr = child.stderr.take().expect("stderr is piped");
    let child = Arc::new(SharedChild::new(child));

    let waited_child = Arc::clone(&child);
    let spawned = thread::Builder::new()
        .name("rtg-file-list".into())
        .spawn(move || report(stdout, stderr, &waited_child, on_event));
    if let Err(error) = spawned {
        tracing::error!(%error, "failed to spawn the file list thread");
        child.kill();
        child.wait();
        return None;
    }

    Some(FileListHandle { child })
}

fn spawn(argv: &[String]) -> std::io::Result<Child> {
    let mut command = Command::new(&argv[0]);
    command
        .args(&argv[1..])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    // Its own process group, so that killing it takes its children along.
    #[cfg(unix)]
    command.process_group(0);
    command.spawn()
}

fn report(
    stdout: impl Read + Send + 'static,
    stderr: impl Read + Send + 'static,
    child: &SharedChild,
    mut on_event: impl FnMut(FileListEvent),
) {
    let (entry_tx, entry_rx) = mpsc::channel();
    let stdout_reader = thread::spawn(move || {
        for path in BufReader::new(stdout).lines().map_while(Result::ok) {
            if let Some(entry) = file_entry(path) {
                if entry_tx.send(entry).is_err() {
                    return;
                }
            }
        }
    });
    let stderr_reader = thread::spawn(move || first_line(stderr));

    let mut listed_any = false;
    while let Ok(first) = entry_rx.recv() {
        thread::sleep(BATCH_WINDOW);
        let mut entries = vec![first];
        entries.extend(entry_rx.try_iter());
        listed_any = true;
        on_event(FileListEvent::Entries(entries));
    }

    let _ = stdout_reader.join();
    let success = child.wait();
    let stderr_line = stderr_reader.join().ok().flatten();
    let error = (!success && !listed_any)
        .then(|| stderr_line.unwrap_or_else(|| "The file list command failed".to_owned()));
    on_event(FileListEvent::Finished { error });
}

/// The file behind a line the command printed, `None` unless the line is
/// the path of an existing regular file.
fn file_entry(path: String) -> Option<FileEntry> {
    let metadata = std::fs::metadata(&path).ok()?;
    if !metadata.is_file() {
        return None;
    }
    let modified_unix_s = metadata
        .modified()
        .ok()
        .and_then(|modified| modified.duration_since(UNIX_EPOCH).ok())
        .map_or(0, |since_epoch| since_epoch.as_secs() as i64);
    Some(FileEntry {
        path,
        size: metadata.len(),
        modified_unix_s,
    })
}

/// The first non-empty line of `output`, which is read to the end so that
/// the command never blocks on a full pipe.
fn first_line(output: impl Read) -> Option<String> {
    let mut first = None;
    for line in BufReader::new(output).lines().map_while(Result::ok) {
        if first.is_none() && !line.trim().is_empty() {
            first = Some(line.trim().to_owned());
        }
    }
    first
}

/// Expands a leading `~` to `home` and keeps the directories that exist.
pub(super) fn existing_dirs(dirs: &[String], home: Option<&Path>) -> Vec<String> {
    dirs.iter()
        .filter_map(|dir| {
            let expanded = match (dir.strip_prefix('~'), home) {
                (Some(rest), Some(home)) if rest.is_empty() || rest.starts_with('/') => {
                    format!("{}{rest}", home.display())
                }
                _ => dir.clone(),
            };
            Path::new(&expanded).is_dir().then_some(expanded)
        })
        .collect()
}
