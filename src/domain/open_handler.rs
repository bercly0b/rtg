//! MIME-based command resolution for opening message files.
//!
//! Looks up the command template for a given MIME type using configurable
//! handlers (exact match, wildcard), falling back to the platform default
//! opener when no handler matches.

use std::collections::HashMap;

use super::open_defaults::DEFAULT_OPEN;

/// Resolves the command template to open a file of the given MIME type.
///
/// Resolution order:
/// 1. Exact MIME match (e.g. `"audio/ogg"`)
/// 2. Wildcard match (e.g. `"audio/*"`)
/// 3. Platform default opener (`open` on macOS, `xdg-open` on Linux)
///
/// Returns the template string with `{file_path}` placeholder intact.
/// The caller (e.g. `start_command`) is responsible for substitution.
pub fn resolve_open_command<'a>(mime_type: &str, handlers: &'a HashMap<String, String>) -> &'a str {
    // 1. Exact match
    if let Some(cmd) = handlers.get(mime_type) {
        return cmd;
    }

    // 2. Wildcard: take the major type (e.g. "audio") and look for "audio/*"
    if let Some(major) = mime_type.split('/').next() {
        let wildcard = format!("{major}/*");
        if let Some(cmd) = handlers.get(&wildcard) {
            return cmd;
        }
    }

    // 3. Platform default
    DEFAULT_OPEN
}

#[cfg(test)]
mod tests {
    use super::*;

    fn handlers() -> HashMap<String, String> {
        let mut m = HashMap::new();
        m.insert(
            "audio/ogg".to_owned(),
            "mpv --speed=1.5 {file_path}".to_owned(),
        );
        m.insert("audio/*".to_owned(), "mpv {file_path}".to_owned());
        m
    }

    #[test]
    fn exact_match_takes_priority() {
        let h = handlers();
        let cmd = resolve_open_command("audio/ogg", &h);
        assert_eq!(cmd, "mpv --speed=1.5 {file_path}");
    }

    #[test]
    fn wildcard_match_when_no_exact() {
        let h = handlers();
        let cmd = resolve_open_command("audio/mpeg", &h);
        assert_eq!(cmd, "mpv {file_path}");
    }

    #[test]
    fn falls_back_to_default_open_for_unknown_type() {
        let h = handlers();
        let cmd = resolve_open_command("video/mp4", &h);
        assert_eq!(cmd, DEFAULT_OPEN);
    }

    #[test]
    fn falls_back_to_default_open_with_empty_handlers() {
        let h = HashMap::new();
        let cmd = resolve_open_command("audio/ogg", &h);
        assert_eq!(cmd, DEFAULT_OPEN);
    }

    #[test]
    fn exact_mime_key_preserved() {
        let mut h = HashMap::new();
        h.insert("text/plain".to_owned(), "less {file_path}".to_owned());
        let cmd = resolve_open_command("text/plain", &h);
        assert_eq!(cmd, "less {file_path}");
    }

    #[test]
    fn empty_mime_type_uses_default() {
        let h = handlers();
        let cmd = resolve_open_command("", &h);
        assert_eq!(cmd, DEFAULT_OPEN);
    }
}
