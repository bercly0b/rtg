/// A hyperlink embedded in message text via a text entity.
///
/// Represents both `TextEntityTypeUrl` (URL visible in text) and
/// `TextEntityTypeTextUrl` (clickable text with a hidden URL).
/// Offsets are **byte** offsets into `Message::text`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextLink {
    /// Byte offset of the link text start in `Message::text`.
    pub offset: usize,
    /// Byte length of the link text in `Message::text`.
    pub length: usize,
    /// The target URL to open.
    pub url: String,
}

/// Extracts the first URL from message text and link entities.
///
/// Checks entity links first (they may contain URLs not visible in text),
/// then falls back to whitespace-delimited scanning of plain text.
/// URLs without a scheme get `http://` prepended so they can be opened by the OS.
pub fn extract_first_url(text: &str, links: &[TextLink]) -> Option<String> {
    if let Some(link) = links.first() {
        return Some(normalize_url(&link.url));
    }
    text.split_whitespace()
        .find(|word| word.starts_with("https://") || word.starts_with("http://"))
        .map(|s| s.to_owned())
}

/// Ensures a URL has an `http://` or `https://` scheme.
///
/// TDLib `TextEntityTypeUrl` may match bare hosts like `127.0.0.1:8080`
/// or `example.com/path` — the OS launcher needs a full scheme to work.
fn normalize_url(url: &str) -> String {
    if url.starts_with("http://") || url.starts_with("https://") {
        url.to_owned()
    } else {
        format!("http://{url}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_url_adds_http_to_bare_host() {
        assert_eq!(normalize_url("127.0.0.1:8080"), "http://127.0.0.1:8080");
        assert_eq!(normalize_url("example.com/path"), "http://example.com/path");
    }

    #[test]
    fn normalize_url_keeps_https() {
        assert_eq!(normalize_url("https://example.com"), "https://example.com");
    }

    #[test]
    fn normalize_url_keeps_http() {
        assert_eq!(normalize_url("http://example.com"), "http://example.com");
    }
}
