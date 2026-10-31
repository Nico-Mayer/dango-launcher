//! How tall the launcher is: the SearchBar, the view, and the ActionBar, from
//! the same fixed sizes the parts are drawn with. Each size here matches one
//! in `Theme`.

use crate::ui::snapshot::{Screen, Snapshot, ViewBody};

pub const MAX: f32 = crate::platform::LAUNCHER_MAX_HEIGHT as f32;
pub const WIDTH: f32 = 720.0;
const SEARCH_BAR: f32 = 56.0;
const ACTION_BAR: f32 = 40.0;
const LIST_PADDING: f32 = 6.0;
pub const ROW: f32 = 40.0;
pub const HEADING: f32 = 28.0;
pub const PANEL_ROW: f32 = 32.0;
pub const PANEL_PADDING: f32 = 6.0;
const PANEL_FILTER: f32 = 36.0;
const PANEL_MARGIN: f32 = 8.0;
pub const GRID_PADDING: f32 = 16.0;
pub const GRID_GAP: f32 = 8.0;
pub const TILE_TITLE: f32 = 24.0;

/// A list or grid is as tall as its content, up to the maximum. Everything
/// else, and an empty view, takes the maximum. An open panel keeps the room
/// it needs above the ActionBar.
pub fn height(snapshot: &Snapshot) -> f32 {
    let body = match &snapshot.screen {
        Screen::Root(root) if root.empty.is_none() => {
            let headings = root.sections.iter().filter(|s| s.heading.is_some()).count();
            let rows: usize = root.sections.iter().map(|s| s.items.len()).sum();
            list(headings, rows)
        }
        Screen::View(view) => match &view.body {
            ViewBody::List(body) if body.empty.is_none() && body.pane.is_none() => {
                let headings = body.sections.iter().filter(|s| s.title.is_some()).count();
                let rows: usize = body.sections.iter().map(|s| s.items.len()).sum();
                list(headings, rows)
            }
            ViewBody::Grid(body) if body.empty.is_none() => super::grid::content_height(body),
            _ => MAX,
        },
        _ => MAX,
    };
    let content = SEARCH_BAR + body + ACTION_BAR;
    let panel = if snapshot.panel.open {
        panel(snapshot) + ACTION_BAR + 2.0 * PANEL_MARGIN
    } else {
        0.0
    };
    content.max(panel).min(MAX)
}

fn list(headings: usize, rows: usize) -> f32 {
    2.0 * LIST_PADDING + headings as f32 * HEADING + rows as f32 * ROW
}

fn panel(snapshot: &Snapshot) -> f32 {
    let actions = &snapshot.panel.actions;
    let headings = super::panel::sectioned(actions)
        .filter(|(heading, _)| heading.is_some())
        .count();
    let rows = actions.len().max(1);
    2.0 * PANEL_PADDING + headings as f32 * HEADING + rows as f32 * PANEL_ROW + PANEL_FILTER
}
