use std::path::Path;
use std::sync::mpsc::Sender;

use crate::domain::events::BackgroundTaskResult;

pub(super) fn dispatch_open_file(
    tx: &Sender<BackgroundTaskResult>,
    cmd_template: String,
    file_path: String,
) {
    let tx = tx.clone();

    if let Err(error) = std::thread::Builder::new()
        .name("rtg-bg-open-file".into())
        .spawn(move || {
            let parts: Vec<&str> = cmd_template.split_whitespace().collect();
            if parts.is_empty() {
                let _ = tx.send(BackgroundTaskResult::OpenFileFailed {
                    stderr: "empty open command".to_owned(),
                });
                return;
            }

            let resolved: Vec<String> = parts
                .iter()
                .map(|p| p.replace("{file_path}", &file_path))
                .collect();

            let result = std::process::Command::new(&resolved[0])
                .args(&resolved[1..])
                .stdin(std::process::Stdio::null())
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::piped())
                .spawn();

            match result {
                Ok(child) => {
                    let output = child.wait_with_output();
                    match output {
                        Ok(out) if !out.status.success() => {
                            let stderr = String::from_utf8_lossy(&out.stderr).trim().to_owned();
                            let _ = tx.send(BackgroundTaskResult::OpenFileFailed { stderr });
                        }
                        Err(e) => {
                            let _ = tx.send(BackgroundTaskResult::OpenFileFailed {
                                stderr: e.to_string(),
                            });
                        }
                        _ => {} // success — no event needed
                    }
                }
                Err(e) => {
                    let _ = tx.send(BackgroundTaskResult::OpenFileFailed {
                        stderr: e.to_string(),
                    });
                }
            }
        })
    {
        tracing::error!(error = %error, "failed to spawn open file background thread");
    }
}

pub(super) fn dispatch_save_file(
    tx: &Sender<BackgroundTaskResult>,
    file_id: i32,
    local_path: String,
    file_name: Option<String>,
) {
    let tx = tx.clone();

    if let Err(error) = std::thread::Builder::new()
        .name("rtg-bg-save-file".into())
        .spawn(move || {
            let src = Path::new(&local_path);

            let dest_name = file_name
                .as_deref()
                .unwrap_or_else(|| src.file_name().and_then(|n| n.to_str()).unwrap_or("file"));

            let downloads_dir = match dirs::download_dir() {
                Some(d) => d,
                None => {
                    let _ = tx.send(BackgroundTaskResult::FileSaveFailed {
                        file_id,
                        error: "could not determine downloads directory".to_owned(),
                    });
                    return;
                }
            };

            let mut dest = downloads_dir.join(dest_name);

            // Avoid overwriting existing files by appending a counter.
            if dest.exists() {
                let stem = dest
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("file")
                    .to_owned();
                let ext = dest
                    .extension()
                    .and_then(|e| e.to_str())
                    .map(|e| format!(".{e}"))
                    .unwrap_or_default();
                let mut found = false;
                for i in 1u32..=9999 {
                    let candidate = downloads_dir.join(format!("{stem} ({i}){ext}"));
                    if !candidate.exists() {
                        dest = candidate;
                        found = true;
                        break;
                    }
                }
                if !found {
                    let _ = tx.send(BackgroundTaskResult::FileSaveFailed {
                        file_id,
                        error: "too many duplicate files in downloads directory".to_owned(),
                    });
                    return;
                }
            }

            match std::fs::copy(src, &dest) {
                Ok(_) => {
                    let saved_name = dest
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("file")
                        .to_owned();
                    let _ = tx.send(BackgroundTaskResult::FileSaved {
                        file_id,
                        file_name: saved_name,
                    });
                }
                Err(e) => {
                    let _ = tx.send(BackgroundTaskResult::FileSaveFailed {
                        file_id,
                        error: e.to_string(),
                    });
                }
            }
        })
    {
        tracing::error!(error = %error, "failed to spawn save file background thread");
    }
}
