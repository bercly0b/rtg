use super::*;
use crate::{domain::update_check::SelfUpdateProgress, usecases::self_update::SelfUpdated};

fn press_u(o: &mut TestOrchestrator) {
    o.handle_event(AppEvent::InputKey(KeyInput::new("U", false)))
        .unwrap();
}

fn finish_self_update(o: &mut TestOrchestrator, result: Result<SelfUpdated, String>) {
    o.handle_event(AppEvent::BackgroundTasksCompleted(vec![
        BackgroundTaskResult::SelfUpdateFinished(result),
    ]))
    .unwrap();
}

#[test]
fn u_in_the_chat_list_starts_the_self_update_in_the_background() {
    let mut o = make_orchestrator();

    press_u(&mut o);

    assert_eq!(o.dispatcher.self_update_dispatch_count(), 1);
    assert_eq!(o.state().self_update(), Some(&SelfUpdateProgress::Running));
}

#[test]
fn u_in_an_open_chat_starts_the_self_update() {
    let mut o = orchestrator_with_open_chat(vec![chat(1, "Alice")], 1, vec![message(1, "Hi")]);

    press_u(&mut o);

    assert_eq!(o.dispatcher.self_update_dispatch_count(), 1);
}

#[test]
fn u_in_a_forum_topic_list_starts_the_self_update() {
    let mut o = orchestrator_with_chats(vec![forum_chat(100, "Topics")]);
    o.handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
        .unwrap();

    press_u(&mut o);

    assert_eq!(o.dispatcher.self_update_dispatch_count(), 1);
}

#[test]
fn u_while_the_self_update_runs_does_not_start_another() {
    let mut o = make_orchestrator();

    press_u(&mut o);
    press_u(&mut o);

    assert_eq!(o.dispatcher.self_update_dispatch_count(), 1);
}

#[test]
fn installed_release_asks_for_a_restart_and_is_not_installed_again() {
    let mut o = make_orchestrator();
    press_u(&mut o);

    finish_self_update(
        &mut o,
        Ok(SelfUpdated::To {
            version: "0.2.0".to_owned(),
        }),
    );
    press_u(&mut o);

    assert_eq!(
        o.state().self_update(),
        Some(&SelfUpdateProgress::Installed {
            version: "0.2.0".to_owned()
        })
    );
    assert_eq!(o.dispatcher.self_update_dispatch_count(), 1);
}

#[test]
fn latest_release_already_running_is_reported_in_a_notification() {
    let mut o = make_orchestrator();
    press_u(&mut o);

    finish_self_update(&mut o, Ok(SelfUpdated::AlreadyLatest));

    assert_eq!(o.state().self_update(), None);
    assert_eq!(
        o.state().active_notification(),
        Some(concat!(
            "rtg ",
            env!("CARGO_PKG_VERSION"),
            " is the latest release"
        ))
    );
}

#[test]
fn update_that_installed_nothing_shows_why_and_can_be_retried() {
    let mut o = make_orchestrator();
    press_u(&mut o);

    finish_self_update(&mut o, Err("Update with: brew upgrade rtg".to_owned()));

    assert_eq!(o.state().self_update(), None);
    assert_eq!(
        o.state().active_notification(),
        Some("Update with: brew upgrade rtg")
    );

    press_u(&mut o);
    assert_eq!(o.dispatcher.self_update_dispatch_count(), 2);
}
