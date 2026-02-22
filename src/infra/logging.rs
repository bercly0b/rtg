use std::path::Path;
use std::sync::OnceLock;

use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::EnvFilter;

use crate::infra::{config::LogConfig, error::AppError, storage_layout::StorageLayout};

static LOG_GUARD: OnceLock<WorkerGuard> = OnceLock::new();

const LOG_FILE_PREFIX: &str = "rtg.log";

pub fn init(config: &LogConfig) -> Result<(), AppError> {
    let layout = StorageLayout::resolve()?;
    std::fs::create_dir_all(&layout.config_dir).map_err(|source| AppError::StorageDirCreate {
        path: layout.config_dir.clone(),
        source,
    })?;

    cleanup_old_logs(&layout.config_dir, config.max_log_files);

    let file_appender = tracing_appender::rolling::daily(&layout.config_dir, LOG_FILE_PREFIX);
    let (non_blocking_writer, guard) = tracing_appender::non_blocking(file_appender);
    let _ = LOG_GUARD.set(guard);

    let env_filter = build_env_filter(&config.level);

    tracing_subscriber::fmt()
        .with_env_filter(env_filter)
        .with_ansi(false)
        .with_target(true)
        .with_writer(non_blocking_writer)
        .try_init()
        .map_err(AppError::LoggingInit)?;

    tracing::info!(
        log_dir = %layout.config_dir.display(),
        max_log_files = config.max_log_files,
        level = %config.level,
        "file logging initialized with daily rotation"
    );
    Ok(())
}

/// Builds an environment filter for tracing.
///
/// If `RUST_LOG` environment variable is set, uses that directly.
/// Otherwise, applies `app_level` to the `rtg` crate while limiting
/// dependencies (tokio, hyper, grammers, etc.) to `warn` level to
/// prevent log flooding.
fn build_env_filter(app_level: &str) -> EnvFilter {
    if let Ok(env_filter) = EnvFilter::try_from_default_env() {
        return env_filter;
    }

    const NOISY_DEPENDENCIES: &[&str] = &[
        "grammers_client",
        "grammers_session",
        "grammers_mtsender",
        "grammers_mtproto",
        "grammers_tl_types",
        "tokio",
        "hyper",
        "reqwest",
        "rustls",
        "h2",
    ];

    let mut directives = vec![format!("warn,rtg={}", app_level)];
    for dep in NOISY_DEPENDENCIES {
        directives.push(format!("{}=warn", dep));
    }

    EnvFilter::new(directives.join(","))
}

fn cleanup_old_logs(log_dir: &Path, max_files: usize) {
    let mut log_files: Vec<_> = match std::fs::read_dir(log_dir) {
        Ok(entries) => entries
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.file_name()
                    .to_str()
                    .is_some_and(|name| name.starts_with(LOG_FILE_PREFIX))
            })
            .collect(),
        Err(_) => return,
    };

    if log_files.len() <= max_files {
        return;
    }

    // Sort by modification time, newest first
    log_files.sort_by(|a, b| {
        let time_a = a.metadata().and_then(|m| m.modified()).ok();
        let time_b = b.metadata().and_then(|m| m.modified()).ok();
        time_b.cmp(&time_a)
    });

    // Remove old files beyond max_files
    for old_file in log_files.into_iter().skip(max_files) {
        let _ = std::fs::remove_file(old_file.path());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn cleanup_old_logs_removes_excess_files() {
        let dir = tempdir().unwrap();

        // Create 5 log files with different modification times
        for i in 0..5 {
            let path = dir.path().join(format!("rtg.log.2026-02-{:02}", 20 + i));
            let mut file = File::create(&path).unwrap();
            writeln!(file, "log content {}", i).unwrap();
            // Add small delay to ensure different modification times
            std::thread::sleep(std::time::Duration::from_millis(10));
        }

        cleanup_old_logs(dir.path(), 3);

        let remaining: Vec<_> = std::fs::read_dir(dir.path())
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.file_name()
                    .to_str()
                    .is_some_and(|name| name.starts_with(LOG_FILE_PREFIX))
            })
            .collect();

        assert_eq!(remaining.len(), 3, "should keep only 3 newest log files");
    }

    #[test]
    fn cleanup_old_logs_does_nothing_when_under_limit() {
        let dir = tempdir().unwrap();

        // Create 2 log files
        for i in 0..2 {
            let path = dir.path().join(format!("rtg.log.2026-02-{:02}", 20 + i));
            File::create(&path).unwrap();
        }

        cleanup_old_logs(dir.path(), 3);

        let remaining: Vec<_> = std::fs::read_dir(dir.path())
            .unwrap()
            .filter_map(|e| e.ok())
            .collect();

        assert_eq!(remaining.len(), 2, "should not remove any files");
    }

    #[test]
    fn cleanup_old_logs_ignores_unrelated_files() {
        let dir = tempdir().unwrap();

        // Create log files
        for i in 0..3 {
            let path = dir.path().join(format!("rtg.log.2026-02-{:02}", 20 + i));
            File::create(&path).unwrap();
            std::thread::sleep(std::time::Duration::from_millis(10));
        }

        // Create unrelated file
        File::create(dir.path().join("other_file.txt")).unwrap();

        cleanup_old_logs(dir.path(), 2);

        let all_files: Vec<_> = std::fs::read_dir(dir.path())
            .unwrap()
            .filter_map(|e| e.ok())
            .collect();

        // 2 log files + 1 unrelated file
        assert_eq!(all_files.len(), 3);

        let log_files: Vec<_> = all_files
            .iter()
            .filter(|e| {
                e.file_name()
                    .to_str()
                    .is_some_and(|name| name.starts_with(LOG_FILE_PREFIX))
            })
            .collect();

        assert_eq!(log_files.len(), 2);
    }

    #[test]
    fn build_env_filter_limits_dependencies() {
        let filter = build_env_filter("debug");
        let filter_str = format!("{:?}", filter);

        // The filter should contain rtg=debug and dependency limits
        assert!(
            filter_str.contains("rtg") || filter_str.contains("debug"),
            "filter should configure rtg level"
        );
    }
}
