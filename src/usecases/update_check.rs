//! Update check: finds out whether a release newer than the running RTG exists.

use std::{
    sync::mpsc::Sender,
    time::{Duration, SystemTime},
};

use crate::{
    domain::update_check::{is_newer_version, AvailableUpdate, InstallChannel},
    usecases::background::BackgroundTaskResult,
};

/// GitHub is asked at most once per this interval; in between, the latest
/// version from the last check is used.
pub const CHECK_INTERVAL: Duration = Duration::from_secs(24 * 60 * 60);

pub trait LatestReleaseSource {
    /// Version of the latest stable release, without the `v` tag prefix.
    fn latest_version(&self) -> anyhow::Result<String>;
}

/// The outcome of the last successful request to the release source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LastUpdateCheck {
    pub checked_at: SystemTime,
    pub latest_version: String,
}

pub trait UpdateCheckCache {
    /// `None` when nothing is cached or the cache cannot be read.
    fn load(&self) -> Option<LastUpdateCheck>;
    fn save(&self, check: &LastUpdateCheck) -> anyhow::Result<()>;
}

/// Returns the latest release version when it is newer than `current_version`.
///
/// Asks `source` only when the cached check is older than [`CHECK_INTERVAL`]
/// (or missing) and caches a successful answer. When the request fails, the
/// cached version is used however old it is, so a known update stays visible
/// offline; the next start asks again.
pub fn check_for_update(
    source: &dyn LatestReleaseSource,
    cache: &dyn UpdateCheckCache,
    current_version: &str,
    now: SystemTime,
) -> Option<String> {
    let last = cache.load();
    let latest = match last.as_ref().filter(|last| is_fresh(last, now)) {
        Some(last) => last.latest_version.clone(),
        None => match source.latest_version() {
            Ok(latest_version) => {
                let check = LastUpdateCheck {
                    checked_at: now,
                    latest_version,
                };
                if let Err(error) = cache.save(&check) {
                    tracing::warn!(error = %error, "update check: failed to cache the result");
                }
                check.latest_version
            }
            Err(error) => {
                tracing::warn!(error = %error, "update check: failed to fetch the latest release");
                last?.latest_version
            }
        },
    };

    is_newer_version(&latest, current_version).then_some(latest)
}

/// Runs [`check_for_update`] on its own thread, so startup never waits for the
/// network, and sends a newer release to the shell as a background result.
/// The install channel is detected only when there is an update to show.
pub fn spawn_update_check(
    source: impl LatestReleaseSource + Send + 'static,
    cache: impl UpdateCheckCache + Send + 'static,
    current_version: &'static str,
    detect_channel: impl FnOnce() -> InstallChannel + Send + 'static,
    results: Sender<BackgroundTaskResult>,
) {
    if let Err(error) = std::thread::Builder::new()
        .name("rtg-update-check".into())
        .spawn(move || {
            let Some(version) =
                check_for_update(&source, &cache, current_version, SystemTime::now())
            else {
                return;
            };
            let update = AvailableUpdate {
                version,
                channel: detect_channel(),
            };
            tracing::info!(version = %update.version, channel = ?update.channel, "update check: newer release available");
            let _ = results.send(BackgroundTaskResult::UpdateAvailable(update));
        })
    {
        tracing::warn!(error = %error, "update check: failed to spawn thread");
    }
}

/// A check dated in the future (the clock moved back) is not fresh.
fn is_fresh(last: &LastUpdateCheck, now: SystemTime) -> bool {
    now.duration_since(last.checked_at)
        .is_ok_and(|age| age < CHECK_INTERVAL)
}

#[cfg(test)]
mod tests {
    use std::{
        cell::{Cell, RefCell},
        sync::mpsc,
    };

    use super::*;

    const CURRENT: &str = "0.1.13";

    struct FakeSource {
        result: Result<&'static str, &'static str>,
        calls: Cell<usize>,
    }

    impl FakeSource {
        fn returning(version: &'static str) -> Self {
            Self {
                result: Ok(version),
                calls: Cell::new(0),
            }
        }

        fn failing() -> Self {
            Self {
                result: Err("network is unreachable"),
                calls: Cell::new(0),
            }
        }
    }

    impl LatestReleaseSource for FakeSource {
        fn latest_version(&self) -> anyhow::Result<String> {
            self.calls.set(self.calls.get() + 1);
            self.result
                .map(str::to_owned)
                .map_err(|error| anyhow::anyhow!(error))
        }
    }

    #[derive(Default)]
    struct FakeCache {
        stored: RefCell<Option<LastUpdateCheck>>,
    }

    impl FakeCache {
        fn with(checked_at: SystemTime, latest_version: &str) -> Self {
            Self {
                stored: RefCell::new(Some(LastUpdateCheck {
                    checked_at,
                    latest_version: latest_version.to_owned(),
                })),
            }
        }

        fn stored(&self) -> Option<LastUpdateCheck> {
            self.stored.borrow().clone()
        }
    }

    impl UpdateCheckCache for FakeCache {
        fn load(&self) -> Option<LastUpdateCheck> {
            self.stored()
        }

        fn save(&self, check: &LastUpdateCheck) -> anyhow::Result<()> {
            *self.stored.borrow_mut() = Some(check.clone());
            Ok(())
        }
    }

    fn now() -> SystemTime {
        SystemTime::UNIX_EPOCH + Duration::from_secs(1_800_000_000)
    }

    fn hours_ago(hours: u64) -> SystemTime {
        now() - Duration::from_secs(hours * 60 * 60)
    }

    #[test]
    fn first_check_asks_the_source_and_caches_the_answer() {
        let source = FakeSource::returning("0.1.14");
        let cache = FakeCache::default();

        let update = check_for_update(&source, &cache, CURRENT, now());

        assert_eq!(update.as_deref(), Some("0.1.14"));
        assert_eq!(source.calls.get(), 1);
        assert_eq!(
            cache.stored(),
            Some(LastUpdateCheck {
                checked_at: now(),
                latest_version: "0.1.14".to_owned(),
            })
        );
    }

    #[test]
    fn no_update_when_the_latest_release_is_the_running_one() {
        let source = FakeSource::returning(CURRENT);
        let cache = FakeCache::default();

        assert_eq!(check_for_update(&source, &cache, CURRENT, now()), None);
        assert_eq!(cache.stored().unwrap().latest_version, CURRENT);
    }

    #[test]
    fn check_within_a_day_uses_the_cache_without_asking_the_source() {
        let source = FakeSource::returning("0.1.15");
        let cache = FakeCache::with(hours_ago(23), "0.1.14");

        let update = check_for_update(&source, &cache, CURRENT, now());

        assert_eq!(update.as_deref(), Some("0.1.14"));
        assert_eq!(source.calls.get(), 0);
        assert_eq!(cache.stored().unwrap().checked_at, hours_ago(23));
    }

    #[test]
    fn cached_version_is_not_reported_once_rtg_is_updated_to_it() {
        let source = FakeSource::returning("0.1.14");
        let cache = FakeCache::with(hours_ago(1), "0.1.14");

        assert_eq!(check_for_update(&source, &cache, "0.1.14", now()), None);
        assert_eq!(source.calls.get(), 0);
    }

    #[test]
    fn check_older_than_a_day_asks_the_source_again() {
        let source = FakeSource::returning("0.1.15");
        let cache = FakeCache::with(hours_ago(24), "0.1.14");

        let update = check_for_update(&source, &cache, CURRENT, now());

        assert_eq!(update.as_deref(), Some("0.1.15"));
        assert_eq!(source.calls.get(), 1);
        assert_eq!(
            cache.stored(),
            Some(LastUpdateCheck {
                checked_at: now(),
                latest_version: "0.1.15".to_owned(),
            })
        );
    }

    #[test]
    fn check_dated_in_the_future_asks_the_source_again() {
        let source = FakeSource::returning("0.1.14");
        let cache = FakeCache::with(now() + Duration::from_secs(60), CURRENT);

        let update = check_for_update(&source, &cache, CURRENT, now());

        assert_eq!(update.as_deref(), Some("0.1.14"));
        assert_eq!(source.calls.get(), 1);
    }

    #[test]
    fn failed_request_without_cache_reports_nothing_and_caches_nothing() {
        let source = FakeSource::failing();
        let cache = FakeCache::default();

        assert_eq!(check_for_update(&source, &cache, CURRENT, now()), None);
        assert_eq!(cache.stored(), None);
    }

    #[test]
    fn failed_request_falls_back_to_the_stale_cache_and_keeps_its_date() {
        let source = FakeSource::failing();
        let cache = FakeCache::with(hours_ago(72), "0.1.14");

        let update = check_for_update(&source, &cache, CURRENT, now());

        assert_eq!(update.as_deref(), Some("0.1.14"));
        assert_eq!(cache.stored().unwrap().checked_at, hours_ago(72));
    }

    #[test]
    fn spawned_check_sends_a_newer_release_with_the_install_channel() {
        let (tx, rx) = mpsc::channel();

        spawn_update_check(
            FakeSource::returning("0.1.14"),
            FakeCache::default(),
            CURRENT,
            || InstallChannel::Homebrew,
            tx,
        );

        assert_eq!(
            rx.recv_timeout(Duration::from_secs(5)),
            Ok(BackgroundTaskResult::UpdateAvailable(AvailableUpdate {
                version: "0.1.14".to_owned(),
                channel: InstallChannel::Homebrew,
            }))
        );
    }

    #[test]
    fn spawned_check_sends_nothing_without_a_newer_release() {
        let (tx, rx) = mpsc::channel();

        spawn_update_check(
            FakeSource::returning(CURRENT),
            FakeCache::default(),
            CURRENT,
            || InstallChannel::CratesIo,
            tx,
        );

        assert_eq!(
            rx.recv_timeout(Duration::from_secs(5)),
            Err(mpsc::RecvTimeoutError::Disconnected)
        );
    }
}
