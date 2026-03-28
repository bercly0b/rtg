use super::*;

// ── extract_first_url tests ──

#[test]
fn extract_first_url_returns_none_when_no_url() {
    assert_eq!(extract_first_url("hello world", &[]), None);
}

#[test]
fn extract_first_url_finds_https() {
    assert_eq!(
        extract_first_url("visit https://example.com please", &[]),
        Some("https://example.com".to_owned())
    );
}

#[test]
fn extract_first_url_finds_http() {
    assert_eq!(
        extract_first_url("go to http://example.com", &[]),
        Some("http://example.com".to_owned())
    );
}

#[test]
fn extract_first_url_returns_first_when_multiple() {
    assert_eq!(
        extract_first_url("see https://first.com and https://second.com", &[]),
        Some("https://first.com".to_owned())
    );
}

#[test]
fn extract_first_url_handles_url_at_start() {
    assert_eq!(
        extract_first_url("https://start.com is the link", &[]),
        Some("https://start.com".to_owned())
    );
}

#[test]
fn extract_first_url_handles_url_at_end() {
    assert_eq!(
        extract_first_url("link: https://end.com", &[]),
        Some("https://end.com".to_owned())
    );
}

#[test]
fn extract_first_url_returns_none_for_empty_string() {
    assert_eq!(extract_first_url("", &[]), None);
}

#[test]
fn extract_first_url_ignores_non_http_schemes() {
    assert_eq!(extract_first_url("check ftp://files.com out", &[]), None);
}

#[test]
fn extract_first_url_prefers_entity_link() {
    let links = vec![TextLink {
        offset: 0,
        length: 9,
        url: "https://hidden.com".to_owned(),
    }];
    assert_eq!(
        extract_first_url("click here and https://visible.com", &links),
        Some("https://hidden.com".to_owned())
    );
}

#[test]
fn extract_first_url_falls_back_to_text_when_no_entities() {
    assert_eq!(
        extract_first_url("go to https://example.com", &[]),
        Some("https://example.com".to_owned())
    );
}

#[test]
fn extract_first_url_prepends_scheme_to_bare_host() {
    let links = vec![TextLink {
        offset: 0,
        length: 18,
        url: "127.0.0.1:18789".to_owned(),
    }];
    assert_eq!(
        extract_first_url("127.0.0.1:18789", &links),
        Some("http://127.0.0.1:18789".to_owned())
    );
}

#[test]
fn extract_first_url_preserves_existing_scheme() {
    let links = vec![TextLink {
        offset: 0,
        length: 22,
        url: "https://example.com".to_owned(),
    }];
    assert_eq!(
        extract_first_url("https://example.com", &links),
        Some("https://example.com".to_owned())
    );
}
