//! The `Panel` global: the actions on the selection, with headings inline.

use std::rc::Rc;

use slint::{ComponentHandle, ModelRc, VecModel};

use super::height::{HEADING, PANEL_PADDING, PANEL_ROW};
use crate::protocol::{Action, ActionStyle};
use crate::slint_shell::generated::{ActionData, LauncherWindow, Panel};
use crate::slint_shell::keys;
use crate::ui::snapshot;

pub struct Writer {
    rows: Rc<VecModel<ActionData>>,
    selected: Option<String>,
    open: bool,
}

impl Writer {
    pub fn new(ui: &LauncherWindow) -> Self {
        let rows = Rc::new(VecModel::default());
        ui.global::<Panel>().set_rows(ModelRc::from(rows.clone()));
        Self {
            rows,
            selected: None,
            open: false,
        }
    }

    pub fn write(&mut self, ui: &LauncherWindow, panel: &snapshot::Panel) {
        let mut rows = Vec::new();
        for (heading, action) in sectioned(&panel.actions) {
            if let Some(heading) = heading {
                rows.push(ActionData {
                    heading: heading.into(),
                    ..Default::default()
                });
            }
            rows.push(ActionData {
                id: action.id.as_str().into(),
                title: action.title.as_str().into(),
                shortcut: action
                    .shortcut
                    .as_ref()
                    .map(keys::shortcut_label)
                    .unwrap_or_default()
                    .into(),
                danger: action.style == ActionStyle::Danger,
                checked: panel.checked.as_deref() == Some(action.id.as_str()),
                selected: panel.selected.as_deref() == Some(action.id.as_str()),
                ..Default::default()
            });
        }
        let place = place(&rows, panel.selected.as_deref());
        self.rows.set_vec(rows);
        ui.global::<Panel>().set_open(panel.open);
        // The input owns its text while the panel is open, so it is seeded
        // only as the panel opens.
        if panel.open && !self.open {
            ui.invoke_set_panel_filter(panel.filter.as_str().into());
        }
        self.open = panel.open;

        if panel.selected != self.selected {
            if let Some((top, height)) = place {
                ui.invoke_reveal_action(top, height);
            }
            self.selected = panel.selected.clone();
        }
    }
}

/// Each action, with the heading drawn above it when it starts a section.
pub fn sectioned(actions: &[Action]) -> impl Iterator<Item = (Option<&str>, &Action)> {
    let mut section = None;
    actions.iter().map(move |action| {
        let starts = action.section.is_some() && action.section.as_ref() != section;
        section = action.section.as_ref();
        (action.section.as_deref().filter(|_| starts), action)
    })
}

/// Where the selected action sits, counted from the fixed row heights. The
/// first action of a section brings its heading into view too.
fn place(rows: &[ActionData], selected: Option<&str>) -> Option<(f32, f32)> {
    let selected = selected?;
    let mut top = PANEL_PADDING;
    let mut heading_above = false;
    for row in rows {
        if !row.heading.is_empty() {
            top += HEADING;
            heading_above = true;
            continue;
        }
        if row.id == selected {
            return Some(if heading_above {
                (top - HEADING, PANEL_ROW + HEADING)
            } else {
                (top, PANEL_ROW)
            });
        }
        top += PANEL_ROW;
        heading_above = false;
    }
    None
}
