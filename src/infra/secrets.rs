use std::panic;

const REDACTED: &str = "[REDACTED]";

const SENSITIVE_MARKERS: [&str; 7] = [
    "password", "passcode", "2fa", "secret", "token", "otp", "code",
];

pub fn redact_text(input: &str) -> String {
    input
        .split_whitespace()
        .map(redact_chunk)
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn sanitize_error_code(code: &str) -> String {
    let valid = code.starts_with("AUTH_")
        && code.len() <= 64
        && code
            .chars()
            .all(|ch| ch.is_ascii_uppercase() || ch.is_ascii_digit() || ch == '_' || ch == '-');

    if valid {
        code.to_owned()
    } else {
        "AUTH_TRANSIENT".to_owned()
    }
}

pub fn install_panic_redaction_hook() {
    panic::set_hook(Box::new(|panic_info| {
        let payload = panic_info
            .payload()
            .downcast_ref::<&str>()
            .map(ToString::to_string)
            .or_else(|| panic_info.payload().downcast_ref::<String>().cloned())
            .unwrap_or_else(|| "panic payload omitted".to_owned());

        let scrubbed = redact_text(&payload);

        if let Some(location) = panic_info.location() {
            eprintln!(
                "rtg panic: {} at {}:{}:{}",
                scrubbed,
                location.file(),
                location.line(),
                location.column()
            );
        } else {
            eprintln!("rtg panic: {}", scrubbed);
        }
    }));
}

fn redact_chunk(chunk: &str) -> String {
    let lowered = chunk.to_ascii_lowercase();
    if SENSITIVE_MARKERS
        .iter()
        .any(|marker| lowered.contains(marker))
        || looks_like_secret_value(chunk)
    {
        REDACTED.to_owned()
    } else {
        chunk.to_owned()
    }
}

fn looks_like_secret_value(value: &str) -> bool {
    let cleaned = value.trim_matches(|ch: char| !ch.is_ascii_alphanumeric());

    let has_mixed = cleaned.chars().any(|ch| ch.is_ascii_alphabetic())
        && cleaned.chars().any(|ch| ch.is_ascii_digit());

    cleaned.len() >= 6 && (cleaned.chars().all(|ch| ch.is_ascii_digit()) || has_mixed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redact_text_scrubs_sensitive_fragments() {
        let input = "wrong password=superSecret99 token=abc123 code 123456";
        let output = redact_text(input);

        assert!(!output.contains("superSecret99"));
        assert!(!output.contains("abc123"));
        assert!(!output.contains("123456"));
        assert!(output.contains("[REDACTED]"));
    }

    #[test]
    fn sanitize_error_code_rejects_untrusted_text() {
        assert_eq!(sanitize_error_code("AUTH_TIMEOUT"), "AUTH_TIMEOUT");
        assert_eq!(
            sanitize_error_code("AUTH_BACKEND: password=123456"),
            "AUTH_TRANSIENT"
        );
    }

    #[test]
    fn redact_text_preserves_non_sensitive_words() {
        assert_eq!(redact_text("hello world"), "hello world");
    }

    #[test]
    fn redact_text_handles_empty_input() {
        assert_eq!(redact_text(""), "");
    }

    #[test]
    fn redact_text_scrubs_case_insensitive_markers() {
        let output = redact_text("my PASSWORD is leaked");
        assert!(!output.contains("PASSWORD"));
        assert!(output.contains("[REDACTED]"));
    }

    #[test]
    fn redact_text_scrubs_each_sensitive_marker() {
        for marker in &SENSITIVE_MARKERS {
            let input = format!("value is {}", marker);
            let output = redact_text(&input);
            assert!(
                output.contains("[REDACTED]"),
                "marker '{}' should be redacted",
                marker
            );
        }
    }

    #[test]
    fn redact_text_scrubs_long_numeric_strings() {
        let output = redact_text("the value 123456 was sent");
        assert!(!output.contains("123456"));
    }

    #[test]
    fn redact_text_scrubs_mixed_alphanumeric_secrets() {
        let output = redact_text("key is abc123def");
        assert!(!output.contains("abc123def"));
    }

    #[test]
    fn redact_text_preserves_short_numbers() {
        let output = redact_text("step 42 done");
        assert!(
            output.contains("42"),
            "short numbers should not be redacted"
        );
    }

    #[test]
    fn sanitize_error_code_accepts_valid_auth_codes() {
        assert_eq!(sanitize_error_code("AUTH_OK"), "AUTH_OK");
        assert_eq!(
            sanitize_error_code("AUTH_2FA_REQUIRED"),
            "AUTH_2FA_REQUIRED"
        );
        assert_eq!(sanitize_error_code("AUTH_RETRY-1"), "AUTH_RETRY-1");
    }

    #[test]
    fn sanitize_error_code_rejects_non_auth_prefix() {
        assert_eq!(sanitize_error_code("ERROR_TIMEOUT"), "AUTH_TRANSIENT");
    }

    #[test]
    fn sanitize_error_code_rejects_empty_string() {
        assert_eq!(sanitize_error_code(""), "AUTH_TRANSIENT");
    }

    #[test]
    fn sanitize_error_code_rejects_lowercase() {
        assert_eq!(sanitize_error_code("AUTH_timeout"), "AUTH_TRANSIENT");
    }

    #[test]
    fn sanitize_error_code_rejects_too_long_code() {
        let long_code = format!("AUTH_{}", "A".repeat(60));
        assert_eq!(sanitize_error_code(&long_code), "AUTH_TRANSIENT");
    }

    #[test]
    fn sanitize_error_code_accepts_boundary_length() {
        // Exactly 64 chars should be valid
        let code = format!("AUTH_{}", "A".repeat(59));
        assert_eq!(code.len(), 64);
        assert_eq!(sanitize_error_code(&code), code);
    }
}
