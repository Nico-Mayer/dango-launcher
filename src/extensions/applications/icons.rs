use std::io::BufWriter;
use std::path::PathBuf;
use std::sync::Arc;

use super::index::IconRgba;

pub const ICON_SIZE: u32 = 64;

/// Extracts and caches application icons as PNGs on disk, keyed by application
/// id. Extraction runs off the search path; a result whose icon is not cached
/// yet simply shows no icon until it appears, and one that cannot be extracted
/// stays without a file so the frontend renders a placeholder.
pub struct IconCache {
    dir: PathBuf,
}

impl IconCache {
    pub fn new(dir: PathBuf) -> Arc<Self> {
        let _ = std::fs::create_dir_all(&dir);
        Arc::new(Self { dir })
    }

    /// The cached icon path for an app, or `None` when nothing is cached yet.
    pub fn path(&self, app_id: &str) -> Option<String> {
        let path = self.file_for(app_id);
        path.exists().then(|| path.to_string_lossy().into_owned())
    }

    /// Writes the icon if it is missing, rendering it only then. The renderer
    /// is passed in rather than held, so the same cache serves anything with an
    /// icon without knowing where it came from.
    pub fn ensure_with(&self, key: &str, render: impl FnOnce() -> Option<IconRgba>) {
        let path = self.file_for(key);
        if path.exists() {
            return;
        }
        let Some(icon) = render() else {
            return;
        };
        if let Err(error) = write_png(&path, &icon.rgba, icon.width, icon.height) {
            eprintln!("[dango] could not cache icon for {key}: {error}");
        }
    }

    fn file_for(&self, app_id: &str) -> PathBuf {
        self.dir.join(format!("{}.png", sanitize(app_id)))
    }

    #[cfg(test)]
    pub fn in_temp() -> Arc<Self> {
        let dir = std::env::temp_dir().join(format!("dango-icons-{}", uuid::Uuid::new_v4()));
        Self::new(dir)
    }
}

fn sanitize(app_id: &str) -> String {
    app_id
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '_' })
        .collect()
}

fn write_png(path: &std::path::Path, rgba: &[u8], width: u32, height: u32) -> std::io::Result<()> {
    let file = std::fs::File::create(path)?;
    let mut encoder = png::Encoder::new(BufWriter::new(file), width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder
        .write_header()
        .and_then(|mut w| w.write_image_data(rgba))
        .map_err(std::io::Error::other)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn blue() -> Option<IconRgba> {
        Some(IconRgba {
            width: 8,
            height: 8,
            rgba: vec![64; 8 * 8 * 4],
        })
    }

    #[test]
    fn no_icon_is_cached_until_ensured() {
        let cache = IconCache::in_temp();
        assert!(cache.path("code").is_none());
    }

    #[test]
    fn ensure_caches_an_icon_that_path_then_finds() {
        let cache = IconCache::in_temp();
        cache.ensure_with("code", blue);
        assert!(cache.path("code").is_some());
    }

    #[test]
    fn a_failed_extraction_leaves_no_file_for_a_placeholder() {
        let cache = IconCache::in_temp();
        cache.ensure_with("code", || None);
        assert!(
            cache.path("code").is_none(),
            "no file means the frontend shows a placeholder"
        );
    }

    #[test]
    fn a_cached_icon_is_not_rendered_twice() {
        let cache = IconCache::in_temp();
        cache.ensure_with("code", blue);
        cache.ensure_with("code", || panic!("must not render again"));
    }
}
