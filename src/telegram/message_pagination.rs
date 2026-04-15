//! Paginated message fetching for TDLib's `getChatHistory`.
//!
//! TDLib documentation states: "For optimal performance, the number of
//! returned messages is chosen by TDLib and can be smaller than the
//! specified limit." This module accumulates results across multiple
//! calls until the requested amount is reached or TDLib has no more.

use std::collections::HashSet;

use crate::usecases::load_messages::MessagesSourceError;

/// Maximum messages per single TDLib `getChatHistory` call.
const TDLIB_PAGE_SIZE: usize = 100;

/// Safety cap on pagination rounds to avoid unbounded loops.
const MAX_PAGINATION_ROUNDS: usize = 5;

/// A single page of raw TDLib messages.
///
/// `messages` must be in reverse chronological order (newest first),
/// matching TDLib's `getChatHistory` return order.
pub struct PageResult<M> {
    pub messages: Vec<M>,
}

/// Fetches messages page by page until `limit` is reached or the chat
/// history is exhausted.
///
/// `fetch_page` is called with `(from_message_id, page_limit)` and must
/// return a batch of messages ordered newest-first (as TDLib does).
/// `message_id` extracts the TDLib message id from a message.
///
/// TDLib's `from_message_id` parameter is inclusive — the message with
/// that ID is included in the result. This function deduplicates across
/// pages to avoid returning the same message twice.
///
/// Note: effective maximum is `TDLIB_PAGE_SIZE * MAX_PAGINATION_ROUNDS`
/// (currently 500). The upstream `LoadMessagesQuery` caps at 200.
///
/// Returns accumulated messages in newest-first order.
pub fn fetch_paginated<M, F, Id>(
    limit: usize,
    initial_from_message_id: i64,
    mut fetch_page: F,
    message_id: Id,
) -> Result<Vec<M>, MessagesSourceError>
where
    F: FnMut(i64, i32) -> Result<PageResult<M>, MessagesSourceError>,
    Id: Fn(&M) -> i64,
{
    let target = limit.min(TDLIB_PAGE_SIZE * MAX_PAGINATION_ROUNDS);
    let mut accumulated: Vec<M> = Vec::with_capacity(target);
    let mut seen_ids: HashSet<i64> = HashSet::with_capacity(target);
    let mut from_message_id: i64 = initial_from_message_id;

    for _round in 0..MAX_PAGINATION_ROUNDS {
        let remaining = target.saturating_sub(accumulated.len());
        if remaining == 0 {
            break;
        }
        let page_limit =
            i32::try_from(remaining.min(TDLIB_PAGE_SIZE)).unwrap_or(TDLIB_PAGE_SIZE as i32);

        let page = fetch_page(from_message_id, page_limit)?;

        if page.messages.is_empty() {
            break; // Chat history exhausted
        }

        // TDLib returns newest-first; the last element is the oldest in this batch.
        let oldest_id = page.messages.last().map(&message_id).unwrap_or(0);

        // TDLib's from_message_id is inclusive — deduplicate across pages.
        for msg in page.messages {
            if seen_ids.insert(message_id(&msg)) {
                accumulated.push(msg);
            }
        }

        from_message_id = oldest_id;
    }

    Ok(accumulated)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Minimal message stub for testing pagination.
    #[derive(Debug, Clone, PartialEq)]
    struct FakeMsg {
        id: i64,
    }

    fn fake_msgs(ids: &[i64]) -> Vec<FakeMsg> {
        ids.iter().map(|&id| FakeMsg { id }).collect()
    }

    #[test]
    fn single_page_returns_all_requested() {
        let messages = fake_msgs(&[50, 49, 48, 47, 46]);
        let messages_clone = messages.clone();

        let result = fetch_paginated(
            5,
            0,
            |_from_id, _limit| {
                Ok(PageResult {
                    messages: messages_clone.clone(),
                })
            },
            |m| m.id,
        )
        .unwrap();

        assert_eq!(result.len(), 5);
        assert_eq!(result[0].id, 50); // newest first
        assert_eq!(result[4].id, 46);
    }

    #[test]
    fn empty_chat_returns_empty_vec() {
        let result = fetch_paginated(
            50,
            0,
            |_from_id, _limit| Ok(PageResult { messages: vec![] }),
            |m: &FakeMsg| m.id,
        )
        .unwrap();

        assert!(result.is_empty());
    }

    #[test]
    fn accumulates_multiple_pages_when_first_is_partial() {
        let mut call_count = 0;

        let result = fetch_paginated(
            5,
            0,
            |from_id, _limit| {
                call_count += 1;
                match call_count {
                    1 => {
                        assert_eq!(from_id, 0, "first call should use from_message_id=0");
                        Ok(PageResult {
                            messages: fake_msgs(&[50, 49]),
                        })
                    }
                    2 => {
                        assert_eq!(
                            from_id, 49,
                            "second call should anchor at oldest from page 1"
                        );
                        Ok(PageResult {
                            messages: fake_msgs(&[48, 47, 46]),
                        })
                    }
                    _ => panic!("unexpected extra call"),
                }
            },
            |m| m.id,
        )
        .unwrap();

        assert_eq!(result.len(), 5);
        assert_eq!(call_count, 2);
        // Order: newest first across pages
        assert_eq!(result[0].id, 50);
        assert_eq!(result[4].id, 46);
    }

    #[test]
    fn deduplicates_when_tdlib_returns_anchor_message() {
        // TDLib's from_message_id is inclusive — the anchor message from
        // the previous page is returned again at the start of the next page.
        let mut call_count = 0;

        let result = fetch_paginated(
            5,
            0,
            |from_id, _limit| {
                call_count += 1;
                match call_count {
                    1 => {
                        assert_eq!(from_id, 0);
                        Ok(PageResult {
                            messages: fake_msgs(&[50, 49]),
                        })
                    }
                    2 => {
                        assert_eq!(from_id, 49);
                        // TDLib includes the anchor message (49) again
                        Ok(PageResult {
                            messages: fake_msgs(&[49, 48, 47, 46]),
                        })
                    }
                    _ => panic!("unexpected extra call"),
                }
            },
            |m| m.id,
        )
        .unwrap();

        // Should have 5 unique messages, not 6
        assert_eq!(result.len(), 5);
        let ids: Vec<i64> = result.iter().map(|m| m.id).collect();
        assert_eq!(ids, vec![50, 49, 48, 47, 46]);
    }

    #[test]
    fn stops_when_chat_history_exhausted() {
        let mut call_count = 0;

        let result = fetch_paginated(
            50,
            0,
            |_from_id, _limit| {
                call_count += 1;
                match call_count {
                    1 => Ok(PageResult {
                        messages: fake_msgs(&[3, 2, 1]),
                    }),
                    // TDLib returned partial result on first call; pagination
                    // continues and gets an empty page confirming exhaustion.
                    2 => Ok(PageResult { messages: vec![] }),
                    _ => panic!("should stop after empty page"),
                }
            },
            |m| m.id,
        )
        .unwrap();

        // Only 3 messages exist in the chat
        assert_eq!(result.len(), 3);
        assert_eq!(call_count, 2);
    }

    #[test]
    fn stops_on_empty_second_page() {
        let mut call_count = 0;

        let result = fetch_paginated(
            10,
            0,
            |_from_id, _limit| {
                call_count += 1;
                match call_count {
                    // Return partial result — pagination continues
                    1 => Ok(PageResult {
                        messages: fake_msgs(&[5, 4, 3, 2, 1]),
                    }),
                    // Empty page signals end of chat history
                    2 => Ok(PageResult { messages: vec![] }),
                    _ => panic!("should stop after empty page"),
                }
            },
            |m| m.id,
        )
        .unwrap();

        assert_eq!(result.len(), 5);
        assert_eq!(call_count, 2);
    }

    #[test]
    fn respects_max_pagination_rounds() {
        let mut call_count = 0;

        let result = fetch_paginated(
            10_000, // Way more than available
            0,
            |_from_id, limit| {
                call_count += 1;
                // Always return exactly `limit` to keep pagination going
                let start_id = (call_count * 1000) as i64;
                let ids: Vec<i64> = (0..limit as i64).map(|i| start_id - i).collect();
                Ok(PageResult {
                    messages: fake_msgs(&ids),
                })
            },
            |m| m.id,
        )
        .unwrap();

        // Should stop at MAX_PAGINATION_ROUNDS (5) * TDLIB_PAGE_SIZE (100) = 500
        assert_eq!(call_count, MAX_PAGINATION_ROUNDS);
        assert_eq!(result.len(), 500);
    }

    #[test]
    fn propagates_fetch_error() {
        let result = fetch_paginated(
            50,
            0,
            |_from_id, _limit| -> Result<PageResult<FakeMsg>, MessagesSourceError> {
                Err(MessagesSourceError::Unavailable)
            },
            |m: &FakeMsg| m.id,
        );

        assert_eq!(result.unwrap_err(), MessagesSourceError::Unavailable);
    }

    #[test]
    fn error_on_second_page_returns_error_not_partial() {
        let mut call_count = 0;

        let result = fetch_paginated(
            10,
            0,
            |_from_id, _limit| {
                call_count += 1;
                match call_count {
                    1 => {
                        // Return partial result so pagination continues
                        Ok(PageResult {
                            messages: fake_msgs(&[10, 9, 8]),
                        })
                    }
                    2 => Err(MessagesSourceError::Unavailable),
                    _ => panic!("should not continue after error"),
                }
            },
            |m: &FakeMsg| m.id,
        );

        assert!(result.is_err());
    }

    #[test]
    fn uses_initial_from_message_id() {
        let mut call_count = 0;

        let result = fetch_paginated(
            5,
            42,
            |from_id, _limit| {
                call_count += 1;
                match call_count {
                    1 => {
                        assert_eq!(from_id, 42, "first call should use initial from_message_id");
                        Ok(PageResult {
                            messages: fake_msgs(&[41, 40, 39, 38, 37]),
                        })
                    }
                    _ => panic!("unexpected extra call"),
                }
            },
            |m| m.id,
        )
        .unwrap();

        assert_eq!(result.len(), 5);
        assert_eq!(result[0].id, 41);
    }

    #[test]
    fn limit_zero_returns_empty() {
        let result = fetch_paginated(
            0,
            0,
            |_from_id, _limit| -> Result<PageResult<FakeMsg>, MessagesSourceError> {
                panic!("should not fetch when limit is 0");
            },
            |m: &FakeMsg| m.id,
        )
        .unwrap();

        assert!(result.is_empty());
    }
}
