use std::future::Future;
use std::panic::{self, AssertUnwindSafe};

use crate::infra::secrets::{panic_payload_text, redact_text};

use super::types::TdLibError;
use super::TdLibClient;

impl TdLibClient {
    /// Runs a TDLib request on the client runtime, contained by
    /// [`guard_request`].
    pub(super) fn block_on_request<T>(
        &self,
        request: &'static str,
        future: impl Future<Output = Result<T, TdLibError>>,
    ) -> Result<T, TdLibError> {
        guard_request(request, || self.rt.block_on(future))
    }
}

/// Turns a panic escaping `tdlib-rs` into a failed request.
///
/// `tdlib-rs` unwraps the deserialization of every response, and it models a
/// field as required unless the TDLib schema documents it with `; may be
/// null`. That marker is missing for fields TDLib genuinely leaves unset —
/// `gift.background` among them — and TDLib omits null object fields from its
/// JSON entirely. Without this guard one chat carrying such a field aborts the
/// process instead of failing the single request that touched it.
pub(super) fn guard_request<T>(
    request: &'static str,
    call: impl FnOnce() -> Result<T, TdLibError>,
) -> Result<T, TdLibError> {
    match panic::catch_unwind(AssertUnwindSafe(call)) {
        Ok(result) => result,
        Err(payload) => {
            let message = redact_text(&panic_payload_text(payload.as_ref()));
            tracing::error!(
                request,
                reason = %message,
                "TDLib request panicked; failing the request instead of aborting"
            );
            Err(TdLibError::Decode { request, message })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn passes_through_a_successful_response() {
        let result = guard_request("getChatHistory", || Ok(7));

        assert!(matches!(result, Ok(7)));
    }

    #[test]
    fn passes_through_a_tdlib_error_unchanged() {
        let result: Result<(), _> = guard_request("getChatHistory", || {
            Err(TdLibError::Request {
                code: 404,
                message: "Chat not found".to_owned(),
            })
        });

        assert!(matches!(result, Err(TdLibError::Request { code: 404, .. })));
    }

    #[test]
    fn converts_a_panic_into_a_decode_error() {
        let result: Result<(), _> = guard_request("getChatHistory", || {
            panic!("missing field `background`");
        });

        let Err(TdLibError::Decode { request, message }) = result else {
            panic!("panic should surface as TdLibError::Decode");
        };
        assert_eq!(request, "getChatHistory");
        assert!(message.contains("missing field"));
    }

    #[test]
    fn redacts_secrets_carried_by_the_panic_payload() {
        let result: Result<(), _> = guard_request("checkAuthenticationPassword", || {
            panic!("rejected password=superSecret99");
        });

        let Err(TdLibError::Decode { message, .. }) = result else {
            panic!("panic should surface as TdLibError::Decode");
        };
        assert!(!message.contains("superSecret99"));
        assert!(message.contains("[REDACTED]"));
    }

    #[test]
    fn keeps_the_process_alive_across_repeated_panics() {
        for _ in 0..3 {
            let result: Result<(), _> = guard_request("getChat", || panic!("boom"));
            assert!(result.is_err());
        }
    }

    /// The real failure unwinds out of a future polled by the client runtime,
    /// which is what `block_on_request` composes — cover that shape, not just a
    /// bare closure.
    #[test]
    fn contains_a_panic_raised_while_polling_on_the_runtime() {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("runtime should build");

        let result: Result<(), _> = guard_request("getChatHistory", || {
            rt.block_on(async { panic!("missing field `background`") })
        });

        assert!(matches!(result, Err(TdLibError::Decode { .. })));
    }
}
