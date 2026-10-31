//! The `ListData` global: rows under headings, flat, so scrolling and selection
//! maths stay in one place.

use std::rc::Rc;

use slint::{ComponentHandle, Model, ModelRc, StyledText, VecModel};

use super::icon;
use crate::protocol::{Accessory, ListItem, TagRole};
use crate::slint_shell::generated::{
    AccessoryData, AccessoryKind, LauncherWindow, ListData, RowData, TagRole as SlintTagRole,
};
use crate::slint_shell::icons::Icons;
use crate::slint_shell::keys;
use crate::ui::snapshot::{ListBody, RootScreen};
use crate::ui::ResultItem;

const ROW_HEIGHT: f32 = 40.0;
const HEADING_HEIGHT: f32 = 28.0;
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
    }

    pub fn root(&mut self, root: &RootScreen, icons: &Icons) {
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

    pub fn view(&mut self, list: &ListBody, icons: &Icons) {
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
                top += HEADING_HEIGHT;
                heading_above = true;
                continue;
            }
            if row.id == id {
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
