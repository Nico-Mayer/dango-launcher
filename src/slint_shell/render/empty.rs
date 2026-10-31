//! The `EmptyData` global: what a view shows when it has nothing.

use slint::ComponentHandle;

use super::icon;
use crate::slint_shell::generated::{EmptyData, LauncherWindow};
use crate::slint_shell::icons::Icons;
use crate::ui::snapshot::EmptyText;

pub fn write(ui: &LauncherWindow, text: &EmptyText, icons: &Icons) {
    let empty = ui.global::<EmptyData>();
    let (image, kind) = icon(icons, text.icon.as_deref());
    empty.set_icon(image);
    empty.set_icon_kind(kind);
    empty.set_title(text.title.as_str().into());
    empty.set_hint(text.description.clone().unwrap_or_default().into());
}
