//! External command templates (`[voice] record_cmd`, `[open]` handlers).
//!
//! Templates are split with POSIX shell quoting rules, so an argument with
//! spaces can be quoted (e.g. `-i ':MacBook Pro Microphone'`). No shell is
//! spawned: the resulting argv is executed directly.

pub const FILE_PATH_PLACEHOLDER: &str = "{file_path}";

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum CommandTemplateError {
    #[error("empty command")]
    Empty,
    #[error("invalid command template: {0}")]
    Parse(#[from] shell_words::ParseError),
}

/// Splits the template into argv and substitutes `{file_path}` in each argument.
///
/// Substitution happens after splitting so a path containing spaces or quotes
/// always stays a single argument.
pub fn resolve_argv(template: &str, file_path: &str) -> Result<Vec<String>, CommandTemplateError> {
    let argv = shell_words::split(template)?;
    if argv.is_empty() {
        return Err(CommandTemplateError::Empty);
    }

    Ok(argv
        .into_iter()
        .map(|arg| arg.replace(FILE_PATH_PLACEHOLDER, file_path))
        .collect())
}

/// Returns the executable (first argv token) of the template.
pub fn program(template: &str) -> Option<String> {
    shell_words::split(template).ok()?.into_iter().next()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_quoted_device_name_with_spaces_is_one_argument() {
        let argv = resolve_argv(
            "ffmpeg -f avfoundation -i ':MacBook Pro Microphone' {file_path}",
            "/tmp/voice.oga",
        )
        .unwrap();

        assert_eq!(
            argv,
            [
                "ffmpeg",
                "-f",
                "avfoundation",
                "-i",
                ":MacBook Pro Microphone",
                "/tmp/voice.oga"
            ]
        );
    }

    #[test]
    fn double_quoted_and_escaped_arguments_are_unquoted() {
        let argv = resolve_argv(
            r#"rec -t "Built-in Microphone" My\ Mic {file_path}"#,
            "/tmp/v.oga",
        )
        .unwrap();

        assert_eq!(
            argv,
            ["rec", "-t", "Built-in Microphone", "My Mic", "/tmp/v.oga"]
        );
    }

    #[test]
    fn file_path_with_spaces_stays_single_argument() {
        let argv = resolve_argv("mpv {file_path}", "/tmp/my voice note.oga").unwrap();

        assert_eq!(argv, ["mpv", "/tmp/my voice note.oga"]);
    }

    #[test]
    fn file_path_with_quotes_is_substituted_verbatim() {
        let argv = resolve_argv("mpv {file_path}", "/tmp/it's \"here\".oga").unwrap();

        assert_eq!(argv, ["mpv", "/tmp/it's \"here\".oga"]);
    }

    #[test]
    fn quoted_placeholder_is_substituted() {
        let argv = resolve_argv("mpv '{file_path}'", "/tmp/a b.oga").unwrap();

        assert_eq!(argv, ["mpv", "/tmp/a b.oga"]);
    }

    #[test]
    fn placeholder_inside_argument_is_substituted() {
        let argv = resolve_argv("recorder --output={file_path}", "/tmp/v.oga").unwrap();

        assert_eq!(argv, ["recorder", "--output=/tmp/v.oga"]);
    }

    #[test]
    fn extra_whitespace_is_ignored() {
        let argv = resolve_argv("  mpv \t {file_path}  ", "/tmp/v.oga").unwrap();

        assert_eq!(argv, ["mpv", "/tmp/v.oga"]);
    }

    #[test]
    fn empty_template_is_rejected() {
        assert_eq!(
            resolve_argv("", "/tmp/v.oga"),
            Err(CommandTemplateError::Empty)
        );
        assert_eq!(
            resolve_argv("   ", "/tmp/v.oga"),
            Err(CommandTemplateError::Empty)
        );
    }

    #[test]
    fn unbalanced_quote_is_rejected() {
        let error = resolve_argv("ffmpeg -i ':0 {file_path}", "/tmp/v.oga").unwrap_err();

        assert!(matches!(error, CommandTemplateError::Parse(_)));
        assert!(error.to_string().contains("missing closing quote"));
    }

    #[test]
    fn program_is_first_unquoted_token() {
        assert_eq!(
            program("  'xdg-open' {file_path}").as_deref(),
            Some("xdg-open")
        );
    }

    #[test]
    fn program_is_none_for_empty_or_invalid_template() {
        assert_eq!(program(""), None);
        assert_eq!(program("open '{file_path}"), None);
    }
}
