//! Update check adapters: the GitHub release source, the cache file and the
//! install channel detection.

use std::{
    env,
    ffi::OsString,
    fs,
    path::{Path, PathBuf},
    time::{Duration, UNIX_EPOCH},
};

use anyhow::Context;
use serde::{Deserialize, Serialize};

use crate::{
    domain::update_check::InstallChannel,
    infra::self_update,
    usecases::update_check::{LastUpdateCheck, LatestReleaseSource, UpdateCheckCache},
};

const LATEST_RELEASE_URL: &str = "https://api.github.com/repos/bercly0b/rtg/releases/latest";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);

/// Reads the latest stable release from the GitHub API, which skips drafts
/// and pre-releases.
pub struct GitHubLatestRelease {
    url: String,
}

impl Default for GitHubLatestRelease {
    fn default() -> Self {
        Self {
            url: LATEST_RELEASE_URL.to_owned(),
        }
    }
}

#[derive(Deserialize)]
struct Release {
    tag_name: String,
}

impl LatestReleaseSource for GitHubLatestRelease {
    fn latest_version(&self) -> anyhow::Result<String> {
        let agent: ureq::Agent = ureq::Agent::config_builder()
            .timeout_global(Some(REQUEST_TIMEOUT))
            .user_agent(concat!("rtg/", env!("CARGO_PKG_VERSION")))
            .build()
            .into();

        let body = agent
            .get(&self.url)
            .header("Accept", "application/vnd.github+json")
            .call()
            .context("GitHub latest release request failed")?
            .body_mut()
            .read_to_string()
            .context("GitHub latest release response is unreadable")?;

        let release: Release =
            serde_json::from_str(&body).context("GitHub latest release has no tag_name")?;

        Ok(release
            .tag_name
            .strip_prefix('v')
            .unwrap_or(&release.tag_name)
            .to_owned())
    }
}

/// Keeps the last update check in a TOML file in the cache directory.
pub struct FileUpdateCheckCache {
    path: PathBuf,
}

impl FileUpdateCheckCache {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }
}

#[derive(Serialize, Deserialize)]
struct CachedCheck {
    /// Unix time in seconds.
    checked_at: u64,
    latest_version: String,
}

impl UpdateCheckCache for FileUpdateCheckCache {
    fn load(&self) -> Option<LastUpdateCheck> {
        let raw = fs::read_to_string(&self.path).ok()?;
        let cached: CachedCheck = toml::from_str(&raw).ok()?;

        Some(LastUpdateCheck {
            checked_at: UNIX_EPOCH + Duration::from_secs(cached.checked_at),
            latest_version: cached.latest_version,
        })
    }

    fn save(&self, check: &LastUpdateCheck) -> anyhow::Result<()> {
        let cached = CachedCheck {
            checked_at: check.checked_at.duration_since(UNIX_EPOCH)?.as_secs(),
            latest_version: check.latest_version.clone(),
        };
        fs::write(&self.path, toml::to_string(&cached)?)
            .with_context(|| format!("failed to write {}", self.path.display()))
    }
}

/// The install receipt goes first: the release installer and `cargo install`
/// share `~/.cargo/bin`, and only the receipt tells them apart. Its check is
/// the one `rtg update` makes, so the status bar suggests `rtg update` only
/// where it works. The other channels are recognised by where the running
/// binary lives.
pub fn detect_install_channel() -> InstallChannel {
    let exe = env::current_exe().and_then(fs::canonicalize).ok();
    let cargo_bin = cargo_bin_dir(env::var_os("CARGO_HOME"), env::var_os("HOME"));

    install_channel(
        exe.as_deref(),
        self_update::is_release_install(),
        cargo_bin.as_deref(),
    )
}

fn install_channel(
    exe: Option<&Path>,
    is_release_install: bool,
    cargo_bin: Option<&Path>,
) -> InstallChannel {
    if is_release_install {
        InstallChannel::ReleaseInstaller
    } else if exe.is_some_and(is_in_homebrew_cellar) {
        InstallChannel::Homebrew
    } else if cargo_bin.is_some_and(|bin| exe.and_then(Path::parent) == Some(bin)) {
        InstallChannel::CratesIo
    } else {
        InstallChannel::Unknown
    }
}

/// `cargo install` puts binaries into `$CARGO_HOME/bin`, `~/.cargo/bin` by
/// default. The path is canonical to compare with the running binary's.
fn cargo_bin_dir(cargo_home: Option<OsString>, home: Option<OsString>) -> Option<PathBuf> {
    let cargo_home = cargo_home
        .map(PathBuf::from)
        .or_else(|| home.map(|home| PathBuf::from(home).join(".cargo")))?;
    let bin = cargo_home.join("bin");

    Some(fs::canonicalize(&bin).unwrap_or(bin))
}

/// Homebrew keeps every formula under `<prefix>/Cellar` and only links it
/// into `<prefix>/bin`; the canonical binary path goes through the Cellar.
fn is_in_homebrew_cellar(exe: &Path) -> bool {
    exe.components()
        .any(|component| component.as_os_str() == "Cellar")
}

#[cfg(test)]
mod tests {
    use std::{
        io::{Read, Write},
        net::TcpListener,
        thread::{self, JoinHandle},
        time::SystemTime,
    };

    use super::*;

    /// Serves one HTTP response on localhost and hands back the raw request.
    fn serve_once(status: &str, body: &str) -> (GitHubLatestRelease, JoinHandle<String>) {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind localhost");
        let url = format!(
            "http://{}/repos/bercly0b/rtg/releases/latest",
            listener.local_addr().expect("local addr")
        );
        let response = format!(
            "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );

        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept");
            let mut request = Vec::new();
            let mut buf = [0_u8; 1024];
            while !request.ends_with(b"\r\n\r\n") {
                let read = stream.read(&mut buf).expect("read request");
                if read == 0 {
                    break;
                }
                request.extend_from_slice(&buf[..read]);
            }
            stream
                .write_all(response.as_bytes())
                .expect("write response");
            String::from_utf8(request).expect("utf-8 request")
        });

        (GitHubLatestRelease { url }, server)
    }

    #[test]
    fn github_source_returns_the_latest_tag_without_v_prefix() {
        let (source, server) = serve_once(
            "200 OK",
            r#"{"tag_name":"v0.1.14","name":"v0.1.14","prerelease":false}"#,
        );

        let version = source.latest_version().expect("latest version");
        let request = server.join().expect("server thread").to_ascii_lowercase();

        assert_eq!(version, "0.1.14");
        assert!(request.starts_with("get /repos/bercly0b/rtg/releases/latest "));
        assert!(request.contains("accept: application/vnd.github+json"));
        assert!(request.contains(&format!("user-agent: rtg/{}", env!("CARGO_PKG_VERSION"))));
    }

    #[test]
    fn github_source_fails_on_error_status() {
        let (source, server) = serve_once("404 Not Found", r#"{"message":"Not Found"}"#);

        assert!(source.latest_version().is_err());
        server.join().expect("server thread");
    }

    #[test]
    fn github_source_fails_without_tag_name() {
        let (source, server) = serve_once("200 OK", r#"{"name":"v0.1.14"}"#);

        assert!(source.latest_version().is_err());
        server.join().expect("server thread");
    }

    #[test]
    fn cache_round_trips_the_last_check() {
        let dir = tempfile::tempdir().expect("temp dir");
        let cache = FileUpdateCheckCache::new(dir.path().join("update_check.toml"));
        let check = LastUpdateCheck {
            checked_at: UNIX_EPOCH + Duration::from_secs(1_800_000_000),
            latest_version: "0.1.14".to_owned(),
        };

        cache.save(&check).expect("save");

        assert_eq!(cache.load(), Some(check));
    }

    #[test]
    fn cache_is_empty_when_the_file_is_missing_or_broken() {
        let dir = tempfile::tempdir().expect("temp dir");
        let path = dir.path().join("update_check.toml");
        let cache = FileUpdateCheckCache::new(path.clone());

        assert_eq!(cache.load(), None);

        fs::write(&path, "checked_at = \"yesterday\"").expect("write");
        assert_eq!(cache.load(), None);
    }

    #[test]
    fn cache_save_fails_when_the_directory_is_missing() {
        let dir = tempfile::tempdir().expect("temp dir");
        let cache = FileUpdateCheckCache::new(dir.path().join("missing").join("update_check.toml"));
        let check = LastUpdateCheck {
            checked_at: SystemTime::now(),
            latest_version: "0.1.14".to_owned(),
        };

        assert!(cache.save(&check).is_err());
    }

    const CARGO_BIN: &str = "/Users/me/.cargo/bin";

    fn channel_of(exe: &str, is_release_install: bool) -> InstallChannel {
        install_channel(
            Some(Path::new(exe)),
            is_release_install,
            Some(Path::new(CARGO_BIN)),
        )
    }

    #[test]
    fn receipt_means_the_release_installer_wherever_the_binary_is() {
        for exe in [
            "/Users/me/.cargo/bin/rtg",
            "/Users/me/.local/bin/rtg",
            "/opt/homebrew/Cellar/rtg/0.1.14/bin/rtg",
        ] {
            assert_eq!(
                channel_of(exe, true),
                InstallChannel::ReleaseInstaller,
                "{exe}"
            );
        }
    }

    #[test]
    fn binary_in_the_homebrew_cellar_is_homebrew() {
        for exe in [
            "/opt/homebrew/Cellar/rtg/0.1.14/bin/rtg",
            "/usr/local/Cellar/rtg/0.1.14/bin/rtg",
            "/home/linuxbrew/.linuxbrew/Cellar/rtg/0.1.14/bin/rtg",
        ] {
            assert_eq!(channel_of(exe, false), InstallChannel::Homebrew, "{exe}");
        }
    }

    #[test]
    fn binary_in_the_cargo_bin_dir_without_a_receipt_is_crates_io() {
        assert_eq!(
            channel_of("/Users/me/.cargo/bin/rtg", false),
            InstallChannel::CratesIo
        );
    }

    #[test]
    fn binary_anywhere_else_is_unknown() {
        for exe in [
            "/usr/local/bin/rtg",
            "/Users/me/.local/bin/rtg",
            "/Users/me/src/rtg/target/release/rtg",
            "/Users/me/.cargo/bin/tools/rtg",
        ] {
            assert_eq!(channel_of(exe, false), InstallChannel::Unknown, "{exe}");
        }

        assert_eq!(
            install_channel(None, false, Some(Path::new(CARGO_BIN))),
            InstallChannel::Unknown
        );
        assert_eq!(
            install_channel(Some(Path::new("/Users/me/.cargo/bin/rtg")), false, None),
            InstallChannel::Unknown
        );
    }

    #[test]
    fn cargo_bin_dir_is_in_cargo_home_or_in_home() {
        assert_eq!(
            cargo_bin_dir(Some("/opt/cargo".into()), Some("/Users/me".into())),
            Some(PathBuf::from("/opt/cargo/bin"))
        );
        assert_eq!(
            cargo_bin_dir(None, Some("/Users/me".into())),
            Some(PathBuf::from("/Users/me/.cargo/bin"))
        );
        assert_eq!(cargo_bin_dir(None, None), None);
    }

    #[test]
    fn cargo_bin_dir_is_canonical_like_the_binary_path() {
        let dir = tempfile::tempdir().expect("temp dir");
        let real = dir.path().join("real");
        fs::create_dir_all(real.join("bin")).expect("create bin");
        let link = dir.path().join("link");
        std::os::unix::fs::symlink(&real, &link).expect("symlink");

        assert_eq!(
            cargo_bin_dir(Some(link.into_os_string()), None),
            Some(fs::canonicalize(real.join("bin")).expect("canonical bin"))
        );
    }
}
