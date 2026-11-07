//! `rtg` ships as a single binary (release archive, `cargo install`), so TDLib
//! must be linked in statically rather than loaded from `libtdjson` at runtime.

use std::process::Command;

#[test]
fn rtg_binary_does_not_load_libtdjson_dynamically() {
    let binary = env!("CARGO_BIN_EXE_rtg");
    let output = if cfg!(target_os = "macos") {
        Command::new("otool").arg("-L").arg(binary).output()
    } else {
        Command::new("ldd").arg(binary).output()
    }
    .expect("failed to run the dynamic library lister");

    assert!(
        output.status.success(),
        "listing dynamic libraries failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let libraries = String::from_utf8_lossy(&output.stdout);
    assert!(
        !libraries.contains("tdjson"),
        "rtg depends on libtdjson at runtime:\n{libraries}"
    );
}
