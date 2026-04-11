use ratatui::text::{Line, Span};

use crate::domain::message::ForwardInfo;
use crate::ui::styles;

pub(super) fn build_forward_line(
    forward: &ForwardInfo,
    indent: &str,
    content_width: usize,
) -> Line<'static> {
    use unicode_width::UnicodeWidthStr;

    let bar = "│ ";
    let bar_width = UnicodeWidthStr::width(bar);
    let label = "Forwarded from ";
    let label_width = UnicodeWidthStr::width(label);

    let available = content_width
        .saturating_sub(bar_width)
        .saturating_sub(label_width);
    let name = super::reply::truncate_to_width(&forward.sender_name, available);

    Line::from(vec![
        Span::raw(indent.to_owned()),
        Span::styled(bar.to_owned(), styles::forward_bar_style()),
        Span::styled(label.to_owned(), styles::forward_label_style()),
        Span::styled(name, styles::forward_sender_style(&forward.sender_name)),
    ])
}
