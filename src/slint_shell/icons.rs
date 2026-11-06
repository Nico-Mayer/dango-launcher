//! Every named icon the product can ask for.
//!
//! An extension names an icon as `icon:<name>`; anything else is a path to an
//! image file it extracted, such as an application's own icon. Adding a named
//! icon is a deliberate line in `named` rather than an arbitrary string
//! somewhere in an extension.

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::Arc;

use icondata_core::IconData;
use slint::{Image, Rgba8Pixel, SharedPixelBuffer};

use crate::extension::NAMED_ICON;

pub enum Icon {
    Named(Image),
    File(Image),
    /// Unknown, or a file that would not load: a placeholder square.
    Missing,
}

/// A file past this size, such as a wallpaper an extension shows, is shrunk on
/// a thread of its own: decoding it on the UI thread froze the launcher, and
/// drawing it at full size slowed the software renderer on every frame.
const LARGE_FILE_BYTES: u64 = 256 * 1024;

/// The longest side of a shrunk image, enough for a tile or a detail.
const THUMBNAIL_SIDE: u32 = 512;

/// Hands a finished thumbnail back to the UI thread.
pub type ThumbnailSink = Arc<dyn Fn(String, SharedPixelBuffer<Rgba8Pixel>) + Send + Sync>;

/// Lives on the UI thread, as `Image` does.
///
/// File images are kept only from one render to the next, so a key press that
/// keeps the rows decodes nothing, yet no image outlives the screen showing it.
#[derive(Default)]
pub struct Icons {
    named: RefCell<HashMap<String, Option<Image>>>,
    shown: RefCell<HashMap<String, Image>>,
    rendering: RefCell<HashMap<String, Image>>,
    thumbnails: RefCell<HashMap<String, Image>>,
    shrinking: RefCell<HashSet<String>>,
    /// Without one, as in tests, a large file is shrunk on the spot.
    sink: Option<ThumbnailSink>,
    #[cfg(test)]
    decoded: std::cell::Cell<usize>,
}

impl Icons {
    pub fn get(&self, icon: Option<&str>) -> Icon {
        let Some(icon) = icon else {
            return Icon::Missing;
        };
        let Some(name) = icon.strip_prefix(NAMED_ICON) else {
            return self.file(icon).map_or(Icon::Missing, Icon::File);
        };
        let image = self
            .named
            .borrow_mut()
            .entry(name.to_string())
            .or_insert_with(|| named_image(name))
            .clone();
        match image {
            Some(image) => Icon::Named(image),
            None => Icon::Missing,
        }
    }

    /// Ends a render: the file images it did not ask for are let go.
    pub fn rendered(&self) {
        *self.shown.borrow_mut() = self.rendering.take();
    }

    pub fn forget_files(&self) {
        self.shown.borrow_mut().clear();
        self.rendering.borrow_mut().clear();
        self.thumbnails.borrow_mut().clear();
    }

    pub fn set_thumbnail_sink(&mut self, sink: ThumbnailSink) {
        self.sink = Some(sink);
    }

    /// Takes a thumbnail a thread finished; the next render shows it.
    pub fn add_thumbnail(&self, path: String, pixels: SharedPixelBuffer<Rgba8Pixel>) {
        self.shrinking.borrow_mut().remove(&path);
        self.thumbnails
            .borrow_mut()
            .insert(path, Image::from_rgba8(pixels));
    }

    fn file(&self, path: &str) -> Option<Image> {
        if let Some(image) = self.rendering.borrow().get(path) {
            return Some(image.clone());
        }
        let image = match self.shown.borrow_mut().remove(path) {
            Some(image) => image,
            None if is_large(path) => self.thumbnail(path)?,
            None => {
                #[cfg(test)]
                self.decoded.set(self.decoded.get() + 1);
                Image::load_from_path(Path::new(path)).ok()?
            }
        };
        self.rendering
            .borrow_mut()
            .insert(path.to_string(), image.clone());
        Some(image)
    }

    #[cfg(test)]
    pub fn decoded(&self) -> usize {
        self.decoded.get()
    }

    #[cfg(test)]
    pub fn holds_files(&self) -> bool {
        !self.shown.borrow().is_empty() || !self.rendering.borrow().is_empty()
    }
}

impl Icons {
    /// A large file's thumbnail once a thread has made it. Until then the
    /// placeholder shows, and the thread is started once.
    fn thumbnail(&self, path: &str) -> Option<Image> {
        if let Some(image) = self.thumbnails.borrow().get(path) {
            return Some(image.clone());
        }
        let Some(sink) = &self.sink else {
            return shrink(path).map(Image::from_rgba8);
        };
        if self.shrinking.borrow_mut().insert(path.to_string()) {
            let sink = sink.clone();
            let path = path.to_string();
            std::thread::spawn(move || {
                if let Some(pixels) = shrink(&path) {
                    sink(path, pixels);
                }
            });
        }
        None
    }
}

fn is_large(path: &str) -> bool {
    std::fs::metadata(path).is_ok_and(|metadata| metadata.len() > LARGE_FILE_BYTES)
}

fn shrink(path: &str) -> Option<SharedPixelBuffer<Rgba8Pixel>> {
    let image = image::open(path)
        .ok()?
        .thumbnail(THUMBNAIL_SIDE, THUMBNAIL_SIDE)
        .to_rgba8();
    Some(SharedPixelBuffer::clone_from_slice(
        image.as_raw(),
        image.width(),
        image.height(),
    ))
}

fn named_image(name: &str) -> Option<Image> {
    let data = named(name)?;
    Image::load_from_svg_data(svg(data).as_bytes()).ok()
}

/// Drawn in black so a theme role can colourise it.
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
        "arrow-left" => lu::LuArrowLeft,
        "arrow-right" => lu::LuArrowRight,
        "calculator" => lu::LuCalculator,
        "check" => lu::LuCheck,
        "circle-dot" => lu::LuCircleDot,
        "circle-alert" => lu::LuCircleAlert,
        "circle-arrow-down" => lu::LuCircleArrowDown,
        "circle-power" => lu::LuCirclePower,
        "clipboard-list" => lu::LuClipboardList,
        "clipboard-type" => lu::LuClipboardType,
        "columns-3" => lu::LuColumns3,
        "file" => lu::LuFile,
        "gamepad-2" => lu::LuGamepad2,
        "git-pull-request" => lu::LuGitPullRequest,
        "github" => lu::LuGithub,
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
        "search" => lu::LuSearch,
        "settings" => lu::LuSettings,
        "sparkles" => lu::LuSparkles,
        "square" => lu::LuSquare,
        "sun" => lu::LuSun,
        "sun-dim" => lu::LuSunDim,
        "terminal" => lu::LuTerminal,
        "trash-2" => lu::LuTrash2,
        "triangle-alert" => lu::LuTriangleAlert,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const REGISTRY: [&str; 39] = [
        "app-window",
        "arrow-left",
        "arrow-right",
        "calculator",
        "check",
        "circle-dot",
        "circle-alert",
        "circle-arrow-down",
        "circle-power",
        "clipboard-list",
        "clipboard-type",
        "columns-3",
        "file",
        "gamepad-2",
        "git-pull-request",
        "github",
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
        "search",
        "settings",
        "sparkles",
        "square",
        "sun",
        "sun-dim",
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

    fn file_icon(name: &str) -> String {
        let path =
            std::env::temp_dir().join(format!("dango-icon-{}-{name}.png", uuid::Uuid::new_v4()));
        std::fs::copy(
            concat!(env!("CARGO_MANIFEST_DIR"), "/icons/32x32.png"),
            &path,
        )
        .unwrap();
        path.to_string_lossy().into_owned()
    }

    #[test]
    fn a_file_asked_for_in_two_renders_in_a_row_is_decoded_once() {
        let icons = Icons::default();
        let path = file_icon("kept");
        for _ in 0..2 {
            assert!(matches!(icons.get(Some(&path)), Icon::File(_)));
            assert!(matches!(icons.get(Some(&path)), Icon::File(_)));
            icons.rendered();
        }
        assert_eq!(icons.decoded.get(), 1);
    }

    #[test]
    fn a_file_a_render_did_not_ask_for_is_let_go() {
        let icons = Icons::default();
        let (first, second) = (file_icon("first"), file_icon("second"));
        icons.get(Some(&first));
        icons.rendered();
        icons.get(Some(&second));
        icons.rendered();
        assert!(!icons.shown.borrow().contains_key(&first));

        icons.get(Some(&first));
        assert_eq!(icons.decoded.get(), 3);
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

    fn large_image() -> String {
        let path =
            std::env::temp_dir().join(format!("dango-wallpaper-{}.png", uuid::Uuid::new_v4()));
        let mut seed = 7u32;
        let image = image::RgbImage::from_fn(1600, 900, |_, _| {
            seed = seed.wrapping_mul(1_103_515_245).wrapping_add(12_345);
            let [a, b, c, _] = seed.to_le_bytes();
            image::Rgb([a, b, c])
        });
        image.save(&path).unwrap();
        assert!(std::fs::metadata(&path).unwrap().len() > LARGE_FILE_BYTES);
        path.to_string_lossy().into_owned()
    }

    #[test]
    fn a_large_file_is_shown_shrunk() {
        let icons = Icons::default();

        let Icon::File(image) = icons.get(Some(&large_image())) else {
            panic!("expected the image");
        };

        assert_eq!(image.size().width, THUMBNAIL_SIDE);
        assert_eq!(icons.decoded.get(), 0, "the full image was decoded");
    }

    #[test]
    fn a_large_file_is_shrunk_off_the_ui_thread_once() {
        let (sender, finished) = std::sync::mpsc::channel();
        let mut icons = Icons::default();
        icons.set_thumbnail_sink(Arc::new(move |path, pixels| {
            let _ = sender.send((path, pixels));
        }));
        let path = large_image();

        let first = icons.get(Some(&path));
        let again = icons.get(Some(&path));
        let (shrunk, pixels) = finished
            .recv_timeout(std::time::Duration::from_secs(10))
            .unwrap();
        icons.add_thumbnail(shrunk, pixels);
        icons.rendered();

        assert!(matches!(first, Icon::Missing));
        assert!(matches!(again, Icon::Missing));
        assert!(finished.try_recv().is_err(), "it was shrunk twice");
        assert!(matches!(icons.get(Some(&path)), Icon::File(_)));
    }
}
