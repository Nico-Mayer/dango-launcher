//! Watching the extensions folder, so an edit reloads only the extension it
//! touched.

use std::collections::BTreeSet;
use std::path::{Component, Path, PathBuf};
use std::time::Duration;

use notify::{RecursiveMode, Watcher};

/// One save writes several files; waiting this long after the first event
/// lets the rest land before the folder is read.
const SETTLE: Duration = Duration::from_millis(300);

/// The generated types file sits beside the extension folders and is Dango's
/// own write, never an edit to react to.
const TYPES_FILE: &str = "dango.d.ts";

/// Calls `apply` with the extension folders that changed, once per burst of
/// events. Returns false when the folder can't be watched.
pub fn watch(dir: PathBuf, apply: impl Fn(Vec<PathBuf>) + Send + 'static) -> bool {
    let (tx, rx) = std::sync::mpsc::channel::<PathBuf>();
    let root = dir.clone();
    let watcher = notify::recommended_watcher(move |result: notify::Result<notify::Event>| {
        if let Ok(event) = result {
            for path in event.paths {
                if let Some(folder) = folder_of(&root, &path) {
                    let _ = tx.send(folder);
                }
            }
        }
    });
    let mut watcher = match watcher {
        Ok(watcher) => watcher,
        Err(error) => {
            crate::log::append(&format!("can't watch the extensions folder: {error}"));
            return false;
        }
    };
    if let Err(error) = watcher.watch(&dir, RecursiveMode::Recursive) {
        crate::log::append(&format!("can't watch {}: {error}", dir.display()));
        return false;
    }
    std::thread::spawn(move || {
        let _watcher = watcher;
        while let Ok(first) = rx.recv() {
            std::thread::sleep(SETTLE);
            let mut folders = BTreeSet::from([first]);
            folders.extend(rx.try_iter());
            apply(folders.into_iter().collect());
        }
    });
    true
}

/// The extension folder a changed path belongs to: the first component under
/// the extensions folder.
fn folder_of(root: &Path, path: &Path) -> Option<PathBuf> {
    let relative = path.strip_prefix(root).ok()?;
    let Some(Component::Normal(first)) = relative.components().next() else {
        return None;
    };
    if first == TYPES_FILE {
        return None;
    }
    Some(root.join(first))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_file_inside_an_extension_names_its_folder() {
        let root = Path::new("/ext");

        assert_eq!(
            folder_of(root, Path::new("/ext/gh-prs/main.js")),
            Some(PathBuf::from("/ext/gh-prs"))
        );
        assert_eq!(
            folder_of(root, Path::new("/ext/gh-prs")),
            Some(PathBuf::from("/ext/gh-prs"))
        );
        assert_eq!(folder_of(root, Path::new("/ext/dango.d.ts")), None);
        assert_eq!(folder_of(root, Path::new("/elsewhere/x")), None);
    }
}
