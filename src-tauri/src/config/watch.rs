//! Watching the config file and applying edits live.
//!
//! The parent directory is watched rather than the file, because editors save
//! by writing a temp file and renaming over the target, which replaces the
//! file the OS was watching. Events are debounced so an editor's several writes
//! per save become one reload, and the app's own write is recognised and
//! ignored, the way the clipboard watcher ignores its own.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use notify::{RecursiveMode, Watcher};

use super::{Config, ConfigError};

const DEBOUNCE: Duration = Duration::from_millis(150);

/// Watches `path` and calls `apply` with the reparsed configuration whenever the
/// file changes, other than the app's own writes. Returns whether the watch
/// started; the watcher lives on its own thread for the life of the process.
pub fn watch(
    path: PathBuf,
    own_write: Arc<Mutex<Option<String>>>,
    apply: impl Fn(Result<Config, ConfigError>) + Send + 'static,
) -> bool {
    let Some(dir) = path.parent().map(|p| p.to_path_buf()) else {
        return false;
    };
    let _ = std::fs::create_dir_all(&dir);

    let file_name = path.file_name().map(|n| n.to_owned());
    let (tx, rx) = std::sync::mpsc::channel();
    let mut watcher =
        match notify::recommended_watcher(move |result: notify::Result<notify::Event>| {
            if let Ok(event) = result {
                if event
                    .paths
                    .iter()
                    .any(|p| p.file_name() == file_name.as_deref())
                {
                    let _ = tx.send(());
                }
            }
        }) {
            Ok(watcher) => watcher,
            Err(error) => {
                eprintln!("[dango] could not watch the config file: {error}");
                return false;
            }
        };

    if let Err(error) = watcher.watch(&dir, RecursiveMode::NonRecursive) {
        eprintln!("[dango] could not watch {}: {error}", dir.display());
        return false;
    }

    std::thread::spawn(move || {
        // Keep the watcher alive for the life of the thread; dropping it stops
        // the watch.
        let _watcher = watcher;
        while rx.recv().is_ok() {
            // Coalesce the burst of events one save produces.
            std::thread::sleep(DEBOUNCE);
            while rx.try_recv().is_ok() {}

            // A missing file reads as empty, which parses to the defaults, so
            // deleting the file returns everything to its defaults.
            let text = std::fs::read_to_string(&path).unwrap_or_default();
            {
                let mut own = own_write.lock().unwrap();
                if own.as_deref() == Some(text.as_str()) {
                    // The app wrote this; consume the signal and do not react.
                    *own = None;
                    continue;
                }
            }
            apply(Config::parse(&text));
        }
    });
    true
}
