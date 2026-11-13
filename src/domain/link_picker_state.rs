use super::{message::LinkTarget, selectable_list::SelectableList};

/// The links and mentions of one message, offered when it holds more than one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinkPickerState {
    targets: SelectableList<LinkTarget>,
}

impl LinkPickerState {
    pub fn new(targets: Vec<LinkTarget>) -> Self {
        let mut list = SelectableList::default();
        list.replace(targets, None);
        Self { targets: list }
    }

    pub fn targets(&self) -> &[LinkTarget] {
        self.targets.items()
    }

    pub fn selected_index(&self) -> Option<usize> {
        self.targets.selected_index()
    }

    pub fn selected(&self) -> Option<&LinkTarget> {
        self.targets.selected()
    }

    pub fn select_next(&mut self) {
        self.targets.select_next();
    }

    pub fn select_previous(&mut self) {
        self.targets.select_previous();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn picker() -> LinkPickerState {
        LinkPickerState::new(vec![
            LinkTarget::Mention("alice".to_owned()),
            LinkTarget::Url("https://example.com".to_owned()),
        ])
    }

    #[test]
    fn opens_with_the_first_target_selected() {
        assert_eq!(
            picker().selected(),
            Some(&LinkTarget::Mention("alice".to_owned()))
        );
    }

    #[test]
    fn selection_stops_at_both_ends() {
        let mut picker = picker();

        picker.select_previous();
        assert_eq!(picker.selected_index(), Some(0));

        picker.select_next();
        picker.select_next();
        assert_eq!(picker.selected_index(), Some(1));
    }
}
