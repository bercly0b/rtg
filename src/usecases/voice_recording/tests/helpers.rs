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
