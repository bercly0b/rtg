//! What RTG presents itself to Telegram with when the config sets nothing of
//! its own; see ADR 0003 in `docs/adr/`.
//!
//! If you distribute a fork as a separate application, please register it at
//! <https://my.telegram.org/apps> and replace this pair.

use super::TelegramConfig;

// Masked and named plainly only so that scrapers and scanners do not pick the
// pair up by pattern. This is not protection; do not strengthen it.
const MASK: &[u8] = b"rtg-tui";

const ID: [u8; 4] = [0x79, 0x90, 0x96, 0x2c];

#[rustfmt::skip]
const HASH: [u8; 32] = [
    0x4a, 0x4d, 0x57, 0x14, 0x40, 0x16, 0x5c, 0x41,
    0x45, 0x50, 0x19, 0x44, 0x10, 0x08, 0x40, 0x4c,
    0x56, 0x19, 0x17, 0x14, 0x0d, 0x45, 0x4d, 0x50,
    0x14, 0x40, 0x46, 0x5b, 0x40, 0x42, 0x50, 0x1e,
];

pub(super) fn built_in() -> TelegramConfig {
    TelegramConfig {
        api_id: i32::from_le_bytes(unmask(ID)),
        api_hash: unmask(HASH).into_iter().map(char::from).collect(),
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
    fn unmasks_to_a_usable_pair() {
        let pair = built_in();

        assert!(pair.api_id > 0);
        assert_eq!(pair.api_hash.len(), 32);
        assert!(pair
            .api_hash
            .chars()
            .all(|ch| matches!(ch, '0'..='9' | 'a'..='f')));
        assert!(pair.is_configured());
    }

    #[test]
    fn source_does_not_hold_the_pair_in_plain_text() {
        let pair = built_in();
        let source = include_str!("built_in.rs");

        assert!(!source.contains(&pair.api_id.to_string()));
        assert!(!source.contains(&pair.api_hash));
    }
}
