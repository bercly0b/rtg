//! Self-update: `rtg update` re-runs the release installer of the latest
//! release over the install it made.

use std::path::{Path, PathBuf};

use crate::domain::update_check::InstallChannel;

/// An install made by the release installer, for the running binary.
pub trait ReleaseInstall {
    /// Whether the latest release is newer than the running RTG; asks the
    /// release source.
    fn is_update_needed(&mut self) -> anyhow::Result<bool>;
    /// The running binary; the installer replaces it in its directory.
    fn executable(&self) -> &Path;
    fn can_write_install_dir(&self) -> bool;
    /// Runs the installer of the latest release and returns its version.
    fn install_latest(&mut self) -> anyhow::Result<String>;
}

#[derive(Debug, PartialEq, Eq)]
pub enum SelfUpdated {
    AlreadyLatest,
    To { version: String },
}

#[derive(Debug, thiserror::Error)]
pub enum SelfUpdateError {
    #[error(
        "rtg update only updates an rtg installed by the release installer. {}",
        how_to_update(.0)
    )]
    OtherInstallChannel(InstallChannel),
    #[error("rtg cannot write to {}. Update it as root:\n\n    {command}", .dir.display())]
    InstallDirNotWritable { dir: PathBuf, command: String },
    #[error(transparent)]
    Failed(#[from] anyhow::Error),
}

fn how_to_update(channel: &InstallChannel) -> String {
    match channel {
        InstallChannel::Unknown => format!(
            "Download the latest release from:\n\n    {}",
            channel.update_hint()
        ),
        _ => format!("Update this one with:\n\n    {}", channel.update_hint()),
    }
}

/// Installs the latest release when it is newer than the running RTG.
///
/// The install directory is checked before the installer runs, so a directory
/// that needs root ends with a ready `sudo` command instead of the
/// installer's permission error.
pub fn self_update(install: &mut dyn ReleaseInstall) -> Result<SelfUpdated, SelfUpdateError> {
    if !install.is_update_needed()? {
        return Ok(SelfUpdated::AlreadyLatest);
    }

    if !install.can_write_install_dir() {
        let exe = install.executable();
        return Err(SelfUpdateError::InstallDirNotWritable {
            dir: exe.parent().unwrap_or(exe).to_path_buf(),
            command: format!("sudo {} update", shell_words::quote(&exe.to_string_lossy())),
        });
    }

    let version = install.install_latest()?;
    Ok(SelfUpdated::To { version })
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeInstall {
        latest: Option<&'static str>,
        writable: bool,
        executable: PathBuf,
        installed: Option<&'static str>,
    }

    impl FakeInstall {
        fn with_latest(latest: Option<&'static str>) -> Self {
            Self {
                latest,
                writable: true,
                executable: PathBuf::from("/Users/me/.cargo/bin/rtg"),
                installed: None,
            }
        }
    }

    impl ReleaseInstall for FakeInstall {
        fn is_update_needed(&mut self) -> anyhow::Result<bool> {
            Ok(self.latest.is_some())
        }

        fn executable(&self) -> &Path {
            &self.executable
        }

        fn can_write_install_dir(&self) -> bool {
            self.writable
        }

        fn install_latest(&mut self) -> anyhow::Result<String> {
            self.installed = self.latest;
            self.latest
                .map(str::to_owned)
                .ok_or_else(|| anyhow::anyhow!("nothing to install"))
        }
    }

    #[test]
    fn newer_release_is_installed() {
        let mut install = FakeInstall::with_latest(Some("0.2.0"));

        let updated = self_update(&mut install).expect("update");

        assert_eq!(
            updated,
            SelfUpdated::To {
                version: "0.2.0".to_owned()
            }
        );
        assert_eq!(install.installed, Some("0.2.0"));
    }

    #[test]
    fn nothing_is_installed_when_rtg_is_the_latest_release() {
        let mut install = FakeInstall::with_latest(None);

        let updated = self_update(&mut install).expect("update");

        assert_eq!(updated, SelfUpdated::AlreadyLatest);
        assert_eq!(install.installed, None);
    }

    #[test]
    fn up_to_date_install_is_not_sent_to_sudo() {
        let mut install = FakeInstall {
            writable: false,
            ..FakeInstall::with_latest(None)
        };

        assert_eq!(
            self_update(&mut install).expect("update"),
            SelfUpdated::AlreadyLatest
        );
    }

    #[test]
    fn install_dir_that_needs_root_gets_a_sudo_command_instead_of_the_installer() {
        let mut install = FakeInstall {
            writable: false,
            executable: PathBuf::from("/usr/local/bin/rtg"),
            ..FakeInstall::with_latest(Some("0.2.0"))
        };

        let error = self_update(&mut install).expect_err("needs root");

        assert_eq!(install.installed, None);
        assert_eq!(
            error.to_string(),
            "rtg cannot write to /usr/local/bin. Update it as root:\n\n    sudo /usr/local/bin/rtg update"
        );
    }

    #[test]
    fn sudo_command_quotes_a_path_with_spaces() {
        let mut install = FakeInstall {
            writable: false,
            executable: PathBuf::from("/opt/my tools/bin/rtg"),
            ..FakeInstall::with_latest(Some("0.2.0"))
        };

        let Err(SelfUpdateError::InstallDirNotWritable { dir, command }) =
            self_update(&mut install)
        else {
            panic!("expected InstallDirNotWritable");
        };

        assert_eq!(dir, PathBuf::from("/opt/my tools/bin"));
        assert_eq!(command, "sudo '/opt/my tools/bin/rtg' update");
    }

    #[test]
    fn other_install_channel_is_pointed_at_its_own_update_command() {
        assert_eq!(
            SelfUpdateError::OtherInstallChannel(InstallChannel::Homebrew).to_string(),
            "rtg update only updates an rtg installed by the release installer. Update this one with:\n\n    brew upgrade rtg"
        );
        assert_eq!(
            SelfUpdateError::OtherInstallChannel(InstallChannel::CratesIo).to_string(),
            "rtg update only updates an rtg installed by the release installer. Update this one with:\n\n    cargo install --locked rtg-tui"
        );
    }

    #[test]
    fn unknown_install_channel_is_pointed_at_the_release_page() {
        assert_eq!(
            SelfUpdateError::OtherInstallChannel(InstallChannel::Unknown).to_string(),
            "rtg update only updates an rtg installed by the release installer. Download the latest release from:\n\n    https://github.com/bercly0b/rtg/releases/latest"
        );
    }
}
