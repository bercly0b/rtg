//! Self-update through `axoupdater`: it reads the install receipt the release
//! installer wrote and re-runs the installer of the latest release into the
//! same directory (ADR 0002).

use std::{
    env,
    io::ErrorKind,
    path::{Path, PathBuf},
};

use anyhow::Context;
use axoupdater::{AxoUpdater, Version};
use tokio::runtime::Runtime;

use crate::usecases::self_update::ReleaseInstall;

/// cargo-dist names the app, and with it the receipt, after the package.
const APP_NAME: &str = env!("CARGO_PKG_NAME");

/// Whether the running binary is the one the release installer installed:
/// its install receipt exists and points at this executable's directory.
pub fn is_release_install() -> bool {
    release_installer_updater().is_some()
}

fn release_installer_updater() -> Option<AxoUpdater> {
    let mut updater = AxoUpdater::new_for(APP_NAME);
    if let Err(error) = updater.load_receipt() {
        tracing::debug!(error = %error, "self-update: no usable install receipt");
        return None;
    }

    match updater.check_receipt_is_for_this_executable() {
        Ok(true) => Some(updater),
        Ok(false) => None,
        Err(error) => {
            tracing::debug!(error = %error, "self-update: cannot match the receipt to this executable");
            None
        }
    }
}

pub struct AxoReleaseInstall {
    updater: AxoUpdater,
    executable: PathBuf,
    runtime: Runtime,
}

impl AxoReleaseInstall {
    /// `None` when the running binary was not installed by the release
    /// installer.
    pub fn detect() -> anyhow::Result<Option<Self>> {
        let Some(mut updater) = release_installer_updater() else {
            return Ok(None);
        };

        // The receipt keeps the version of the last installer run; the running
        // binary knows its own, which is also what the update check compares.
        updater.set_current_version(Version::parse(env!("CARGO_PKG_VERSION"))?)?;

        Ok(Some(Self {
            updater,
            executable: env::current_exe()?.canonicalize()?,
            runtime: tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()?,
        }))
    }
}

impl ReleaseInstall for AxoReleaseInstall {
    fn is_update_needed(&mut self) -> anyhow::Result<bool> {
        self.runtime
            .block_on(self.updater.is_update_needed())
            .context("failed to look up the latest release")
    }

    fn executable(&self) -> &Path {
        &self.executable
    }

    fn can_write_install_dir(&self) -> bool {
        self.executable.parent().is_none_or(can_write)
    }

    fn install_latest(&mut self) -> anyhow::Result<String> {
        let result = self
            .runtime
            .block_on(self.updater.run())
            .context("the release installer failed")?
            .context("the latest release is already installed")?;

        Ok(result.new_version.to_string())
    }
}

/// The installer stages the binary in a temporary directory inside the install
/// directory, so this probes the same way. Only a permission error counts:
/// `sudo` does not help with any other failure, which the installer reports.
fn can_write(dir: &Path) -> bool {
    match tempfile::tempdir_in(dir) {
        Ok(_) => true,
        Err(error) => error.kind() != ErrorKind::PermissionDenied,
    }
}

#[cfg(test)]
mod tests {
    use std::{
        ffi::OsString,
        fs,
        io::{Read, Write},
        net::TcpListener,
        os::unix::fs::PermissionsExt,
        thread::{self, JoinHandle},
    };

    use super::*;
    use crate::test_support::env_lock;

    /// Sets `vars` while `f` runs and restores them afterwards. Callers hold
    /// the env lock.
    fn with_env<T>(vars: &[(&str, OsString)], f: impl FnOnce() -> T) -> T {
        let saved: Vec<_> = vars
            .iter()
            .map(|(name, _)| (*name, env::var_os(name)))
            .collect();
        for (name, value) in vars {
            // SAFETY: env is guarded by the process-wide test mutex.
            unsafe { env::set_var(name, value) };
        }

        let result = f();

        for (name, value) in saved {
            // SAFETY: restoring env while the guard is held.
            unsafe {
                match value {
                    Some(value) => env::set_var(name, value),
                    None => env::remove_var(name),
                }
            }
        }
        result
    }

    fn this_executable_dir() -> PathBuf {
        env::current_exe()
            .and_then(fs::canonicalize)
            .expect("current exe")
            .parent()
            .expect("exe dir")
            .to_path_buf()
    }

    /// Writes a receipt the way the release installer does, into `dir`.
    fn write_receipt(dir: &Path, install_prefix: &Path) {
        let receipt = serde_json::json!({
            "install_prefix": install_prefix,
            "install_layout": "flat",
            "binaries": ["rtg"],
            "cdylibs": [],
            "cstaticlibs": [],
            "source": {
                "release_type": "github",
                "owner": "bercly0b",
                "name": "rtg",
                "app_name": "rtg-tui"
            },
            "version": "0.1.0",
            "provider": { "source": "cargo-dist", "version": "0.34.0" },
            "binary_aliases": {},
            "modify_path": true
        });
        fs::write(dir.join("rtg-tui-receipt.json"), receipt.to_string()).expect("write receipt");
    }

    /// Serves `requests` requests: the GitHub latest release, tagged `tag`,
    /// and its installer, which writes the install directory it was given
    /// into `$RTG_TEST_INSTALLER_MARKER`.
    fn serve_release(listener: TcpListener, tag: &'static str, requests: usize) -> JoinHandle<()> {
        let base = format!("http://{}", listener.local_addr().expect("local addr"));
        thread::spawn(move || {
            for stream in listener.incoming().take(requests) {
                let mut stream = stream.expect("accept");
                let mut request = Vec::new();
                let mut buf = [0_u8; 1024];
                while !request.ends_with(b"\r\n\r\n") {
                    let read = stream.read(&mut buf).expect("read request");
                    if read == 0 {
                        break;
                    }
                    request.extend_from_slice(&buf[..read]);
                }
                let request = String::from_utf8(request).expect("utf-8 request");

                let body = if request.starts_with("GET /api/v3/repos/bercly0b/rtg/releases/latest ")
                {
                    let release = serde_json::json!({
                        "tag_name": tag,
                        "name": tag,
                        "url": format!("{base}/release"),
                        "prerelease": false,
                        "assets": [{
                            "name": "rtg-tui-installer.sh",
                            "url": format!("{base}/asset"),
                            "browser_download_url": format!("{base}/rtg-tui-installer.sh")
                        }]
                    });
                    release.to_string()
                } else if request.starts_with("GET /rtg-tui-installer.sh ") {
                    "#!/bin/sh\nprintf '%s' \"$RTG_TUI_INSTALL_DIR\" > \"$RTG_TEST_INSTALLER_MARKER\"\n"
                        .to_owned()
                } else {
                    panic!("unexpected request: {request}");
                };

                let response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                stream
                    .write_all(response.as_bytes())
                    .expect("write response");
            }
        })
    }

    #[test]
    fn receipt_for_this_executable_means_a_release_install() {
        let _guard = env_lock();
        let config = tempfile::tempdir().expect("temp dir");
        write_receipt(config.path(), &this_executable_dir());

        let detected = with_env(
            &[("AXOUPDATER_CONFIG_PATH", config.path().into())],
            is_release_install,
        );

        assert!(detected);
    }

    #[test]
    fn receipt_for_another_directory_is_not_this_release_install() {
        let _guard = env_lock();
        let config = tempfile::tempdir().expect("temp dir");
        let elsewhere = tempfile::tempdir().expect("temp dir");
        write_receipt(config.path(), elsewhere.path());

        let detected = with_env(
            &[("AXOUPDATER_CONFIG_PATH", config.path().into())],
            is_release_install,
        );

        assert!(!detected);
    }

    #[test]
    fn no_receipt_means_no_release_install() {
        let _guard = env_lock();
        let config = tempfile::tempdir().expect("temp dir");

        let detected = with_env(&[("AXOUPDATER_CONFIG_PATH", config.path().into())], || {
            AxoReleaseInstall::detect().expect("detect").is_some()
        });

        assert!(!detected);
    }

    #[test]
    fn newer_release_installer_runs_into_the_receipt_install_dir() {
        let _guard = env_lock();
        let config = tempfile::tempdir().expect("temp dir");
        let install_prefix = this_executable_dir();
        write_receipt(config.path(), &install_prefix);
        let marker = config.path().join("installed-into");
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind localhost");
        let api = format!("http://{}/", listener.local_addr().expect("local addr"));
        let server = serve_release(listener, "v99.0.0", 2);

        let (needed, installed) = with_env(
            &[
                ("AXOUPDATER_CONFIG_PATH", config.path().into()),
                ("RTG_TUI_INSTALLER_GHE_BASE_URL", api.into()),
                ("RTG_TEST_INSTALLER_MARKER", marker.clone().into()),
            ],
            || {
                let mut install = AxoReleaseInstall::detect()
                    .expect("detect")
                    .expect("release install");
                let needed = install.is_update_needed().expect("update check");
                (needed, install.install_latest().expect("install"))
            },
        );
        server.join().expect("server thread");

        assert!(needed);
        assert_eq!(installed, "99.0.0");
        assert_eq!(
            fs::read_to_string(&marker).expect("installer ran"),
            install_prefix.to_string_lossy()
        );
    }

    #[test]
    fn running_version_is_not_updated_to_itself() {
        let _guard = env_lock();
        let config = tempfile::tempdir().expect("temp dir");
        write_receipt(config.path(), &this_executable_dir());
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind localhost");
        let api = format!("http://{}/", listener.local_addr().expect("local addr"));
        let tag = concat!("v", env!("CARGO_PKG_VERSION"));
        let server = serve_release(listener, tag, 1);

        let needed = with_env(
            &[
                ("AXOUPDATER_CONFIG_PATH", config.path().into()),
                ("RTG_TUI_INSTALLER_GHE_BASE_URL", api.into()),
            ],
            || {
                AxoReleaseInstall::detect()
                    .expect("detect")
                    .expect("release install")
                    .is_update_needed()
                    .expect("update check")
            },
        );

        server.join().expect("server thread");

        assert!(!needed);
    }

    #[test]
    fn directory_without_write_permission_is_not_writable() {
        let dir = tempfile::tempdir().expect("temp dir");
        let bin = dir.path().join("bin");
        fs::create_dir(&bin).expect("bin dir");

        assert!(can_write(&bin));

        fs::set_permissions(&bin, fs::Permissions::from_mode(0o555)).expect("chmod");
        let read_only = can_write(&bin);
        fs::set_permissions(&bin, fs::Permissions::from_mode(0o755)).expect("chmod back");

        assert!(!read_only);
        assert_eq!(fs::read_dir(&bin).expect("read dir").count(), 0);
    }
}
