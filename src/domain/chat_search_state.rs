#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ChatSearchState {
    query: String,
    cursor_position: usize,
}

#[cfg_attr(not(test), allow(dead_code))]
impl ChatSearchState {
    pub fn query(&self) -> &str {
        &self.query
    }

    pub fn cursor_position(&self) -> usize {
        self.cursor_position
    }

    pub fn insert_char(&mut self, ch: char) {
        self.query.insert(self.cursor_position, ch);
        self.cursor_position += ch.len_utf8();
    }

    pub fn delete_char_before(&mut self) {
        if self.cursor_position == 0 {
            return;
        }
        let prev = self.query[..self.cursor_position]
            .char_indices()
            .next_back()
            .map(|(i, _)| i)
            .unwrap_or(0);
        self.query.drain(prev..self.cursor_position);
        self.cursor_position = prev;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_empty() {
        let s = ChatSearchState::default();
        assert_eq!(s.query(), "");
        assert_eq!(s.cursor_position(), 0);
    }

    #[test]
    fn insert_char_appends() {
        let mut s = ChatSearchState::default();
        s.insert_char('a');
        s.insert_char('b');
        assert_eq!(s.query(), "ab");
        assert_eq!(s.cursor_position(), 2);
    }

    #[test]
    fn insert_unicode() {
        let mut s = ChatSearchState::default();
        s.insert_char('ы');
        assert_eq!(s.query(), "ы");
        assert_eq!(s.cursor_position(), 2);
    }

    #[test]
    fn delete_char_before_removes_last() {
        let mut s = ChatSearchState::default();
        s.insert_char('a');
        s.insert_char('b');
        s.delete_char_before();
        assert_eq!(s.query(), "a");
        assert_eq!(s.cursor_position(), 1);
    }

    #[test]
    fn delete_char_before_at_start_is_noop() {
        let mut s = ChatSearchState::default();
        s.delete_char_before();
        assert_eq!(s.query(), "");
        assert_eq!(s.cursor_position(), 0);
    }

    #[test]
    fn delete_unicode_char() {
        let mut s = ChatSearchState::default();
        s.insert_char('п');
        s.insert_char('р');
        s.delete_char_before();
        assert_eq!(s.query(), "п");
    }
}
