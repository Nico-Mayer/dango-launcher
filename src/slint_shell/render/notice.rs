//! The notice window's properties, and when the window comes and goes.

use crate::slint_shell::generated::NoticeWindow;
use crate::ui::snapshot::Notice;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Change {
    Show,
    Hide,
}

/// A notice with a new id shows again even when one is up, so its text changes
/// and it is placed on the display the user is on now.
pub fn change(shown: Option<u64>, notice: Option<&Notice>) -> Option<Change> {
    match (shown, notice) {
        (_, Some(notice)) if shown != Some(notice.id) => Some(Change::Show),
        (Some(_), None) => Some(Change::Hide),
        _ => None,
    }
}

pub fn write(ui: &NoticeWindow, notice: &Notice) {
    ui.set_text(notice.text.as_str().into());
    ui.set_style(super::shell::ticker_style(notice.style));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::TickerStyle;

    fn notice(id: u64) -> Notice {
        Notice {
            id,
            style: TickerStyle::Success,
            text: "Copied to clipboard".into(),
        }
    }

    #[test]
    fn a_new_notice_shows_and_a_gone_one_hides() {
        assert_eq!(change(None, Some(&notice(1))), Some(Change::Show));
        assert_eq!(change(Some(1), Some(&notice(1))), None);
        assert_eq!(change(Some(1), Some(&notice(2))), Some(Change::Show));
        assert_eq!(change(Some(1), None), Some(Change::Hide));
        assert_eq!(change(None, None), None);
    }
}
