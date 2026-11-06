use std::collections::{HashMap, VecDeque};
use std::time::{Duration, Instant};

/// Smaller files finish too fast for a speed reading to be useful: it would
/// only flash in the status bar for a moment.
pub const MIN_TRACKED_FILE_SIZE: u64 = 5_000_000;

const SPEED_WINDOW: Duration = Duration::from_secs(3);

/// A shorter span gives a noisy first reading and lets downloads that finish
/// within it blink in and out of the status bar.
const MIN_MEASURED_SPAN: Duration = Duration::from_secs(1);

/// Combined download speed of the large files that are downloading right now.
///
/// Fed with each file's running downloaded byte count; the speed is averaged
/// over a sliding window so it does not jump on every progress update.
#[derive(Debug, Clone, Default)]
pub struct DownloadSpeedMeter {
    downloaded_by_file: HashMap<i32, u64>,
    received_total: u64,
    /// `(time, received_total)` points; the front one is the latest point at
    /// or before the window start, so the window always has a baseline.
    samples: VecDeque<(Instant, u64)>,
}

impl DownloadSpeedMeter {
    pub fn record_progress(
        &mut self,
        file_id: i32,
        file_size: u64,
        downloaded_size: u64,
        now: Instant,
    ) {
        if file_size < MIN_TRACKED_FILE_SIZE {
            return;
        }

        // The first report only sets the baseline: bytes downloaded before it
        // (e.g. a resumed download) were not received in the window.
        if let Some(previous) = self.downloaded_by_file.insert(file_id, downloaded_size) {
            self.received_total += downloaded_size.saturating_sub(previous);
        }

        self.samples.push_back((now, self.received_total));
        while self
            .samples
            .get(1)
            .is_some_and(|&(at, _)| at + SPEED_WINDOW <= now)
        {
            self.samples.pop_front();
        }
    }

    /// Stops tracking a completed or cancelled download.
    pub fn finish(&mut self, file_id: i32) {
        self.downloaded_by_file.remove(&file_id);
        if self.downloaded_by_file.is_empty() {
            *self = Self::default();
        }
    }

    /// Returns `None` when no large file is downloading or it has not been
    /// downloading long enough to measure.
    pub fn bytes_per_second(&self, now: Instant) -> Option<u64> {
        let &(baseline_at, baseline_total) = self.samples.front()?;
        let &(latest_at, _) = self.samples.back()?;

        // Measuring up to the latest report keeps the value steady between
        // reports (the status bar redraws far more often than they arrive);
        // once reports stop for a whole window, the speed decays towards zero.
        let end = now
            .checked_sub(SPEED_WINDOW)
            .map_or(latest_at, |window_start| window_start.max(latest_at));
        let span = end.saturating_duration_since(baseline_at);
        if span < MIN_MEASURED_SPAN {
            return None;
        }

        let received = self.received_total - baseline_total;
        Some((received as f64 / span.as_secs_f64()) as u64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const LARGE: u64 = MIN_TRACKED_FILE_SIZE;

    fn secs(n: f64) -> Duration {
        Duration::from_secs_f64(n)
    }

    #[test]
    fn idle_meter_has_no_speed() {
        let meter = DownloadSpeedMeter::default();

        assert_eq!(meter.bytes_per_second(Instant::now()), None);
    }

    #[test]
    fn measures_bytes_received_since_first_report() {
        let start = Instant::now();
        let mut meter = DownloadSpeedMeter::default();

        meter.record_progress(1, LARGE, 0, start);
        meter.record_progress(1, LARGE, 1_000_000, start + secs(1.0));
        meter.record_progress(1, LARGE, 2_000_000, start + secs(2.0));

        assert_eq!(meter.bytes_per_second(start + secs(2.0)), Some(1_000_000));
    }

    #[test]
    fn bytes_downloaded_before_first_report_are_not_counted() {
        let start = Instant::now();
        let mut meter = DownloadSpeedMeter::default();

        meter.record_progress(1, LARGE, 3_000_000, start);
        meter.record_progress(1, LARGE, 3_500_000, start + secs(1.0));

        assert_eq!(meter.bytes_per_second(start + secs(1.0)), Some(500_000));
    }

    #[test]
    fn small_files_are_not_tracked() {
        let start = Instant::now();
        let mut meter = DownloadSpeedMeter::default();

        meter.record_progress(1, LARGE - 1, 0, start);
        meter.record_progress(1, LARGE - 1, 1_000_000, start + secs(1.0));

        assert_eq!(meter.bytes_per_second(start + secs(1.0)), None);
    }

    #[test]
    fn no_speed_until_measured_long_enough() {
        let start = Instant::now();
        let mut meter = DownloadSpeedMeter::default();

        meter.record_progress(1, LARGE, 0, start);
        meter.record_progress(1, LARGE, 400_000, start + secs(0.5));
        assert_eq!(meter.bytes_per_second(start + secs(0.9)), None);

        meter.record_progress(1, LARGE, 800_000, start + secs(1.0));
        assert_eq!(meter.bytes_per_second(start + secs(1.0)), Some(800_000));
    }

    #[test]
    fn speed_follows_recent_window_only() {
        let start = Instant::now();
        let mut meter = DownloadSpeedMeter::default();

        meter.record_progress(1, LARGE, 0, start);
        meter.record_progress(1, LARGE, 10_000_000, start + secs(1.0));
        for second in 2..=5 {
            meter.record_progress(
                1,
                LARGE,
                10_000_000 + (second - 1) * 1_000_000,
                start + secs(second as f64),
            );
        }

        assert_eq!(meter.bytes_per_second(start + secs(5.0)), Some(1_000_000));
    }

    #[test]
    fn concurrent_downloads_add_up() {
        let start = Instant::now();
        let mut meter = DownloadSpeedMeter::default();

        meter.record_progress(1, LARGE, 0, start);
        meter.record_progress(2, LARGE, 0, start);
        meter.record_progress(1, LARGE, 1_000_000, start + secs(1.0));
        meter.record_progress(2, LARGE, 500_000, start + secs(1.0));

        assert_eq!(meter.bytes_per_second(start + secs(1.0)), Some(1_500_000));
    }

    #[test]
    fn speed_holds_steady_between_progress_reports() {
        let start = Instant::now();
        let mut meter = DownloadSpeedMeter::default();

        meter.record_progress(1, LARGE, 0, start);
        meter.record_progress(1, LARGE, 2_000_000, start + secs(1.0));

        assert_eq!(meter.bytes_per_second(start + secs(1.0)), Some(2_000_000));
        assert_eq!(meter.bytes_per_second(start + secs(4.0)), Some(2_000_000));
    }

    #[test]
    fn speed_decays_once_progress_stalls_for_a_window() {
        let start = Instant::now();
        let mut meter = DownloadSpeedMeter::default();

        meter.record_progress(1, LARGE, 0, start);
        meter.record_progress(1, LARGE, 2_000_000, start + secs(1.0));

        assert_eq!(meter.bytes_per_second(start + secs(5.0)), Some(1_000_000));
    }

    #[test]
    fn speed_disappears_when_last_download_finishes() {
        let start = Instant::now();
        let mut meter = DownloadSpeedMeter::default();

        meter.record_progress(1, LARGE, 0, start);
        meter.record_progress(2, LARGE, 0, start);
        meter.record_progress(1, LARGE, 1_000_000, start + secs(1.0));

        meter.finish(1);
        assert!(meter.bytes_per_second(start + secs(1.0)).is_some());

        meter.finish(2);
        assert_eq!(meter.bytes_per_second(start + secs(1.0)), None);
    }
}
