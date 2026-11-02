use std::collections::HashSet;
use std::io::BufWriter;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

pub const ICON_SIZE: u32 = 64;

pub struct IconRgba {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

/// Caches system icons as PNGs on disk, keyed by whatever the caller names
/// them by. Extraction runs off the path that builds a view; an icon that is
/// not cached yet simply shows a fallback until it appears, and one that cannot
/// be extracted stays without a file.
pub struct IconCache {
    dir: PathBuf,
    cached: RwLock<HashSet<String>>,
}

impl IconCache {
    pub fn new(dir: PathBuf) -> Arc<Self> {
        let _ = std::fs::create_dir_all(&dir);
        let cached = std::fs::read_dir(&dir)
            .into_iter()
            .flatten()
            .filter_map(|entry| entry.ok()?.file_name().into_string().ok())
            .filter(|name| name.ends_with(".png"))
            .collect();
        Arc::new(Self {
            dir,
            cached: RwLock::new(cached),
        })
    }

    pub fn path(&self, key: &str) -> Option<String> {
        let name = file_name(key);
        self.cached
            .read()
            .unwrap()
            .contains(&name)
            .then(|| self.dir.join(name).to_string_lossy().into_owned())
    }

    /// Writes the icon if it is missing, rendering it only then. The renderer
    /// is passed in rather than held, so the same cache serves anything with an
    /// icon without knowing where it came from.
    pub fn ensure_with(&self, key: &str, render: impl FnOnce() -> Option<IconRgba>) {
        let name = file_name(key);
        if self.cached.read().unwrap().contains(&name) {
            return;
        }
        let Some(icon) = render() else {
            return;
        };
        match write_png(&self.dir.join(&name), &icon.rgba, icon.width, icon.height) {
            Ok(()) => {
                self.cached.write().unwrap().insert(name);
            }
            Err(error) => eprintln!("[dango] could not cache icon for {key}: {error}"),
        }
    }

    /// Renders every `(key, locator)` icon not cached yet, once per key, and
    /// says whether any appeared. Lets a command show its view first and push
    /// it again once the icons exist.
    pub fn fill(
        &self,
        wanted: impl IntoIterator<Item = (String, String)>,
        render: impl Fn(&str) -> Option<IconRgba>,
        cancelled: impl Fn() -> bool,
    ) -> bool {
        let mut tried = HashSet::new();
        let mut appeared = false;
        for (key, locator) in wanted {
            if cancelled() {
                break;
            }
            if !tried.insert(key.clone()) || self.path(&key).is_some() {
                continue;
            }
            self.ensure_with(&key, || render(&locator));
            appeared |= self.path(&key).is_some();
        }
        appeared
    }

    #[cfg(test)]
    pub fn in_temp() -> Arc<Self> {
        let dir = std::env::temp_dir().join(format!("dango-icons-{}", uuid::Uuid::new_v4()));
        Self::new(dir)
    }
}

fn file_name(key: &str) -> String {
    let stem: String = key
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '_' })
        .collect();
    format!("{stem}.png")
}

fn write_png(path: &std::path::Path, rgba: &[u8], width: u32, height: u32) -> std::io::Result<()> {
    let partial = path.with_extension(format!("{}.tmp", uuid::Uuid::new_v4()));
    let written =
        encode_png(&partial, rgba, width, height).and_then(|()| std::fs::rename(&partial, path));
    if written.is_err() {
        let _ = std::fs::remove_file(&partial);
    }
    written
}

fn encode_png(path: &std::path::Path, rgba: &[u8], width: u32, height: u32) -> std::io::Result<()> {
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
    fn caching_leaves_only_the_finished_icon() {
        let cache = IconCache::in_temp();
        cache.ensure_with("code", blue);
        let names: Vec<_> = std::fs::read_dir(&cache.dir)
            .unwrap()
            .map(|entry| entry.unwrap().file_name())
            .collect();
        assert_eq!(names, ["code.png"]);
    }

    #[test]
    fn icons_cached_by_an_earlier_run_are_found() {
        let cache = IconCache::in_temp();
        cache.ensure_with("code", blue);
        let reopened = IconCache::new(cache.dir.clone());
        assert_eq!(reopened.path("code"), cache.path("code"));
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

    fn wanted(keys: &[&str]) -> Vec<(String, String)> {
        keys.iter()
            .map(|key| (key.to_string(), format!("/{key}")))
            .collect()
    }

    #[test]
    fn fill_renders_each_missing_key_once_and_reports_it() {
        let cache = IconCache::in_temp();
        cache.ensure_with("cached", blue);
        let rendered = std::sync::Mutex::new(Vec::new());

        let appeared = cache.fill(
            wanted(&["cached", "new", "new"]),
            |locator| {
                rendered.lock().unwrap().push(locator.to_string());
                blue()
            },
            || false,
        );

        assert!(appeared);
        assert_eq!(*rendered.lock().unwrap(), ["/new"]);
        assert!(cache.path("new").is_some());
    }

    #[test]
    fn fill_tries_a_failing_key_once() {
        let cache = IconCache::in_temp();
        let tries = std::sync::atomic::AtomicUsize::new(0);

        let appeared = cache.fill(
            wanted(&["pdf", "pdf"]),
            |_| {
                tries.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                None
            },
            || false,
        );

        assert!(!appeared);
        assert_eq!(tries.into_inner(), 1);
    }

    #[test]
    fn fill_stops_once_cancelled() {
        let cache = IconCache::in_temp();
        let appeared = cache.fill(wanted(&["new"]), |_| panic!("must not render"), || true);
        assert!(!appeared);
    }

    #[test]
    fn a_cached_icon_is_not_rendered_twice() {
        let cache = IconCache::in_temp();
        cache.ensure_with("code", blue);
        cache.ensure_with("code", || panic!("must not render again"));
    }
}
