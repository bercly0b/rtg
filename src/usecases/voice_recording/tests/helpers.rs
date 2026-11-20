use super::super::*;

#[test]
fn generate_voice_file_path_has_oga_extension() {
    let path = generate_voice_file_path();
    assert!(path.ends_with(".oga"));
}

#[test]
fn generate_voice_file_path_contains_voice_prefix() {
    let path = generate_voice_file_path();
    assert!(path.contains("voice-"));
}

#[test]
fn generate_voice_file_path_is_stamped_with_date_and_time() {
    let path = generate_voice_file_path();

    let stamp = path
        .rsplit_once("/voice-")
        .and_then(|(_, name)| name.strip_suffix(".oga"))
        .unwrap();
    let (date, time) = stamp.split_once('_').unwrap();
    assert_eq!(date.len(), 8, "{stamp}");
    assert_eq!(time.len(), 6, "{stamp}");
    assert!(
        date.chars().chain(time.chars()).all(|c| c.is_ascii_digit()),
        "{stamp}"
    );
}

#[test]
fn default_record_cmd_contains_ffmpeg() {
    assert!(DEFAULT_RECORD_CMD.contains("ffmpeg"));
}

#[test]
fn default_record_cmd_contains_file_path_placeholder() {
    assert!(DEFAULT_RECORD_CMD.contains("{file_path}"));
}

#[test]
fn default_record_cmd_uses_opus_codec() {
    assert!(DEFAULT_RECORD_CMD.contains("libopus"));
}
