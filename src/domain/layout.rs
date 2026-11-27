use serde::{Deserialize, Serialize};

/// The `[layout] columns` setting: how many panels the window shows at once.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LayoutColumns {
    /// One column in a narrow window, two in a wide one.
    #[default]
    Auto,
    One,
    Two,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColumnLayout {
    SingleColumn,
    TwoColumn,
}

/// The narrowest window, in terminal columns, that `LayoutColumns::Auto`
/// still splits in two.
const TWO_COLUMN_MIN_WIDTH: u16 = 100;

impl LayoutColumns {
    pub fn layout_at(self, window_width: u16) -> ColumnLayout {
        match self {
            Self::One => ColumnLayout::SingleColumn,
            Self::Two => ColumnLayout::TwoColumn,
            Self::Auto if window_width < TWO_COLUMN_MIN_WIDTH => ColumnLayout::SingleColumn,
            Self::Auto => ColumnLayout::TwoColumn,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auto_shows_one_column_below_100_columns_and_two_from_100() {
        assert_eq!(
            LayoutColumns::Auto.layout_at(99),
            ColumnLayout::SingleColumn
        );
        assert_eq!(LayoutColumns::Auto.layout_at(100), ColumnLayout::TwoColumn);
    }

    #[test]
    fn one_and_two_ignore_the_window_width() {
        assert_eq!(
            LayoutColumns::One.layout_at(200),
            ColumnLayout::SingleColumn
        );
        assert_eq!(LayoutColumns::Two.layout_at(40), ColumnLayout::TwoColumn);
    }
}
