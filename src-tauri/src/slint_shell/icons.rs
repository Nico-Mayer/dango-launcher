//! Every named icon the product can ask for, and a cache for icon files.
//!
//! An extension names an icon as `icon:<name>`; anything else is a path to an
//! image file it extracted, such as an application's own icon. Adding a named
//! icon is a deliberate line in `named` rather than an arbitrary string
//! somewhere in an extension.

use std::cell::RefCell;
use std::collections::HashMap;
use std::path::Path;

use icondata_core::IconData;
use slint::Image;

use crate::extension::NAMED_ICON;

pub enum Icon {
    Named(Image),
    File(Image),
    /// Unknown, or a file that would not load: a placeholder square.
    Missing,
}

/// Lives on the UI thread, as `Image` does.
#[derive(Default)]
pub struct Icons {
    cache: RefCell<HashMap<String, Option<Image>>>,
}

impl Icons {
    pub fn get(&self, icon: Option<&str>) -> Icon {
        let Some(icon) = icon else {
            return Icon::Missing;
        };
        let named = icon.strip_prefix(NAMED_ICON);
        let image = self
            .cache
            .borrow_mut()
            .entry(icon.to_string())
            .or_insert_with(|| match named {
                Some(name) => named_image(name),
                None => Image::load_from_path(Path::new(icon)).ok(),
            })
            .clone();
        match (image, named) {
            (Some(image), Some(_)) => Icon::Named(image),
            (Some(image), None) => Icon::File(image),
            (None, _) => Icon::Missing,
        }
    }
}

fn named_image(name: &str) -> Option<Image> {
    let data = named(name)?;
    Image::load_from_svg_data(svg(data).as_bytes()).ok()
}

/// Drawn in black so a tint can colourise it.
fn svg(data: &IconData) -> String {
    format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="{}" fill="none" stroke="black" stroke-width="{}" stroke-linecap="round" stroke-linejoin="round">{}</svg>"#,
        data.view_box.unwrap_or("0 0 24 24"),
        data.stroke_width.unwrap_or("2"),
        data.data,
    )
}

fn named(name: &str) -> Option<&'static IconData> {
    use icondata_lu as lu;
    Some(match name {
        "app-window" => lu::LuAppWindow,
        "check" => lu::LuCheck,
        "circle-alert" => lu::LuCircleAlert,
        "circle-arrow-down" => lu::LuCircleArrowDown,
        "circle-power" => lu::LuCirclePower,
        "clipboard-list" => lu::LuClipboardList,
        "clipboard-type" => lu::LuClipboardType,
        "columns-3" => lu::LuColumns3,
        "grid-2x2" => lu::LuGrid2x2,
        "loader-circle" => lu::LuLoaderCircle,
        "link" => lu::LuLink,
        "lock" => lu::LuLock,
        "maximize" => lu::LuMaximize,
        "maximize-2" => lu::LuMaximize2,
        "minimize-2" => lu::LuMinimize2,
        "monitor" => lu::LuMonitor,
        "moon" => lu::LuMoon,
        "panel-bottom" => lu::LuPanelBottom,
        "panel-left" => lu::LuPanelLeft,
        "panel-right" => lu::LuPanelRight,
        "panel-top" => lu::LuPanelTop,
        "refresh-cw" => lu::LuRefreshCw,
        "settings" => lu::LuSettings,
        "sparkles" => lu::LuSparkles,
        "square" => lu::LuSquare,
        "terminal" => lu::LuTerminal,
        "trash-2" => lu::LuTrash2,
        "triangle-alert" => lu::LuTriangleAlert,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const REGISTRY: [&str; 28] = [
        "app-window",
        "check",
        "circle-alert",
        "circle-arrow-down",
        "circle-power",
        "clipboard-list",
        "clipboard-type",
        "columns-3",
        "grid-2x2",
        "loader-circle",
        "link",
        "lock",
        "maximize",
        "maximize-2",
        "minimize-2",
        "monitor",
        "moon",
        "panel-bottom",
        "panel-left",
        "panel-right",
        "panel-top",
        "refresh-cw",
        "settings",
        "sparkles",
        "square",
        "terminal",
        "trash-2",
        "triangle-alert",
    ];

    #[test]
    fn every_name_in_the_registry_resolves_to_an_image() {
        let icons = Icons::default();
        for name in REGISTRY {
            let icon = icons.get(Some(&format!("{NAMED_ICON}{name}")));
            match icon {
                Icon::Named(image) => assert_eq!(image.size().width, 24, "{name}"),
                _ => panic!("{name} did not resolve"),
            }
        }
    }

    #[test]
    fn an_unknown_name_falls_back_to_the_placeholder() {
        let icons = Icons::default();
        assert!(matches!(
            icons.get(Some(&format!("{NAMED_ICON}no-such-icon"))),
            Icon::Missing
        ));
        assert!(matches!(icons.get(None), Icon::Missing));
    }

    #[test]
    fn a_missing_file_falls_back_to_the_placeholder() {
        let icons = Icons::default();
        let path = std::env::temp_dir().join("dango-no-such-icon.png");
        assert!(matches!(
            icons.get(Some(path.to_str().unwrap())),
            Icon::Missing
        ));
    }
}
