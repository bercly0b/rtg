//! Reads the metadata of a video, an audio file or an animation with
//! `ffprobe`, the tool that comes with `ffmpeg`.

use std::{
    io::ErrorKind,
    process::{Command, Stdio},
};

use crate::domain::attachment::MediaMetadata;

/// `ffprobe` is not installed: the one probe failure the user can fix.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProbeToolMissing;

/// Reads the metadata of the file at `path`. A file `ffprobe` cannot make
/// sense of has no metadata rather than an error: nothing can be done
/// about it.
pub fn probe_media(path: &str) -> Result<MediaMetadata, ProbeToolMissing> {
    probe_with("ffprobe", path)
}

fn probe_with(program: &str, path: &str) -> Result<MediaMetadata, ProbeToolMissing> {
    let output = Command::new(program)
        .args([
            "-v",
            "error",
            "-show_entries",
            "stream=width,height:stream_tags:stream_side_data=rotation:format=duration:format_tags",
            "-of",
            "default=noprint_wrappers=1",
            "-i",
            path,
        ])
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output();

    match output {
        Ok(output) => Ok(parse_probe_output(&String::from_utf8_lossy(&output.stdout))),
        Err(error) if error.kind() == ErrorKind::NotFound => Err(ProbeToolMissing),
        Err(error) => {
            tracing::warn!(error = %error, "failed to run ffprobe");
            Ok(MediaMetadata::default())
        }
    }
}

/// Parses the `key=value` lines `ffprobe` prints. The first value of a key
/// wins: streams come before the container, and the video stream decides
/// the frame size.
fn parse_probe_output(output: &str) -> MediaMetadata {
    let mut metadata = MediaMetadata::default();
    let mut rotation = 0;

    for line in output.lines() {
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        match key.to_ascii_lowercase().as_str() {
            "width" if metadata.width == 0 => metadata.width = value.parse().unwrap_or(0),
            "height" if metadata.height == 0 => metadata.height = value.parse().unwrap_or(0),
            "duration" if metadata.duration_secs == 0 => {
                metadata.duration_secs = value.parse::<f64>().map_or(0, |secs| secs.round() as i32);
            }
            // Newer ffprobe reports the display matrix, older ones the tag.
            "rotation" | "tag:rotate" if rotation == 0 => rotation = value.parse().unwrap_or(0),
            "tag:title" if metadata.title.is_empty() => metadata.title = value.to_owned(),
            "tag:artist" if metadata.performer.is_empty() => {
                metadata.performer = value.to_owned();
            }
            _ => {}
        }
    }

    // A phone stores a portrait video as a landscape frame plus a rotation.
    if rotation % 180 != 0 {
        std::mem::swap(&mut metadata.width, &mut metadata.height);
    }
    metadata
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn video_output_gives_duration_and_frame_size() {
        let output = "width=1920\nheight=1080\nTAG:language=und\nTAG:handler_name=VideoHandler\n\
                      TAG:language=und\nTAG:handler_name=SoundHandler\nduration=12.400000\n\
                      TAG:major_brand=isom\n";

        assert_eq!(
            parse_probe_output(output),
            MediaMetadata {
                duration_secs: 12,
                width: 1920,
                height: 1080,
                ..MediaMetadata::default()
            }
        );
    }

    #[test]
    fn audio_output_gives_duration_title_and_performer() {
        let output = "duration=183.600000\nTAG:title=Song\nTAG:artist=Band\nTAG:encoder=Lavf63\n";

        assert_eq!(
            parse_probe_output(output),
            MediaMetadata {
                duration_secs: 184,
                title: "Song".to_owned(),
                performer: "Band".to_owned(),
                ..MediaMetadata::default()
            }
        );
    }

    #[test]
    fn tags_are_found_on_a_stream_and_in_any_case() {
        let output =
            "TAG:encoder=Lavc63 flac\nTAG:TITLE=Song\nTAG:ARTIST=Band\nduration=3.000000\n";

        let metadata = parse_probe_output(output);

        assert_eq!(metadata.title, "Song");
        assert_eq!(metadata.performer, "Band");
    }

    #[test]
    fn a_title_may_hold_an_equals_sign() {
        assert_eq!(parse_probe_output("TAG:title=1+1=2\n").title, "1+1=2");
    }

    #[test]
    fn a_rotated_video_swaps_width_and_height() {
        let rotated = MediaMetadata {
            duration_secs: 2,
            width: 1080,
            height: 1920,
            ..MediaMetadata::default()
        };

        assert_eq!(
            parse_probe_output("width=1920\nheight=1080\nrotation=-90\nduration=2.000000\n"),
            rotated
        );
        assert_eq!(
            parse_probe_output("width=1920\nheight=1080\nTAG:rotate=90\nduration=2.000000\n"),
            rotated
        );
    }

    #[test]
    fn a_video_turned_upside_down_keeps_its_frame_size() {
        let metadata = parse_probe_output("width=1920\nheight=1080\nrotation=180\n");

        assert_eq!((metadata.width, metadata.height), (1920, 1080));
    }

    #[test]
    fn unreadable_output_gives_no_metadata() {
        assert_eq!(parse_probe_output(""), MediaMetadata::default());
        assert_eq!(
            parse_probe_output("width=N/A\nduration=N/A\n"),
            MediaMetadata::default()
        );
    }

    #[test]
    fn a_missing_tool_is_told_apart_from_an_unreadable_file() {
        assert_eq!(
            probe_with("rtg-no-such-ffprobe", "/d/clip.mp4"),
            Err(ProbeToolMissing)
        );
    }
}
