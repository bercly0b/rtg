use super::*;

// ── open_targets tests ──

#[test]
fn open_targets_returns_nothing_when_no_url() {
    assert_eq!(open_targets("hello world", &[]), vec![]);
}

#[test]
fn open_targets_finds_https() {
    assert_eq!(
        open_targets("visit https://example.com please", &[]),
        vec![LinkTarget::Url("https://example.com".to_owned())]
    );
}

#[test]
fn open_targets_finds_http() {
    assert_eq!(
        open_targets("go to http://example.com", &[]),
        vec![LinkTarget::Url("http://example.com".to_owned())]
    );
}

#[test]
fn open_targets_returns_first_when_multiple() {
    assert_eq!(
        open_targets("see https://first.com and https://second.com", &[]),
        vec![LinkTarget::Url("https://first.com".to_owned())]
    );
}

#[test]
fn open_targets_handles_url_at_start() {
    assert_eq!(
        open_targets("https://start.com is the link", &[]),
        vec![LinkTarget::Url("https://start.com".to_owned())]
    );
}

#[test]
fn open_targets_handles_url_at_end() {
    assert_eq!(
        open_targets("link: https://end.com", &[]),
        vec![LinkTarget::Url("https://end.com".to_owned())]
    );
}

#[test]
fn open_targets_returns_nothing_for_empty_string() {
    assert_eq!(open_targets("", &[]), vec![]);
}

#[test]
fn open_targets_ignores_non_http_schemes() {
    assert_eq!(open_targets("check ftp://files.com out", &[]), vec![]);
}

#[test]
fn open_targets_prefers_entity_link() {
    let links = vec![TextLink {
        offset: 0,
        length: 9,
        target: LinkTarget::Url("https://hidden.com".to_owned()),
    }];
    assert_eq!(
        open_targets("click here and https://visible.com", &links),
        vec![LinkTarget::Url("https://hidden.com".to_owned())]
    );
}

#[test]
fn open_targets_falls_back_to_text_when_no_entities() {
    assert_eq!(
        open_targets("go to https://example.com", &[]),
        vec![LinkTarget::Url("https://example.com".to_owned())]
    );
}

#[test]
fn open_targets_prepends_scheme_to_bare_host() {
    let links = vec![TextLink {
        offset: 0,
        length: 18,
        target: LinkTarget::Url("127.0.0.1:18789".to_owned()),
    }];
    assert_eq!(
        open_targets("127.0.0.1:18789", &links),
        vec![LinkTarget::Url("http://127.0.0.1:18789".to_owned())]
    );
}

#[test]
fn open_targets_preserves_existing_scheme() {
    let links = vec![TextLink {
        offset: 0,
        length: 22,
        target: LinkTarget::Url("https://example.com".to_owned()),
    }];
    assert_eq!(
        open_targets("https://example.com", &links),
        vec![LinkTarget::Url("https://example.com".to_owned())]
    );
}

fn link(offset: usize, target: LinkTarget) -> TextLink {
    TextLink {
        offset,
        length: 1,
        target,
    }
}

#[test]
fn open_targets_lists_links_and_mentions_in_text_order() {
    let links = vec![
        link(0, LinkTarget::Mention("alice".to_owned())),
        link(10, LinkTarget::Url("https://example.com".to_owned())),
        link(40, LinkTarget::Mention("bob".to_owned())),
    ];

    assert_eq!(
        open_targets("@alice see https://example.com and ask @bob", &links),
        vec![
            LinkTarget::Mention("alice".to_owned()),
            LinkTarget::Url("https://example.com".to_owned()),
            LinkTarget::Mention("bob".to_owned()),
        ]
    );
}

#[test]
fn open_targets_lists_a_repeated_target_once() {
    let links = vec![
        link(0, LinkTarget::Mention("alice".to_owned())),
        link(7, LinkTarget::Url("example.com".to_owned())),
        link(19, LinkTarget::Mention("alice".to_owned())),
        link(26, LinkTarget::Url("http://example.com".to_owned())),
    ];

    assert_eq!(
        open_targets("@alice example.com @alice http://example.com", &links),
        vec![
            LinkTarget::Mention("alice".to_owned()),
            LinkTarget::Url("http://example.com".to_owned()),
        ]
    );
}

#[test]
fn open_targets_skips_text_fallback_when_message_has_a_mention() {
    let links = vec![link(0, LinkTarget::Mention("alice".to_owned()))];

    assert_eq!(
        open_targets("@alice https://unparsed.com", &links),
        vec![LinkTarget::Mention("alice".to_owned())]
    );
}

#[test]
fn mention_label_shows_the_at_sign() {
    assert_eq!(LinkTarget::Mention("alice".to_owned()).label(), "@alice");
}
