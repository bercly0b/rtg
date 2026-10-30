use crate::usecases::background::BackgroundError;

#[test]
fn background_error_displays_its_code() {
    let error = BackgroundError::new("CHAT_LIST_UNAVAILABLE");

    assert_eq!(error.to_string(), "CHAT_LIST_UNAVAILABLE");
}

#[test]
fn background_error_converts_into_error_chains() {
    let error = anyhow::Error::from(BackgroundError::new("THREAD_SPAWN_FAILED"));

    assert_eq!(
        error.downcast_ref::<BackgroundError>(),
        Some(&BackgroundError::new("THREAD_SPAWN_FAILED"))
    );
}
