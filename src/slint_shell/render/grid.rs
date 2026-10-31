//! The `GridData` global: tiles and headings with their places worked out
//! here, from the window's fixed width.

use std::rc::Rc;

use slint::{Color, ComponentHandle, Model, ModelRc, VecModel};

use super::icon;
use crate::protocol::{GridItem, TileContent};
use crate::slint_shell::generated::{GridData, LauncherWindow, TileData, TileKind};
use crate::slint_shell::icons::Icons;
use crate::ui::snapshot::GridBody;

const WIDTH: f32 = 720.0;
const PADDING: f32 = 16.0;
const GAP: f32 = 8.0;
const TITLE_HEIGHT: f32 = 24.0;
const HEADING_HEIGHT: f32 = 28.0;

pub struct Writer {
    tiles: Rc<VecModel<TileData>>,
    selected: Option<String>,
}

impl Writer {
    pub fn new(ui: &LauncherWindow) -> Self {
        let tiles = Rc::new(VecModel::default());
        ui.global::<GridData>()
            .set_tiles(ModelRc::from(tiles.clone()));
        Self {
            tiles,
            selected: None,
        }
    }

    pub fn write(&mut self, ui: &LauncherWindow, grid: &GridBody, icons: &Icons) {
        let columns = usize::from(grid.columns.max(1));
        let inner = WIDTH - 2.0 * PADDING;
        let width = (inner - GAP * (columns as f32 - 1.0)) / columns as f32;
        let box_height = width / grid.aspect.max(0.1);
        let row_height = box_height + TITLE_HEIGHT + GAP;

        let mut tiles = Vec::new();
        let mut top = GAP;
        for section in &grid.sections {
            if let Some(title) = &section.title {
                tiles.push(TileData {
                    heading: title.as_str().into(),
                    heading_subtitle: section.subtitle.clone().unwrap_or_default().into(),
                    x: PADDING,
                    y: top,
                    width: inner,
                    height: HEADING_HEIGHT,
                    ..Default::default()
                });
                top += HEADING_HEIGHT;
            }
            for (index, item) in section.items.iter().enumerate() {
                let (row, column) = (index / columns, index % columns);
                tiles.push(TileData {
                    x: PADDING + column as f32 * (width + GAP),
                    y: top + row as f32 * row_height,
                    width,
                    height: box_height + TITLE_HEIGHT,
                    box_height,
                    selected: grid.selected.as_deref() == Some(item.id.as_str()),
                    ..tile(item, icons)
                });
            }
            top += section.items.len().div_ceil(columns) as f32 * row_height;
        }
        ui.global::<GridData>().set_content_height(top + GAP);
        self.tiles.set_vec(tiles);

        if grid.selected != self.selected {
            let place = self
                .tiles
                .iter()
                .find(|tile| Some(tile.id.as_str()) == grid.selected.as_deref())
                .map(|tile| (tile.y, tile.height));
            if let Some((top, height)) = place {
                ui.invoke_reveal_tile(top, height);
            }
            self.selected = grid.selected.clone();
        }
    }
}

fn tile(item: &GridItem, icons: &Icons) -> TileData {
    let base = TileData {
        id: item.id.as_str().into(),
        title: item.title.as_str().into(),
        ..Default::default()
    };
    match &item.content {
        TileContent::Image { image } => {
            let (image, image_kind) = icon(icons, Some(image));
            TileData {
                kind: TileKind::Image,
                image,
                image_kind,
                ..base
            }
        }
        TileContent::Color { color } => TileData {
            kind: TileKind::Color,
            color: parse_color(color).unwrap_or_default(),
            ..base
        },
        TileContent::Glyph { glyph } => TileData {
            kind: TileKind::Glyph,
            glyph: glyph.as_str().into(),
            ..base
        },
        TileContent::Empty => base,
    }
}

/// `#rrggbb`, the only colour form the protocol takes.
fn parse_color(text: &str) -> Option<Color> {
    let hex = text.strip_prefix('#')?;
    if hex.len() != 6 {
        return None;
    }
    let value = u32::from_str_radix(hex, 16).ok()?;
    Some(Color::from_rgb_u8(
        (value >> 16) as u8,
        (value >> 8) as u8,
        value as u8,
    ))
}
