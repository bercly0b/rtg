//! Default constants for the `[attach]` configuration.

/// Directories the file picker searches; `~` stands for the home directory.
pub const DEFAULT_DIRS: [&str; 4] = ["~/Desktop", "~/Downloads", "~/Documents", "~/Pictures"];

/// Default file list command: every file under the directories, except
/// hidden files and whatever lives in hidden directories.
pub const DEFAULT_LIST_CMD: &str = "find {dirs} -type f -not -path '*/.*'";

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::command_template::resolve_list_argv;

    #[test]
    fn default_list_cmd_keeps_the_hidden_path_pattern_as_one_argument() {
        let argv = resolve_list_argv(DEFAULT_LIST_CMD, &["/home/me/Desktop".to_owned()]).unwrap();

        assert_eq!(
            argv,
            [
                "find",
                "/home/me/Desktop",
                "-type",
                "f",
                "-not",
                "-path",
                "*/.*"
            ]
        );
    }
}
