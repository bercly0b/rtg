/// The way RTG got onto the user's machine; it decides which command updates it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstallChannel {
    ReleaseInstaller,
    Homebrew,
    CratesIo,
    /// None of the above: a build from source, a copied binary or an install
    /// from an older release.
    Unknown,
}

impl InstallChannel {
    /// The command that updates this install; for an unknown channel, the
    /// release page to update it from by hand.
    pub fn update_hint(self) -> &'static str {
        match self {
            Self::ReleaseInstaller => "rtg update",
            Self::Homebrew => "brew upgrade rtg",
            Self::CratesIo => "cargo install --locked rtg-tui",
            Self::Unknown => "https://github.com/bercly0b/rtg/releases/latest",
        }
    }
}

/// A release newer than the running RTG, found by the update check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AvailableUpdate {
    /// Release version without the `v` tag prefix, e.g. `0.2.0`.
    pub version: String,
    pub channel: InstallChannel,
}

/// An update the user started from the TUI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SelfUpdateProgress {
    /// `rtg update` runs in the background.
    Running,
    /// The release is installed; the running RTG keeps its version until it
    /// restarts.
    Installed { version: String },
}

/// Whether release `candidate` is newer than `current`.
///
/// Versions are `MAJOR.MINOR.PATCH` with an optional `v` prefix and an
/// optional pre-release suffix (`0.2.0-rc.1`), which ranks below its release.
/// An unparsable version is never newer.
pub fn is_newer_version(candidate: &str, current: &str) -> bool {
    let (Some((candidate, candidate_pre)), Some((current, current_pre))) =
        (parse_version(candidate), parse_version(current))
    else {
        return false;
    };

    candidate > current || (candidate == current && current_pre && !candidate_pre)
}

fn parse_version(version: &str) -> Option<((u64, u64, u64), bool)> {
    let version = version.strip_prefix('v').unwrap_or(version);
    let (core, is_pre_release) = match version.split_once('-') {
        Some((core, _)) => (core, true),
        None => (version, false),
    };

    let mut numbers = core.split('.').map(|part| part.parse::<u64>().ok());
    let triple = (numbers.next()??, numbers.next()??, numbers.next()??);
    if numbers.next().is_some() {
        return None;
    }

    Some((triple, is_pre_release))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn higher_patch_minor_or_major_is_newer() {
        assert!(is_newer_version("0.1.14", "0.1.13"));
        assert!(is_newer_version("0.2.0", "0.1.13"));
        assert!(is_newer_version("1.0.0", "0.9.9"));
    }

    #[test]
    fn same_or_lower_version_is_not_newer() {
        assert!(!is_newer_version("0.1.13", "0.1.13"));
        assert!(!is_newer_version("0.1.12", "0.1.13"));
        assert!(!is_newer_version("0.1.99", "0.2.0"));
    }

    #[test]
    fn components_compare_as_numbers_not_strings() {
        assert!(is_newer_version("0.1.10", "0.1.9"));
    }

    #[test]
    fn v_prefix_is_ignored() {
        assert!(is_newer_version("v0.1.14", "0.1.13"));
        assert!(!is_newer_version("v0.1.13", "0.1.13"));
    }

    #[test]
    fn release_is_newer_than_its_pre_release() {
        assert!(is_newer_version("0.2.0", "0.2.0-rc.1"));
        assert!(!is_newer_version("0.2.0-rc.1", "0.2.0"));
        assert!(!is_newer_version("0.1.13", "0.2.0-rc.1"));
    }

    #[test]
    fn unparsable_version_is_never_newer() {
        for (candidate, current) in [
            ("latest", "0.1.13"),
            ("0.2", "0.1.13"),
            ("0.2.0.1", "0.1.13"),
            ("", "0.1.13"),
            ("0.2.0", "dev"),
        ] {
            assert!(
                !is_newer_version(candidate, current),
                "{candidate} vs {current}"
            );
        }
    }

    #[test]
    fn each_install_channel_has_its_own_update_hint() {
        assert_eq!(InstallChannel::ReleaseInstaller.update_hint(), "rtg update");
        assert_eq!(InstallChannel::Homebrew.update_hint(), "brew upgrade rtg");
        assert_eq!(
            InstallChannel::CratesIo.update_hint(),
            "cargo install --locked rtg-tui"
        );
        assert_eq!(
            InstallChannel::Unknown.update_hint(),
            "https://github.com/bercly0b/rtg/releases/latest"
        );
    }
}
