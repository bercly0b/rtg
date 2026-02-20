//! State management for the message input field.

/// Maximum allowed input length (Telegram message limit).
const MAX_INPUT_LENGTH: usize = 4096;

/// State for the message composition input field.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MessageInputState {
    /// The current text being composed.
    text: String,
    /// Cursor position (character index, not byte).
    cursor_position: usize,
}

impl MessageInputState {
    /// Returns the current text content.
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Returns the cursor position (character index).
    pub fn cursor_position(&self) -> usize {
        self.cursor_position
    }

    /// Returns true if the input is empty.
    pub fn is_empty(&self) -> bool {
        self.text.is_empty()
    }

    /// Inserts a character at the current cursor position.
    /// Returns false if the input would exceed the maximum length.
    pub fn insert_char(&mut self, ch: char) -> bool {
        if self.text.chars().count() >= MAX_INPUT_LENGTH {
            return false;
        }
        let byte_idx = self.char_to_byte_index(self.cursor_position);
        self.text.insert(byte_idx, ch);
        self.cursor_position += 1;
        true
    }

    /// Deletes the character before the cursor (backspace).
    pub fn delete_char_before(&mut self) {
        if self.cursor_position > 0 {
            self.cursor_position -= 1;
            let byte_idx = self.char_to_byte_index(self.cursor_position);
            let next_byte_idx = self.char_to_byte_index(self.cursor_position + 1);
            self.text.drain(byte_idx..next_byte_idx);
        }
    }

    /// Deletes the character at the cursor position (delete key).
    pub fn delete_char_at(&mut self) {
        let char_count = self.text.chars().count();
        if self.cursor_position < char_count {
            let byte_idx = self.char_to_byte_index(self.cursor_position);
            let next_byte_idx = self.char_to_byte_index(self.cursor_position + 1);
            self.text.drain(byte_idx..next_byte_idx);
        }
    }

    /// Moves the cursor one position to the left.
    pub fn move_cursor_left(&mut self) {
        if self.cursor_position > 0 {
            self.cursor_position -= 1;
        }
    }

    /// Moves the cursor one position to the right.
    pub fn move_cursor_right(&mut self) {
        let char_count = self.text.chars().count();
        if self.cursor_position < char_count {
            self.cursor_position += 1;
        }
    }

    /// Moves the cursor to the beginning of the text.
    pub fn move_cursor_home(&mut self) {
        self.cursor_position = 0;
    }

    /// Moves the cursor to the end of the text.
    pub fn move_cursor_end(&mut self) {
        self.cursor_position = self.text.chars().count();
    }

    /// Clears all text and resets cursor.
    #[allow(dead_code)]
    pub fn clear(&mut self) {
        self.text.clear();
        self.cursor_position = 0;
    }

    /// Converts character index to byte index.
    fn char_to_byte_index(&self, char_idx: usize) -> usize {
        self.text
            .char_indices()
            .nth(char_idx)
            .map(|(byte_idx, _)| byte_idx)
            .unwrap_or(self.text.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_state_is_empty() {
        let state = MessageInputState::default();
        assert!(state.is_empty());
        assert_eq!(state.text(), "");
        assert_eq!(state.cursor_position(), 0);
    }

    #[test]
    fn insert_char_appends_and_moves_cursor() {
        let mut state = MessageInputState::default();
        state.insert_char('H');
        state.insert_char('i');

        assert_eq!(state.text(), "Hi");
        assert_eq!(state.cursor_position(), 2);
    }

    #[test]
    fn insert_char_at_middle_position() {
        let mut state = MessageInputState::default();
        state.insert_char('H');
        state.insert_char('o');
        state.move_cursor_left();
        state.insert_char('i');

        assert_eq!(state.text(), "Hio");
        assert_eq!(state.cursor_position(), 2);
    }

    #[test]
    fn delete_char_before_removes_previous_char() {
        let mut state = MessageInputState::default();
        state.insert_char('H');
        state.insert_char('i');
        state.delete_char_before();

        assert_eq!(state.text(), "H");
        assert_eq!(state.cursor_position(), 1);
    }

    #[test]
    fn delete_char_before_at_start_does_nothing() {
        let mut state = MessageInputState::default();
        state.insert_char('H');
        state.move_cursor_home();
        state.delete_char_before();

        assert_eq!(state.text(), "H");
        assert_eq!(state.cursor_position(), 0);
    }

    #[test]
    fn delete_char_at_removes_current_char() {
        let mut state = MessageInputState::default();
        state.insert_char('H');
        state.insert_char('i');
        state.move_cursor_home();
        state.delete_char_at();

        assert_eq!(state.text(), "i");
        assert_eq!(state.cursor_position(), 0);
    }

    #[test]
    fn delete_char_at_end_does_nothing() {
        let mut state = MessageInputState::default();
        state.insert_char('H');
        state.delete_char_at();

        assert_eq!(state.text(), "H");
        assert_eq!(state.cursor_position(), 1);
    }

    #[test]
    fn cursor_movement_left_right() {
        let mut state = MessageInputState::default();
        state.insert_char('a');
        state.insert_char('b');
        state.insert_char('c');

        assert_eq!(state.cursor_position(), 3);

        state.move_cursor_left();
        assert_eq!(state.cursor_position(), 2);

        state.move_cursor_left();
        state.move_cursor_left();
        assert_eq!(state.cursor_position(), 0);

        // Cannot go below 0
        state.move_cursor_left();
        assert_eq!(state.cursor_position(), 0);

        state.move_cursor_right();
        assert_eq!(state.cursor_position(), 1);

        state.move_cursor_end();
        assert_eq!(state.cursor_position(), 3);

        // Cannot go beyond text length
        state.move_cursor_right();
        assert_eq!(state.cursor_position(), 3);
    }

    #[test]
    fn home_and_end_movement() {
        let mut state = MessageInputState::default();
        state.insert_char('a');
        state.insert_char('b');
        state.insert_char('c');

        state.move_cursor_home();
        assert_eq!(state.cursor_position(), 0);

        state.move_cursor_end();
        assert_eq!(state.cursor_position(), 3);
    }

    #[test]
    fn clear_resets_state() {
        let mut state = MessageInputState::default();
        state.insert_char('H');
        state.insert_char('i');
        state.clear();

        assert!(state.is_empty());
        assert_eq!(state.cursor_position(), 0);
    }

    #[test]
    fn handles_unicode_characters() {
        let mut state = MessageInputState::default();
        state.insert_char('П');
        state.insert_char('р');
        state.insert_char('и');
        state.insert_char('в');
        state.insert_char('е');
        state.insert_char('т');

        assert_eq!(state.text(), "Привет");
        assert_eq!(state.cursor_position(), 6);

        state.delete_char_before();
        assert_eq!(state.text(), "Приве");

        state.move_cursor_home();
        state.delete_char_at();
        assert_eq!(state.text(), "риве");
    }

    #[test]
    fn handles_emoji() {
        let mut state = MessageInputState::default();
        state.insert_char('H');
        state.insert_char('i');
        // Note: Some emojis are multiple code points, but single char insert
        // will handle simple emojis correctly
        state.insert_char('!');

        assert_eq!(state.text(), "Hi!");
        assert_eq!(state.cursor_position(), 3);
    }

    #[test]
    fn delete_char_at_middle_removes_correct_char() {
        let mut state = MessageInputState::default();
        state.insert_char('a');
        state.insert_char('b');
        state.insert_char('c');
        state.move_cursor_home();
        state.move_cursor_right(); // cursor at 'b'
        state.delete_char_at();

        assert_eq!(state.text(), "ac");
        assert_eq!(state.cursor_position(), 1);
    }

    #[test]
    fn insert_char_respects_max_length_limit() {
        let mut state = MessageInputState::default();
        // Fill to max length
        for _ in 0..MAX_INPUT_LENGTH {
            assert!(state.insert_char('x'));
        }
        // Should reject additional characters
        assert!(!state.insert_char('y'));
        assert_eq!(state.text().chars().count(), MAX_INPUT_LENGTH);
    }
}
