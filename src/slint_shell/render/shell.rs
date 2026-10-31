//! The `Shell` and `Search` globals: where the user is, what Enter does, and
//! what is happening.

use slint::ComponentHandle;

use super::icon;
use crate::slint_shell::generated::{
    LauncherWindow, ScreenKind, Search, Shell, ToastStyle as SlintToastStyle,
};
use crate::slint_shell::icons::Icons;
use crate::slint_shell::keys;
use crate::ui::snapshot::{Screen, Snapshot};
use crate::ui::ToastStyle;

pub fn init(ui: &LauncherWindow, icons: &Icons) {
    let shell = ui.global::<Shell>();
    shell.set_back_icon(icon(icons, Some("icon:arrow-left")).0);
    shell.set_panel_key(keys::panel_label().into());
}

pub fn write(ui: &LauncherWindow, snapshot: &Snapshot, kind: ScreenKind, icons: &Icons) {
    let shell = ui.global::<Shell>();
    shell.set_screen(kind);
    let typing = matches!(&snapshot.screen, Screen::Root(_))
        || matches!(kind, ScreenKind::List)
        || matches!(&snapshot.screen, Screen::View(_)) && kind == ScreenKind::Empty;
    shell.set_typing(typing);
    shell.set_can_go_back(!matches!(snapshot.screen, Screen::Root(_)));
    shell.set_loading(matches!(&snapshot.screen, Screen::View(view) if view.busy));

    let footer = &snapshot.footer;
    let (context_icon, context_kind) = icon(icons, footer.context.icon.as_deref());
    shell.set_context(footer.context.title.as_str().into());
    shell.set_context_icon(context_icon);
    shell.set_context_icon_kind(context_kind);
    shell.set_title(footer.context.title.as_str().into());
    shell.set_primary_label(footer.primary_label.clone().unwrap_or_default().into());
    shell.set_confirm_key(keys::confirm_label(footer.confirm_with_command).into());
    shell.set_panel_hint(footer.many_actions);

    match &snapshot.toast {
        Some(toast) => {
            shell.set_toast_shown(true);
            shell.set_toast_text(toast.text.as_str().into());
            shell.set_toast_style(match toast.style {
                ToastStyle::Busy => SlintToastStyle::Busy,
                ToastStyle::Success => SlintToastStyle::Success,
                ToastStyle::Failure => SlintToastStyle::Failure,
            });
        }
        None => shell.set_toast_shown(false),
    }

    ui.global::<Search>().set_placeholder(
        if matches!(snapshot.screen, Screen::Root(_)) {
            "Search apps and commands"
        } else {
            "Search"
        }
        .into(),
    );
}
