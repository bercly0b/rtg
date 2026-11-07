# Download speed in the status bar

While a file of at least 5 MB is downloading, the status bar shows the download
speed right after the connectivity label:

```
 ● Connected  ↓ 6.6 MB/s                                             ? for help
```

- **Units**: bytes per second with binary (1024) multiples, the same
  calculation as tmux-net-speed and similar system monitors, so the two can be
  compared side by side: `B/s`, `KB/s`, `MB/s`. Values up to 99 get one
  decimal, truncated rather than rounded (`6.6 MB/s`, `512 KB/s`).
- **Only large files**: smaller files (voice messages, photos, most
  auto-downloads) finish in a moment, and a speed reading would only flash in
  the status bar. The threshold is `MIN_TRACKED_FILE_SIZE` in
  `domain/download_speed.rs`.
- **No blinking**: the speed appears only after one second of measured
  progress, so a download that finishes faster never shows it, and disappears as
  soon as the last large download completes or is cancelled.
- **Several downloads** are added up into one speed.
- **Smoothing**: the speed is averaged over the last 3 seconds and changes only
  when progress arrives. If no progress arrives for 3 seconds, it decays towards
  zero until the download resumes.

## TDLib integration

TDLib has no download-speed API. Downloads started with `downloadFile` report
progress through `updateFile`, whose `file.local.downloaded_size` is the total
number of downloaded bytes and is meant for progress calculation. The speed is
the growth of that number over time:

- The update loop forwards `updateFile` as `TdLibUpdate::FileUpdated`; the chat
  updates monitor maps it to `ChatUpdate::FileUpdated`, which carries
  `downloaded_size`.
- The orchestrator's `file_updates` module feeds the progress of the downloads
  RTG started (`Downloads::active_downloads`) into `DownloadSpeedMeter` in
  `ShellState`, and stops tracking a file when it completes or is cancelled.
- The first report of each file only sets its baseline, so bytes downloaded
  earlier (a resumed download) do not inflate the speed.

`updateFileDownloads` is not used: it covers only TDLib's download list
(`addFileToDownloads`), which RTG does not use.
