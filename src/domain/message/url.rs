/// What a [`TextLink`] opens.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LinkTarget {
    /// An address that opens in the browser.
    Url(String),
    /// A username, without the leading `@`, whose chat opens in RTG.
    Mention(String),
}

impl LinkTarget {
    pub fn label(&self) -> String {
        match self {
            Self::Url(url) => url.clone(),
            Self::Mention(username) => format!("@{username}"),
        }
    }
}

/// A link or a mention embedded in message text via a text entity.
///
/// Represents `TextEntityTypeUrl` (URL visible in text), `TextEntityTypeTextUrl`
/// (clickable text with a hidden URL) and `TextEntityTypeMention` (`@username`).
/// Offsets are **byte** offsets into `Message::text`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextLink {
    /// Byte offset of the link text start in `Message::text`.
    pub offset: usize,
    /// Byte length of the link text in `Message::text`.
    pub length: usize,
    pub target: LinkTarget,
}

/// Lists what can be opened from a message: its links and mentions in text
/// order, each one once.
///
/// A message without text entities falls back to the first whitespace-delimited
/// `http(s)://` word of its text.
/// URLs without a scheme get `http://` prepended so they can be opened by the OS.
pub fn open_targets(text: &str, links: &[TextLink]) -> Vec<LinkTarget> {
    let mut targets: Vec<LinkTarget> = Vec::new();
    for link in links {
        let target = match &link.target {
            LinkTarget::Url(url) => LinkTarget::Url(normalize_url(url)),
            mention @ LinkTarget::Mention(_) => mention.clone(),
        };
        if !targets.contains(&target) {
            targets.push(target);
        }
    }
    if targets.is_empty() {
        targets.extend(
            text.split_whitespace()
                .find(|word| word.starts_with("https://") || word.starts_with("http://"))
                .map(|url| LinkTarget::Url(url.to_owned())),
        );
    }
    targets
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
