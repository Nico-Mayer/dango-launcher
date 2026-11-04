//! The `AnswerData` global: the calculator's card above root search.

use slint::ComponentHandle;

use crate::slint_shell::generated::{AnswerData, LauncherWindow};
use crate::slint_shell::icons::{Icon, Icons};
use crate::ui::snapshot::RootScreen;

pub fn write(ui: &LauncherWindow, root: Option<&RootScreen>, icons: &Icons) {
    let data = ui.global::<AnswerData>();
    let Some(card) = root.and_then(|root| root.answer.as_ref()) else {
        data.set_shown(false);
        return;
    };
    data.set_shown(true);
    data.set_id(card.id.as_str().into());
    data.set_expression(card.expression.as_str().into());
    data.set_answer(card.answer.as_str().into());
    data.set_selected(card.selected);
    if let Icon::Named(arrow) = icons.get(Some("icon:arrow-right")) {
        data.set_arrow(arrow);
    }
}
