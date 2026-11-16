/// A local file offered by the file picker.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileEntry {
    pub path: String,
    pub size: u64,
    pub modified_unix_s: i64,
}

/// How far the file list command has got.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileListing {
    Running,
    Finished,
    /// The command could not list files; the text says why.
    Failed(String),
}

/// The file picker: the files named by one run of the file list command,
/// newest first, narrowed by the query the user types.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FilePickerState {
    listing_id: u64,
    /// The home directory, shown as `~` in paths.
    home: Option<String>,
    listing: FileListing,
    query: String,
    entries: Vec<FileEntry>,
    /// Indices into `entries` of the files matching `query`.
    visible: Vec<usize>,
    /// Index into `visible`.
    selected: usize,
}

impl FilePickerState {
    /// `listing_id` tells this picker's run of the file list command from
    /// the runs of pickers opened before it; `home` is the home directory.
    pub fn new(listing_id: u64, home: Option<String>) -> Self {
        Self {
            listing_id,
            home,
            listing: FileListing::Running,
            query: String::new(),
            entries: Vec::new(),
            visible: Vec::new(),
            selected: 0,
        }
    }

    pub fn listing_id(&self) -> u64 {
        self.listing_id
    }

    pub fn listing(&self) -> &FileListing {
        &self.listing
    }

    pub fn query(&self) -> &str {
        &self.query
    }

    pub fn visible_len(&self) -> usize {
        self.visible.len()
    }

    /// The `index`-th file matching the query.
    pub fn visible_entry(&self, index: usize) -> Option<&FileEntry> {
        self.visible.get(index).map(|&i| &self.entries[i])
    }

    /// The path as the picker shows it and as the query matches it: with
    /// the home directory shortened to `~`.
    pub fn display_path(&self, entry: &FileEntry) -> String {
        self.home
            .as_deref()
            .and_then(|home| entry.path.strip_prefix(home))
            .filter(|rest| rest.starts_with('/'))
            .map_or_else(|| entry.path.clone(), |rest| format!("~{rest}"))
    }

    pub fn selected_index(&self) -> Option<usize> {
        (!self.visible.is_empty()).then_some(self.selected)
    }

    pub fn selected(&self) -> Option<&FileEntry> {
        self.visible_entry(self.selected)
    }

    /// Adds files the command has named since the last call. The selection
    /// stays on the newest file while the user has not moved it, and follows
    /// the selected file once they have.
    pub fn add_entries(&mut self, entries: Vec<FileEntry>) {
        let followed = (self.selected > 0)
            .then(|| self.selected().map(|entry| entry.path.clone()))
            .flatten();

        self.entries.extend(entries);
        self.entries.sort_by(|a, b| {
            b.modified_unix_s
                .cmp(&a.modified_unix_s)
                .then_with(|| a.path.cmp(&b.path))
        });
        // Overlapping directories make a command name a file twice.
        self.entries.dedup_by(|a, b| a.path == b.path);
        self.refilter();

        self.selected = followed
            .and_then(|path| {
                self.visible
                    .iter()
                    .position(|&i| self.entries[i].path == path)
            })
            .unwrap_or(0);
    }

    /// Records that the command has ended, with `error` when it failed.
    pub fn finish(&mut self, error: Option<String>) {
        self.listing = match error {
            Some(error) => FileListing::Failed(error),
            None => FileListing::Finished,
        };
    }

    pub fn insert_char(&mut self, ch: char) {
        self.query.push(ch);
        self.refilter();
        self.selected = 0;
    }

    pub fn delete_char_before(&mut self) {
        if self.query.pop().is_some() {
            self.refilter();
            self.selected = 0;
        }
    }

    pub fn select_next(&mut self) {
        let last = self.visible.len().saturating_sub(1);
        self.selected = (self.selected + 1).min(last);
    }

    pub fn select_previous(&mut self) {
        self.selected = self.selected.saturating_sub(1);
    }

    /// A file matches when its displayed path contains every word of the
    /// query, ignoring case.
    fn refilter(&mut self) {
        let words: Vec<String> = self
            .query
            .split_whitespace()
            .map(str::to_lowercase)
            .collect();
        self.visible = self
            .entries
            .iter()
            .enumerate()
            .filter(|(_, entry)| {
                let path = self.display_path(entry).to_lowercase();
                words.iter().all(|word| path.contains(word))
            })
            .map(|(i, _)| i)
            .collect();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(path: &str, modified_unix_s: i64) -> FileEntry {
        FileEntry {
            path: path.to_owned(),
            size: 1,
            modified_unix_s,
        }
    }

    fn visible_paths(picker: &FilePickerState) -> Vec<&str> {
        (0..picker.visible_len())
            .filter_map(|i| picker.visible_entry(i))
            .map(|entry| entry.path.as_str())
            .collect()
    }

    fn type_query(picker: &mut FilePickerState, query: &str) {
        query.chars().for_each(|ch| picker.insert_char(ch));
    }

    #[test]
    fn newest_file_comes_first_whatever_order_the_command_names_them_in() {
        let mut picker = FilePickerState::new(1, None);

        picker.add_entries(vec![entry("/d/old.png", 100), entry("/d/new.png", 300)]);
        picker.add_entries(vec![entry("/d/mid.png", 200)]);

        assert_eq!(
            visible_paths(&picker),
            ["/d/new.png", "/d/mid.png", "/d/old.png"]
        );
        assert_eq!(picker.selected(), Some(&entry("/d/new.png", 300)));
    }

    #[test]
    fn an_empty_picker_has_nothing_selected() {
        let picker = FilePickerState::new(1, None);

        assert_eq!(picker.selected_index(), None);
        assert_eq!(picker.selected(), None);
    }

    #[test]
    fn query_keeps_files_whose_path_holds_every_word_ignoring_case() {
        let mut picker = FilePickerState::new(1, None);
        picker.add_entries(vec![
            entry("/home/me/Desktop/shot.png", 3),
            entry("/home/me/Desktop/notes.txt", 2),
            entry("/home/me/Downloads/photo.PNG", 1),
        ]);

        type_query(&mut picker, "desk png");

        assert_eq!(visible_paths(&picker), ["/home/me/Desktop/shot.png"]);
    }

    #[test]
    fn query_does_not_match_the_home_directory_hidden_behind_the_tilde() {
        let mut picker = FilePickerState::new(1, Some("/home/me".to_owned()));
        picker.add_entries(vec![
            entry("/home/me/Desktop/shot.png", 2),
            entry("/home/me/Desktop/home.txt", 1),
        ]);
        assert_eq!(
            picker.display_path(&entry("/home/me/Desktop/shot.png", 2)),
            "~/Desktop/shot.png"
        );
        assert_eq!(
            picker.display_path(&entry("/home/merlin/a.png", 2)),
            "/home/merlin/a.png"
        );

        type_query(&mut picker, "home");

        assert_eq!(visible_paths(&picker), ["/home/me/Desktop/home.txt"]);
    }

    #[test]
    fn a_file_named_twice_is_listed_once() {
        let mut picker = FilePickerState::new(1, None);

        picker.add_entries(vec![entry("/d/a.png", 2), entry("/d/b.png", 1)]);
        picker.add_entries(vec![entry("/d/a.png", 2)]);

        assert_eq!(visible_paths(&picker), ["/d/a.png", "/d/b.png"]);
    }

    #[test]
    fn deleting_the_query_brings_the_files_back() {
        let mut picker = FilePickerState::new(1, None);
        picker.add_entries(vec![entry("/d/a.png", 2), entry("/d/b.txt", 1)]);
        type_query(&mut picker, "q");
        assert_eq!(picker.visible_len(), 0);

        picker.delete_char_before();

        assert_eq!(picker.query(), "");
        assert_eq!(picker.visible_len(), 2);
    }

    #[test]
    fn selection_stops_at_both_ends() {
        let mut picker = FilePickerState::new(1, None);
        picker.add_entries(vec![entry("/d/a", 2), entry("/d/b", 1)]);

        picker.select_previous();
        assert_eq!(picker.selected_index(), Some(0));

        picker.select_next();
        picker.select_next();
        assert_eq!(picker.selected_index(), Some(1));
    }

    #[test]
    fn changing_the_query_selects_the_newest_match() {
        let mut picker = FilePickerState::new(1, None);
        picker.add_entries(vec![entry("/d/a.png", 3), entry("/d/b.png", 2)]);
        picker.select_next();

        type_query(&mut picker, "png");

        assert_eq!(picker.selected(), Some(&entry("/d/a.png", 3)));
    }

    #[test]
    fn selection_stays_on_the_newest_file_while_untouched() {
        let mut picker = FilePickerState::new(1, None);
        picker.add_entries(vec![entry("/d/a", 100)]);

        picker.add_entries(vec![entry("/d/b", 200)]);

        assert_eq!(picker.selected(), Some(&entry("/d/b", 200)));
    }

    #[test]
    fn selection_follows_the_chosen_file_when_newer_files_arrive() {
        let mut picker = FilePickerState::new(1, None);
        picker.add_entries(vec![entry("/d/a", 300), entry("/d/b", 200)]);
        picker.select_next();

        picker.add_entries(vec![entry("/d/c", 400), entry("/d/d", 250)]);

        assert_eq!(picker.selected(), Some(&entry("/d/b", 200)));
        assert_eq!(picker.selected_index(), Some(3));
    }

    #[test]
    fn finish_records_how_the_listing_ended() {
        let mut picker = FilePickerState::new(1, None);
        assert_eq!(picker.listing(), &FileListing::Running);

        picker.finish(None);
        assert_eq!(picker.listing(), &FileListing::Finished);

        picker.finish(Some("command not found: fd".to_owned()));
        assert_eq!(
            picker.listing(),
            &FileListing::Failed("command not found: fd".to_owned())
        );
    }
}
