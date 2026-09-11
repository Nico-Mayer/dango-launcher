use std::sync::{Arc, RwLock};

use crate::store::Store;

/// One installed application, uniform across platforms. `id` is the launch
/// identifier (an AppUserModelID on Windows, a bundle path on macOS); `target`
/// is the filesystem path for reveal and copy, when there is one.
#[derive(Clone, Debug, PartialEq)]
pub struct IndexedApp {
    pub id: String,
    pub name: String,
    pub target: Option<String>,
}

pub struct IconRgba {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

#[derive(Debug, thiserror::Error)]
pub enum LaunchError {
    #[error("application no longer exists")]
    NotFound,
    #[error("{0}")]
    Failed(String),
}

/// The platform boundary. Each platform enumerates, launches, reveals, and
/// renders icons for applications; everything above this is platform-neutral.
pub trait AppIndexer: Send + Sync {
    fn index(&self) -> Vec<IndexedApp>;
    fn launch(&self, app: &IndexedApp) -> Result<(), LaunchError>;
    fn reveal(&self, app: &IndexedApp) -> std::io::Result<()>;
    fn icon(&self, app: &IndexedApp, size: u32) -> Option<IconRgba>;
}

/// The in-memory application index. Loaded from the store at startup so results
/// are searchable immediately, rebuilt in the background, and mutated when an
/// entry proves stale.
pub struct AppIndex {
    apps: RwLock<Vec<IndexedApp>>,
    indexer: Arc<dyn AppIndexer>,
    store: Option<Arc<Store>>,
}

impl AppIndex {
    pub fn new(indexer: Arc<dyn AppIndexer>, store: Option<Arc<Store>>) -> Arc<Self> {
        let apps = store
            .as_ref()
            .and_then(|s| s.load_apps().ok())
            .map(|rows| {
                rows.into_iter()
                    .map(|(id, name, target)| IndexedApp { id, name, target })
                    .collect()
            })
            .unwrap_or_default();
        Arc::new(Self {
            apps: RwLock::new(apps),
            indexer,
            store,
        })
    }

    pub fn snapshot(&self) -> Vec<IndexedApp> {
        self.apps.read().unwrap().clone()
    }

    pub fn get(&self, id: &str) -> Option<IndexedApp> {
        self.apps
            .read()
            .unwrap()
            .iter()
            .find(|a| a.id == id)
            .cloned()
    }

    /// Re-enumerates and replaces the index, persisting the result. Picks up
    /// installs and uninstalls without a restart.
    pub fn rebuild(&self) {
        let apps = self.indexer.index();
        self.persist(&apps);
        *self.apps.write().unwrap() = apps;
    }

    pub fn remove(&self, id: &str) {
        self.apps.write().unwrap().retain(|a| a.id != id);
        if let Some(store) = &self.store {
            let _ = store.delete_app(id);
        }
    }

    pub fn launch(&self, app: &IndexedApp) -> Result<(), LaunchError> {
        self.indexer.launch(app)
    }

    pub fn reveal(&self, app: &IndexedApp) -> std::io::Result<()> {
        self.indexer.reveal(app)
    }

    fn persist(&self, apps: &[IndexedApp]) {
        if let Some(store) = &self.store {
            let rows: Vec<_> = apps
                .iter()
                .map(|a| (a.id.clone(), a.name.clone(), a.target.clone()))
                .collect();
            if let Err(error) = store.replace_apps(&rows) {
                eprintln!("[dango] could not persist application index: {error}");
            }
        }
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};

    pub struct FakeIndexer {
        apps: RwLock<Vec<IndexedApp>>,
        launch_not_found: AtomicBool,
        with_icons: bool,
    }

    impl FakeIndexer {
        pub fn with_apps(apps: &[(&str, &str, Option<&str>)]) -> Self {
            Self {
                apps: RwLock::new(
                    apps.iter()
                        .map(|(id, name, target)| IndexedApp {
                            id: (*id).into(),
                            name: (*name).into(),
                            target: target.map(|t| t.into()),
                        })
                        .collect(),
                ),
                launch_not_found: AtomicBool::new(false),
                with_icons: true,
            }
        }

        pub fn set_apps(&self, apps: &[(&str, &str, Option<&str>)]) {
            *self.apps.write().unwrap() = apps
                .iter()
                .map(|(id, name, target)| IndexedApp {
                    id: (*id).into(),
                    name: (*name).into(),
                    target: target.map(|t| t.into()),
                })
                .collect();
        }

        pub fn make_launch_fail_not_found(&self) {
            self.launch_not_found.store(true, Ordering::SeqCst);
        }

        pub fn without_icons(mut self) -> Self {
            self.with_icons = false;
            self
        }
    }

    impl AppIndexer for FakeIndexer {
        fn index(&self) -> Vec<IndexedApp> {
            self.apps.read().unwrap().clone()
        }
        fn launch(&self, _app: &IndexedApp) -> Result<(), LaunchError> {
            if self.launch_not_found.load(Ordering::SeqCst) {
                Err(LaunchError::NotFound)
            } else {
                Ok(())
            }
        }
        fn reveal(&self, _app: &IndexedApp) -> std::io::Result<()> {
            Ok(())
        }
        fn icon(&self, _app: &IndexedApp, size: u32) -> Option<IconRgba> {
            if !self.with_icons {
                return None;
            }
            Some(IconRgba {
                width: size,
                height: size,
                rgba: vec![255; (size * size * 4) as usize],
            })
        }
    }

    #[test]
    fn rebuild_populates_the_snapshot() {
        let indexer = Arc::new(FakeIndexer::with_apps(&[("a", "Alpha", None)]));
        let index = AppIndex::new(indexer, None);
        assert!(index.snapshot().is_empty());
        index.rebuild();
        assert_eq!(index.snapshot().len(), 1);
    }

    #[test]
    fn rebuild_drops_uninstalled_apps() {
        let indexer = Arc::new(FakeIndexer::with_apps(&[
            ("a", "Alpha", None),
            ("b", "Beta", None),
        ]));
        let index = AppIndex::new(indexer.clone(), None);
        index.rebuild();
        indexer.set_apps(&[("a", "Alpha", None)]);
        index.rebuild();
        assert_eq!(index.snapshot().len(), 1);
        assert!(index.get("b").is_none());
    }

    #[test]
    fn remove_takes_an_entry_out() {
        let indexer = Arc::new(FakeIndexer::with_apps(&[("a", "Alpha", None)]));
        let index = AppIndex::new(indexer, None);
        index.rebuild();
        index.remove("a");
        assert!(index.get("a").is_none());
    }

    #[test]
    fn a_persisted_index_is_available_before_the_first_rebuild() {
        let store = Arc::new(Store::in_memory().unwrap());
        let indexer = Arc::new(FakeIndexer::with_apps(&[("a", "Alpha", Some("/a"))]));

        let index = AppIndex::new(indexer.clone(), Some(store.clone()));
        index.rebuild();

        // A fresh index over the same store sees the persisted apps immediately.
        let reopened = AppIndex::new(indexer, Some(store));
        assert_eq!(reopened.snapshot().len(), 1);
        assert_eq!(reopened.get("a").unwrap().target.as_deref(), Some("/a"));
    }
}
