//! The `ListData` global: rows under headings, flat, so scrolling and selection
//! maths stay in one place.

use std::rc::Rc;

use slint::{ComponentHandle, Model, ModelRc, StyledText, VecModel};

use super::height::{HEADING, ROW};
use super::icon;
use crate::protocol::{Accessory, ListItem, Status, TagRole};
use crate::slint_shell::generated::{
    AccessoryData, AccessoryKind, LauncherWindow, ListData, RowData, Status as SlintStatus,
    TagRole as SlintTagRole,
};
use crate::slint_shell::icons::Icons;
use crate::slint_shell::keys;
use crate::ui::snapshot::{ListBody, RootScreen};
use crate::ui::ResultItem;

const ACCESSORY_LIMIT: usize = 3;

pub struct Writer {
    rows: Rc<VecModel<RowData>>,
    selected: Option<String>,
}

impl Writer {
    pub fn new(ui: &LauncherWindow) -> Self {
        let rows = Rc::new(VecModel::default());
        ui.global::<ListData>()
            .set_rows(ModelRc::from(rows.clone()));
        Self {
            rows,
            selected: None,
        }
    }

    pub fn clear(&mut self) {
        self.rows.set_vec(Vec::new());
        self.selected = None;
    }

    pub fn root(&mut self, ui: &LauncherWindow, root: &RootScreen, icons: &Icons) {
        ui.global::<ListData>().set_detail_pane(false);
        ui.global::<ListData>().set_skeleton(false);
        let selected = root.selected.as_deref();
        let rows = root.sections.iter().flat_map(|section| {
            let heading = section.heading.iter().map(|title| heading(title, None));
            let items = section
                .items
                .iter()
                .map(|item| result_row(item, selected, icons));
            heading.chain(items).collect::<Vec<_>>()
        });
        self.rows.set_vec(rows.collect::<Vec<_>>());
    }

    pub fn view(&mut self, ui: &LauncherWindow, list: &ListBody, icons: &Icons) {
        ui.global::<ListData>().set_detail_pane(list.pane.is_some());
        ui.global::<ListData>().set_skeleton(list.skeleton);
        let selected = list.selected.as_deref();
        let rows = list.sections.iter().flat_map(|section| {
            let heading = section
                .title
                .iter()
                .map(|title| heading(title, section.subtitle.as_deref()));
            let items = section
                .items
                .iter()
                .map(|item| list_row(item, selected, icons));
            heading.chain(items).collect::<Vec<_>>()
        });
        self.rows.set_vec(rows.collect::<Vec<_>>());
    }

    /// Scrolls a newly selected row into view.
    pub fn reveal(&mut self, ui: &LauncherWindow, selected: Option<String>) {
        if selected == self.selected {
            return;
        }
        if let Some(id) = &selected {
            if let Some((top, height)) = self.place(id) {
                ui.invoke_reveal_row(top, height);
            }
        }
        self.selected = selected;
    }

    /// Rows have fixed heights, so the selected row's place is known without
    /// asking the list. A group's first row brings its heading into view too.
    fn place(&self, id: &str) -> Option<(f32, f32)> {
        let mut top = 0.0;
        let mut heading_above = false;
        for row in self.rows.iter() {
            if !row.heading.is_empty() {
                top += HEADING;
                heading_above = true;
                continue;
            }
            if row.id == id {
                return Some(if heading_above {
                    (top - HEADING, ROW + HEADING)
                } else {
                    (top, ROW)
                });
            }
            top += ROW;
            heading_above = false;
        }
        None
    }
}

fn heading(title: &str, subtitle: Option<&str>) -> RowData {
    RowData {
        heading: title.into(),
        heading_subtitle: subtitle.unwrap_or_default().into(),
        ..Default::default()
    }
}

fn result_row(item: &ResultItem, selected: Option<&str>, icons: &Icons) -> RowData {
    let (icon, icon_kind) = icon(icons, item.icon.as_deref());
    RowData {
        id: item.id.as_str().into(),
        title: highlighted(&item.title, &item.match_positions),
        label: item.title.as_str().into(),
        subtitle: item.subtitle.clone().unwrap_or_default().into(),
        icon,
        icon_kind,
        selected: selected == Some(item.id.as_str()),
        ..Default::default()
    }
}

fn list_row(item: &ListItem, selected: Option<&str>, icons: &Icons) -> RowData {
    let (icon, icon_kind) = icon(icons, item.icon.as_deref());
    let accessories: Vec<AccessoryData> = item
        .accessories
        .iter()
        .take(ACCESSORY_LIMIT)
        .map(|accessory| accessory_data(accessory, icons))
        .collect();
    RowData {
        id: item.id.as_str().into(),
        title: StyledText::from_plain_text(&item.title),
        label: item.title.as_str().into(),
        subtitle: item.subtitle.clone().unwrap_or_default().into(),
        icon,
        icon_kind,
        content: true,
        accessories: ModelRc::from(Rc::new(VecModel::from(accessories))),
        selected: selected == Some(item.id.as_str()),
        ..Default::default()
    }
}

fn accessory_data(accessory: &Accessory, icons: &Icons) -> AccessoryData {
    let text = |kind, text: &str| AccessoryData {
        kind,
        text: text.into(),
        ..Default::default()
    };
    match accessory {
        Accessory::Text { text: value } => text(AccessoryKind::Text, value),
        Accessory::Icon { icon: name } => {
            let (icon, icon_kind) = icon(icons, Some(name));
            AccessoryData {
                kind: AccessoryKind::Icon,
                icon,
                icon_kind,
                ..Default::default()
            }
        }
        Accessory::Tag { text: value, role } => AccessoryData {
            role: match role {
                TagRole::Accent => SlintTagRole::Accent,
                TagRole::Muted => SlintTagRole::Muted,
                TagRole::Danger => SlintTagRole::Danger,
            },
            ..text(AccessoryKind::Tag, value)
        },
        Accessory::Kbd { shortcut } => text(AccessoryKind::Kbd, &keys::shortcut_label(shortcut)),
        Accessory::Progress { value } => AccessoryData {
            kind: AccessoryKind::Progress,
            value: value.map_or(-1.0, |value| value.clamp(0.0, 1.0)),
            ..Default::default()
        },
        Accessory::Status { status } => AccessoryData {
            kind: AccessoryKind::Status,
            status: slint_status(*status),
            ..Default::default()
        },
    }
}

/// The title with the characters that matched the query underlined. Not
/// emphasis, because emphasis markers stop working next to spaces and
/// punctuation and a tag does not.
fn highlighted(title: &str, positions: &[usize]) -> StyledText {
    if positions.is_empty() {
        return StyledText::from_plain_text(title);
    }
    let mut markdown = String::new();
    let mut open = false;
    for (index, c) in title.chars().enumerate() {
        let matched = positions.contains(&index);
        if matched != open {
            markdown.push_str(if matched { "<u>" } else { "</u>" });
            open = matched;
        }
        if c.is_ascii_punctuation() {
            markdown.push('\\');
        }
        markdown.push(c);
    }
    if open {
        markdown.push_str("</u>");
    }
    StyledText::from_markdown(&markdown).unwrap_or_else(|_| StyledText::from_plain_text(title))
}

pub(super) fn slint_status(status: Status) -> SlintStatus {
    match status {
        Status::Working => SlintStatus::Working,
        Status::NeedsInput => SlintStatus::NeedsInput,
        Status::Done => SlintStatus::Done,
        Status::Failed => SlintStatus::Failed,
        Status::Queued => SlintStatus::Queued,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn punctuation_in_a_title_keeps_its_text_and_underlines() {
        // StyledText exposes its text and spans only through Debug.
        let title = format!("{:?}", highlighted("C++ (x)", &[0, 4]));
        assert!(title.contains(r#"text: "C++ (x)""#), "{title}");
        assert!(title.contains("range: 0..1, style: Underline"), "{title}");
        assert!(title.contains("range: 4..5, style: Underline"), "{title}");
    }
}
