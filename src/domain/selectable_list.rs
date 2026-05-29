//! Generic cursor-over-Vec used by left-panel state types (chat list,
//! forum topic list).
//!
//! Owns the items and the selected index, and exposes cursor-only navigation
//! (`select_next`, `select_previous`, `select_first`). Replacing the items is
//! a separate step that lets the caller resolve which item should remain
//! selected — selection-by-id logic stays in the owning state because the id
//! types differ (`i64` for chats, `i32` for topics).

#[cfg_attr(not(test), allow(dead_code))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectableList<T> {
    items: Vec<T>,
    selected_index: Option<usize>,
}

impl<T> Default for SelectableList<T> {
    fn default() -> Self {
        Self {
            items: Vec::new(),
            selected_index: None,
        }
    }
}

#[cfg_attr(not(test), allow(dead_code))]
impl<T> SelectableList<T> {
    pub fn items(&self) -> &[T] {
        &self.items
    }

    pub fn selected_index(&self) -> Option<usize> {
        self.selected_index
    }

    pub fn selected(&self) -> Option<&T> {
        self.selected_index.and_then(|i| self.items.get(i))
    }

    pub fn selected_mut(&mut self) -> Option<&mut T> {
        let index = self.selected_index?;
        self.items.get_mut(index)
    }

    /// Replaces items, preferring the index pre-resolved by the caller. Falls
    /// back to selecting the first item, or `None` when the new list is empty.
    pub fn replace(&mut self, items: Vec<T>, preferred_index: Option<usize>) {
        if items.is_empty() {
            self.clear();
            return;
        }
        let len = items.len();
        self.items = items;
        self.selected_index = preferred_index.filter(|&i| i < len).or(Some(0));
    }

    pub fn clear(&mut self) {
        self.items.clear();
        self.selected_index = None;
    }

    /// Sets the selected index, or `None` to deselect. Out-of-bounds indices
    /// are ignored.
    pub fn set_selected_index(&mut self, index: Option<usize>) {
        match index {
            Some(i) if i < self.items.len() => self.selected_index = Some(i),
            None => self.selected_index = None,
            _ => {}
        }
    }

    pub fn select_next(&mut self) {
        let Some(index) = self.selected_index else {
            return;
        };
        let last = self.items.len().saturating_sub(1);
        self.selected_index = Some(std::cmp::min(index.saturating_add(1), last));
    }

    pub fn select_previous(&mut self) {
        let Some(index) = self.selected_index else {
            return;
        };
        self.selected_index = Some(index.saturating_sub(1));
    }

    pub fn select_first(&mut self) {
        if !self.items.is_empty() {
            self.selected_index = Some(0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_empty_with_no_selection() {
        let list: SelectableList<i32> = SelectableList::default();
        assert!(list.items().is_empty());
        assert_eq!(list.selected_index(), None);
        assert_eq!(list.selected(), None);
    }

    #[test]
    fn replace_with_data_selects_first_when_no_preferred() {
        let mut list = SelectableList::default();
        list.replace(vec![10, 20, 30], None);
        assert_eq!(list.items(), &[10, 20, 30]);
        assert_eq!(list.selected_index(), Some(0));
    }

    #[test]
    fn replace_honors_preferred_index_when_in_bounds() {
        let mut list = SelectableList::default();
        list.replace(vec![10, 20, 30], Some(2));
        assert_eq!(list.selected_index(), Some(2));
    }

    #[test]
    fn replace_falls_back_to_first_when_preferred_out_of_bounds() {
        let mut list = SelectableList::default();
        list.replace(vec![10, 20], Some(5));
        assert_eq!(list.selected_index(), Some(0));
    }

    #[test]
    fn replace_with_empty_clears_selection() {
        let mut list = SelectableList::default();
        list.replace(vec![10, 20], None);
        list.replace(Vec::<i32>::new(), None);
        assert!(list.items().is_empty());
        assert_eq!(list.selected_index(), None);
    }

    #[test]
    fn select_next_clamps_at_last() {
        let mut list = SelectableList::default();
        list.replace(vec![1, 2, 3], None);
        list.select_next();
        list.select_next();
        list.select_next();
        assert_eq!(list.selected_index(), Some(2));
    }

    #[test]
    fn select_previous_clamps_at_first() {
        let mut list = SelectableList::default();
        list.replace(vec![1, 2, 3], Some(2));
        list.select_previous();
        list.select_previous();
        list.select_previous();
        assert_eq!(list.selected_index(), Some(0));
    }

    #[test]
    fn select_first_resets_cursor_to_zero() {
        let mut list = SelectableList::default();
        list.replace(vec![1, 2, 3], Some(2));
        list.select_first();
        assert_eq!(list.selected_index(), Some(0));
    }

    #[test]
    fn navigation_no_op_when_empty() {
        let mut list: SelectableList<i32> = SelectableList::default();
        list.select_next();
        list.select_previous();
        list.select_first();
        assert_eq!(list.selected_index(), None);
    }

    #[test]
    fn set_selected_index_within_bounds_updates_selection() {
        let mut list = SelectableList::default();
        list.replace(vec![1, 2, 3], None);
        list.set_selected_index(Some(2));
        assert_eq!(list.selected_index(), Some(2));
    }

    #[test]
    fn set_selected_index_out_of_bounds_is_ignored() {
        let mut list = SelectableList::default();
        list.replace(vec![1, 2, 3], Some(1));
        list.set_selected_index(Some(99));
        assert_eq!(list.selected_index(), Some(1));
    }

    #[test]
    fn set_selected_index_none_deselects() {
        let mut list = SelectableList::default();
        list.replace(vec![1, 2, 3], Some(1));
        list.set_selected_index(None);
        assert_eq!(list.selected_index(), None);
    }

    #[test]
    fn selected_mut_returns_mutable_reference() {
        let mut list = SelectableList::default();
        list.replace(vec![10, 20, 30], Some(1));
        if let Some(v) = list.selected_mut() {
            *v = 99;
        }
        assert_eq!(list.items(), &[10, 99, 30]);
    }
}
