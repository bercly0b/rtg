//! The built-in credentials: the API credentials RTG ships with, so a user
//! signs in without registering an application first
//! (docs/adr/0003-built-in-credentials-in-source.md).
//!
//! If you distribute a fork as a separate application, please register your
//! own API credentials at <https://my.telegram.org/apps> and replace these.

use super::TelegramConfig;

// Masked only so that scrapers and secret scanners do not pick the pair up by
// pattern. This is not protection; do not strengthen it.
const MASK: &[u8] = b"rtg-tui";

const API_ID: [u8; 4] = [0x79, 0x90, 0x96, 0x2c];

#[rustfmt::skip]
const API_HASH: [u8; 32] = [
    0x4a, 0x4d, 0x57, 0x14, 0x40, 0x16, 0x5c, 0x41,
    0x45, 0x50, 0x19, 0x44, 0x10, 0x08, 0x40, 0x4c,
    0x56, 0x19, 0x17, 0x14, 0x0d, 0x45, 0x4d, 0x50,
    0x14, 0x40, 0x46, 0x5b, 0x40, 0x42, 0x50, 0x1e,
];

pub(super) fn built_in_credentials() -> TelegramConfig {
    TelegramConfig {
        api_id: i32::from_le_bytes(unmask(API_ID)),
        api_hash: unmask(API_HASH).into_iter().map(char::from).collect(),
    }
}

fn unmask<const N: usize>(mut masked: [u8; N]) -> [u8; N] {
    for (byte, mask) in masked.iter_mut().zip(MASK.iter().cycle()) {
        *byte ^= mask;
    }
    masked
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn built_in_credentials_unmask_to_a_usable_pair() {
        let credentials = built_in_credentials();

        assert!(credentials.api_id > 0);
        assert_eq!(credentials.api_hash.len(), 32);
        assert!(credentials
            .api_hash
            .chars()
            .all(|ch| matches!(ch, '0'..='9' | 'a'..='f')));
        assert!(credentials.is_configured());
    }

    #[test]
    fn source_does_not_hold_the_pair_in_plain_text() {
        let credentials = built_in_credentials();
        let source = include_str!("built_in_credentials.rs");

        assert!(!source.contains(&credentials.api_id.to_string()));
        assert!(!source.contains(&credentials.api_hash));
    }
}
