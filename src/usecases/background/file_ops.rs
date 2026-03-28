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
