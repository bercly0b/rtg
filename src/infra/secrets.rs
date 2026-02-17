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
}
