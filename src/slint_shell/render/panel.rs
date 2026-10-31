//! The `Panel` global: the actions on the selection, with headings inline.

use std::rc::Rc;

use slint::{ComponentHandle, ModelRc, VecModel};

use crate::protocol::ActionStyle;
use crate::slint_shell::generated::{ActionData, LauncherWindow, Panel};
use crate::slint_shell::keys;
use crate::ui::snapshot;

const PADDING: f32 = 6.0;
const ROW_HEIGHT: f32 = 32.0;
const HEADING_HEIGHT: f32 = 28.0;

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
        let mut section = None;
        for action in &panel.actions {
            if action.section.is_some() && action.section != section {
                rows.push(ActionData {
                    heading: action.section.clone().unwrap_or_default().into(),
                    ..Default::default()
                });
            }
            section = action.section.clone();
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

/// Where the selected action sits, counted from the fixed row heights. The
/// first action of a section brings its heading into view too.
fn place(rows: &[ActionData], selected: Option<&str>) -> Option<(f32, f32)> {
    let selected = selected?;
    let mut top = PADDING;
    let mut heading_above = false;
    for row in rows {
        if !row.heading.is_empty() {
            top += HEADING_HEIGHT;
            heading_above = true;
            continue;
        }
        if row.id == selected {
            return Some(if heading_above {
                (top - HEADING_HEIGHT, ROW_HEIGHT + HEADING_HEIGHT)
            } else {
                (top, ROW_HEIGHT)
            });
        }
        top += ROW_HEIGHT;
        heading_above = false;
    }
    None
}
