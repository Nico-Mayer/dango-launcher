//! The `Shell` and `Search` globals: where the user is, what Enter does, and
//! what is happening.

use std::rc::Rc;

use slint::{ComponentHandle, ModelRc, VecModel};

use super::icon;
use crate::slint_shell::generated::{
    LauncherWindow, OptionData, ScreenKind, Search, Shell, ToastStyle as SlintToastStyle,
};
use crate::slint_shell::icons::Icons;
use crate::slint_shell::keys;
use crate::ui::snapshot::{Screen, Snapshot, ViewBody};
use crate::ui::ToastStyle;

pub fn init(ui: &LauncherWindow, icons: &Icons) {
    let shell = ui.global::<Shell>();
    shell.set_back_icon(icon(icons, Some("icon:arrow-left")).0);
    shell.set_panel_key(keys::panel_label().into());
}

pub fn write(
    ui: &LauncherWindow,
    snapshot: &Snapshot,
    kind: ScreenKind,
    typing: bool,
    launcher_height: f32,
    icons: &Icons,
) {
    let shell = ui.global::<Shell>();
    shell.set_screen(kind);
    shell.set_launcher_height(launcher_height);
    shell.set_card_shown(!matches!(&snapshot.screen, Screen::Root(root) if root.bare));
    // The textarea's focus handler may not run when its form is torn down.
    if kind != ScreenKind::Form {
        shell.set_multiline_focused(false);
    }
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
            shell.set_toast_style(toast_style(toast.style));
            let (label, keys) = toast
                .action
                .as_ref()
                .map_or_else(Default::default, |action| {
                    (
                        action.title.as_str().into(),
                        keys::shortcut_label(&action.shortcut).into(),
                    )
                });
            shell.set_toast_action_label(label);
            shell.set_toast_action_keys(keys);
        }
        None => shell.set_toast_shown(false),
    }

    let search = ui.global::<Search>();
    search.set_placeholder(
        if matches!(snapshot.screen, Screen::Root(_)) {
            "Search apps and commands"
        } else {
            "Search"
        }
        .into(),
    );
    let filter = match &snapshot.screen {
        Screen::View(view) => match &view.body {
            ViewBody::List(list) => list.filter.as_ref(),
            ViewBody::Grid(grid) => grid.filter.as_ref(),
            _ => None,
        },
        _ => None,
    };
    search.set_filter_shown(filter.is_some());
    if let Some(filter) = filter {
        let options: Vec<OptionData> = filter
            .options
            .iter()
            .map(|option| OptionData {
                id: option.id.as_str().into(),
                title: option.title.as_str().into(),
            })
            .collect();
        let current = filter
            .options
            .iter()
            .position(|option| option.id == filter.selected)
            .unwrap_or(0);
        search.set_filter_options(ModelRc::from(Rc::new(VecModel::from(options))));
        search.set_filter_current(current as i32);
    }
}

pub(super) fn toast_style(style: ToastStyle) -> SlintToastStyle {
    match style {
        ToastStyle::Busy => SlintToastStyle::Busy,
        ToastStyle::Success => SlintToastStyle::Success,
        ToastStyle::Failure => SlintToastStyle::Failure,
    }
}
