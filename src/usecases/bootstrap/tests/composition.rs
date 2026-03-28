use crate::{
    domain::events::{AppEvent, ConnectivityStatus},
    infra::config::{AppConfig, TelegramConfig},
    telegram::TelegramAdapter,
    usecases::{context::AppContext, contracts::AppEventSource},
};

use super::super::{compose_shell, compose_shell_with_factory};

use super::StubConnectivityMonitorFactory;

#[test]
fn composes_shell_dependencies_in_bootstrap_layer() {
    let context = AppContext::new(AppConfig::default(), TelegramAdapter::stub());
    let mut shell = compose_shell(&context);

    assert!(shell.orchestrator.state().is_running());

    shell
        .orchestrator
        .handle_event(AppEvent::QuitRequested)
        .expect("quit event should be handled");

    assert!(!shell.orchestrator.state().is_running());
}

#[test]
fn compose_shell_injects_channel_backed_source_when_telegram_monitor_starts() {
    let mut config = AppConfig::default();
    config.telegram = TelegramConfig {
        api_id: 100,
        api_hash: "configured".to_owned(),
    };
    let context = AppContext::new(config, TelegramAdapter::stub());

    let factory = StubConnectivityMonitorFactory {
        should_fail: false,
        chat_updates_should_fail: false,
    };

    let mut shell = compose_shell_with_factory(&context, &factory);
    let first_event = shell
        .event_source
        .next_event()
        .expect("event should be readable");
    let second_event = shell
        .event_source
        .next_event()
        .expect("second event should be readable");

    let events = [first_event, second_event];
    assert!(events
        .iter()
        .any(|e| matches!(e, Some(AppEvent::ChatUpdateReceived { .. }))));
    assert!(events.contains(&Some(AppEvent::ConnectivityChanged(
        ConnectivityStatus::Connected
    ))));
}

#[test]
fn compose_shell_falls_back_when_telegram_monitor_start_fails() {
    let mut config = AppConfig::default();
    config.telegram = TelegramConfig {
        api_id: 100,
        api_hash: "configured".to_owned(),
    };
    let context = AppContext::new(config, TelegramAdapter::stub());

    let factory = StubConnectivityMonitorFactory {
        should_fail: true,
        chat_updates_should_fail: true,
    };

    let mut shell = compose_shell_with_factory(&context, &factory);
    shell
        .orchestrator
        .handle_event(AppEvent::QuitRequested)
        .expect("fallback composition should still wire orchestrator");

    assert!(!shell.orchestrator.state().is_running());
}

#[test]
fn compose_shell_keeps_chat_updates_when_only_connectivity_monitor_fails() {
    let mut config = AppConfig::default();
    config.telegram = TelegramConfig {
        api_id: 100,
        api_hash: "configured".to_owned(),
    };
    let context = AppContext::new(config, TelegramAdapter::stub());

    let factory = StubConnectivityMonitorFactory {
        should_fail: true,
        chat_updates_should_fail: false,
    };

    let mut shell = compose_shell_with_factory(&context, &factory);
    let event = shell
        .event_source
        .next_event()
        .expect("event should be readable");

    assert!(matches!(event, Some(AppEvent::ChatUpdateReceived { .. })));
}
