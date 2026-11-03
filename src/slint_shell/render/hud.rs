//! The HUD window's properties, and when the window comes and goes.

use crate::slint_shell::generated::HudWindow;
use crate::ui::snapshot::Hud;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Change {
    Show,
    Hide,
}

/// A HUD with a new id shows again even when one is up, so its text changes
/// and it is placed on the display the user is on now.
pub fn change(shown: Option<u64>, hud: Option<&Hud>) -> Option<Change> {
    match (shown, hud) {
        (_, Some(hud)) if shown != Some(hud.id) => Some(Change::Show),
        (Some(_), None) => Some(Change::Hide),
        _ => None,
    }
}

pub fn write(ui: &HudWindow, hud: &Hud) {
    ui.set_text(hud.text.as_str().into());
    ui.set_style(super::shell::toast_style(hud.style));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::ToastStyle;

    fn hud(id: u64) -> Hud {
        Hud {
            id,
            style: ToastStyle::Success,
            text: "Copied to clipboard".into(),
        }
    }

    #[test]
    fn a_new_hud_shows_and_a_gone_one_hides() {
        assert_eq!(change(None, Some(&hud(1))), Some(Change::Show));
        assert_eq!(change(Some(1), Some(&hud(1))), None);
        assert_eq!(change(Some(1), Some(&hud(2))), Some(Change::Show));
        assert_eq!(change(Some(1), None), Some(Change::Hide));
        assert_eq!(change(None, None), None);
    }
}
