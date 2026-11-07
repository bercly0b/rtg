//! `mise run release` moves the changelog fragments from `changelog.d/` into a
//! new `CHANGELOG.md` section, which cargo-dist then puts into the GitHub
//! Release body. cargo-dist silently skips a version without a section, so a
//! release without fragments must fail instead.

use std::fs;
use std::path::Path;
use std::process::{Command, Output};

const SCRIPT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/scripts/release-changelog.sh");

const HEADER: &str = "# Changelog\n\nIntro.\n";

fn release(dir: &Path) -> Output {
    Command::new(SCRIPT)
        .args(["0.2.0", "2026-11-07"])
        .current_dir(dir)
        .output()
        .expect("failed to run the release changelog script")
}

fn repo_with(changelog: &str, fragments: &[(&str, &str)]) -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("failed to create a temp dir");
    fs::write(dir.path().join("CHANGELOG.md"), changelog).unwrap();
    fs::create_dir(dir.path().join("changelog.d")).unwrap();
    fs::write(dir.path().join("changelog.d/.gitkeep"), "").unwrap();
    for (name, content) in fragments {
        fs::write(dir.path().join("changelog.d").join(name), content).unwrap();
    }
    dir
}

fn changelog(dir: &Path) -> String {
    fs::read_to_string(dir.join("CHANGELOG.md")).unwrap()
}

fn fragment_dir_entries(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(dir.join("changelog.d"))
        .unwrap()
        .map(|entry| entry.unwrap().file_name().into_string().unwrap())
        .collect();
    names.sort();
    names
}

#[test]
fn first_release_adds_the_section_after_the_header() {
    let dir = repo_with(
        HEADER,
        &[
            ("roz-2.md", "- Second entry.\n"),
            ("roz-1.md", "- First entry.\n- Another first entry.\n"),
        ],
    );

    let output = release(dir.path());

    assert!(output.status.success(), "{output:?}");
    assert_eq!(
        changelog(dir.path()),
        "# Changelog\n\nIntro.\n\n## 0.2.0 - 2026-11-07\n\n\
         - First entry.\n- Another first entry.\n- Second entry.\n"
    );
}

#[test]
fn new_section_goes_above_previous_releases() {
    let dir = repo_with(
        "# Changelog\n\nIntro.\n\n## 0.1.0 - 2026-11-01\n\n- Old entry.\n",
        &[("roz-1.md", "- New entry.\n")],
    );

    let output = release(dir.path());

    assert!(output.status.success(), "{output:?}");
    assert_eq!(
        changelog(dir.path()),
        "# Changelog\n\nIntro.\n\n## 0.2.0 - 2026-11-07\n\n- New entry.\n\n\
         ## 0.1.0 - 2026-11-01\n\n- Old entry.\n"
    );
}

#[test]
fn fragments_are_removed_after_release() {
    let dir = repo_with(HEADER, &[("roz-1.md", "- Entry.\n")]);

    let output = release(dir.path());

    assert!(output.status.success(), "{output:?}");
    assert_eq!(fragment_dir_entries(dir.path()), [".gitkeep"]);
}

#[test]
fn fragment_without_trailing_newline_or_with_blank_lines_stays_a_clean_list() {
    let dir = repo_with(
        HEADER,
        &[
            ("roz-1.md", "- First entry."),
            ("roz-2.md", "\n- Second entry.\n\n"),
        ],
    );

    let output = release(dir.path());

    assert!(output.status.success(), "{output:?}");
    assert_eq!(
        changelog(dir.path()),
        "# Changelog\n\nIntro.\n\n## 0.2.0 - 2026-11-07\n\n- First entry.\n- Second entry.\n"
    );
}

#[test]
fn release_without_fragments_fails_and_leaves_the_changelog_untouched() {
    let dir = repo_with(HEADER, &[]);

    let output = release(dir.path());

    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("no changelog fragments"),
        "{output:?}"
    );
    assert_eq!(changelog(dir.path()), HEADER);
}
