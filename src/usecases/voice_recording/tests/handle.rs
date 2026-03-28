use super::super::*;

#[test]
fn try_exit_success_returns_true_for_zero_exit_code() {
    let child = std::process::Command::new("true").spawn().unwrap();
    let mut handle = RecordingHandle::from_child(child);
    std::thread::sleep(std::time::Duration::from_millis(50));
    assert_eq!(handle.try_exit_success(), Some(true));
}

#[test]
fn try_exit_success_returns_false_for_nonzero_exit_code() {
    let child = std::process::Command::new("false").spawn().unwrap();
    let mut handle = RecordingHandle::from_child(child);
    std::thread::sleep(std::time::Duration::from_millis(50));
    assert_eq!(handle.try_exit_success(), Some(false));
}
