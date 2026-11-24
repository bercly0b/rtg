# Sending attachments

An attachment is a local file the user sends into a chat; the file picker is
the popup that chooses it, and the file list command is the external command
that names the files the picker shows (see "Attachments" in
[GLOSSARY.md](../../GLOSSARY.md)).

## Opening the file picker

`A` in the messages panel (rebindable as `attach_file`) opens the file picker
for the open chat or forum topic. It does not open from the message input:
leave it with `Esc` first. In a channel it says "Channel is read-only", in a
closed topic "Topic is closed".

## The file list command

The picker does not walk the file system itself. It runs `[attach] list_cmd`
and reads its output:

- **contract** — the command prints one path per line and nothing else. Any
  tool that lists files fits: `find`, `fd`, `rg --files`, `git ls-files`, a
  script;
- **metadata** — RTG finds out the size and the modification time of each
  path itself, and silently drops lines that are not existing regular files.
  A relative path is taken from the directory RTG runs in;
- **no shell** — the template is split with shell quoting rules and executed
  directly, like `[voice] record_cmd`;
- **`{dirs}`** — an argument of its own, replaced with the directories of
  `[attach] dirs`, one argument each. `~` is the home directory and directories that do not exist
  are left out. A command may leave `{dirs}` out;
- **defaults** — `find {dirs} -type f -not -path '*/.*'` over `~/Desktop`,
  `~/Downloads` and `~/Documents`: the whole home directory would take
  `find` far too long.

The command runs anew every time the picker opens, so that a screenshot taken
a second ago is there; nothing is cached. The picker opens at once and fills
while the command runs. Closing the picker kills the command.

When there is nothing to show, the picker stays open and says why:

| Situation | Text |
|-----------|------|
| The command is still running | `Listing files...` |
| The command named no files | `No files found` |
| The query filters everything out | `No files match` |
| The command is not installed | `command not found: <name>` |
| None of `[attach] dirs` exist | `None of the [attach] dirs exist` |
| The command failed without naming a file | the first line it wrote to stderr |

There is no fallback to `find`: it would hide a typo in the config. A command
that fails after naming files (say, `find` hitting a directory it may not
read) still shows those files.

## The popup

- A flat list, no directory navigation, newest file first whatever order the
  command names them in.
- Columns: path (the home directory shortened to `~`), size, modification
  time as `Today 14:32`, `12 Nov` or `2025-03-01`. A path that does not fit is
  cut from the left, so the file name stays visible. Creation time is not
  shown: not every Linux file system has it.
- Typing filters as you type: a file stays when its path, as shown, contains
  every word of the query, ignoring case (`desk png` finds
  `~/Desktop/shot.png`). The home directory behind `~` does not take part.
- A file the command names twice is listed once.
- The selection starts on the newest file and stays there while files keep
  arriving; once moved, it follows the chosen file.

| Key | Action |
|-----|--------|
| `Ctrl+J` / `Down` | next file |
| `Ctrl+K` / `Up` | previous file |
| `Ctrl+L` | open the selected file, see below |
| `Enter` | send the selected file |
| `Backspace` | delete the last character of the query |
| `Esc` | close |

Plain letters always go into the query, which is why navigation is on `Ctrl`.

## Looking at a file before sending

`Ctrl+L` opens the selected file without closing the picker; the selection and
the query stay as they were.

- The file opens with the system opener (`open` / `xdg-open`), which
  launches in the background.
- The `[open]` handlers are not used: they are chosen by MIME type, and a
  local file does not come with one.
- When the system opener fails, the status bar says why.

## Sending

`Enter` sends the selected file at once, without a confirmation: a wrong file
is deleted with `dd`.

- **Attachment kind** — the file name decides whether the file goes as a
  video, an audio file or an animation, so that it plays in the recipient's
  client:

  | Kind | Extensions |
  |------|------------|
  | Video | `mp4`, `mov`, `m4v` |
  | Audio | `mp3`, `m4a`, `flac`, `ogg`, `oga`, `opus`, `wav`, `aac` |
  | Animation | `gif` |

  The case of the extension does not matter. Other video formats (`mkv`,
  `webm`, `avi`) stay documents: not every Telegram client plays them. An
  `.ogg` file is an audio file, not a voice note.
- **Photo or document** — of the remaining files a JPEG, PNG or WebP image is
  sent as a compressed photo. Everything else is sent as a document, including
  an image outside Telegram's photo limits (over 10 MB, sides over 10000 px in
  total, or a side ratio over 20), such as a very long screenshot. A photo is
  told by the file content, not by its name; the other kinds are told by the
  name so that they do not depend on `ffprobe` being installed.
- **Metadata** — a video, an audio file and an animation are sent with their
  duration; a video and an animation also with their frame size, an audio
  file with the title and the artist from its tags. RTG reads them with
  `ffprobe`, which comes with `ffmpeg`, while the message already shows in
  the chat.
- **Without `ffprobe`** — the file is still sent as its kind, without the
  metadata, and the status bar says
  "ffprobe not found: sent without duration and size". Some clients then
  show no duration and a wrong frame shape until the file is played. A file
  `ffprobe` cannot read is sent the same way, silently.
- **Caption** — the text in the message input becomes the caption and the
  input is cleared. An empty input sends no caption.
- **Topic** — the attachment goes into the open forum topic.
- **Reply** — a pending reply target is passed along like for a text message,
  but leaving the message input drops it and the picker does not open from the
  input, so an attachment cannot be sent as a reply yet.
- **Optimistic UI** — the message appears in the chat at once as being sent;
  Telegram uploads the file in the background.
- **Upload progress** — while the file uploads, its metadata says how much of
  it is there: `[Document] type=pdf, sending=42%, size=1.2MB, name=report.pdf`.
  Once the upload is over `sending=` goes away, and the `sending...` status
  with it as soon as Telegram accepts the message. A message being sent does
  not say `download=yes`: the file is the user's own. `dd` on such a message
  cancels the send.
- **Failure** — the pending message is removed, the caption goes back into
  the input (also when another chat is open by then), and the status bar says
  "Failed to send the file".
- **Size limit** — a file over 2000 MB (4000 MB with Telegram Premium) is not
  sent: "File is too large to send".

## TDLib integration

- A photo is `inputMessagePhoto` with the width and height read from the
  image header; a document is `inputMessageDocument` with content type
  detection left on. A video is `inputMessageVideo`, an audio file
  `inputMessageAudio`, an animation `inputMessageAnimation`; TDLib takes 0
  and an empty string for a value that is not known. All go through
  `sendMessage` with `inputFileLocal`, the same call as text and voice notes.
- TDLib does not read the metadata of a local file itself, so it comes from
  one `ffprobe` call in the background task, before `sendMessage`. A video
  stored with a rotation (a portrait video from a phone) gets its width and
  height swapped.
- No thumbnail is passed, and `supports_streaming` is off: streaming needs
  the index at the start of the file, which RTG does not check.
- `sendMessage` returns as soon as TDLib has queued the message; TDLib
  uploads the file itself. The refresh after a send replaces the optimistic
  message with TDLib's own pending message.
- The photo limits are the ones TDLib documents for `inputMessagePhoto`.
- A message TDLib has queued and not sent yet has
  `sendingState = messageSendingStatePending`; it is shown as being sent.
- Upload progress comes from `updateFile`, the update that also reports
  downloads: `remote.uploaded_size` out of the file size while
  `remote.is_uploading_active`. It is applied to the messages being sent
  whose file has that id.
- TDLib has no option for the upload size limit, so it is derived from the
  `is_premium` option.

## Not covered

One file at a time, no choice of the attachment kind (a video cannot be sent
as a document), no thumbnails or album covers, no voice or video notes from
the picker, no path input in the picker, no pasting from the clipboard,
substring search only. Each is a separate issue.
