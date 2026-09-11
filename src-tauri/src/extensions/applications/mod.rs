//! The `applications` built-in extension: the platform-neutral core that turns
//! an `AppIndexer` into searchable, launchable results with cached icons. The
//! per-platform indexers live in `crate::platform`.

mod icons;
mod index;

pub use icons::IconCache;
pub use index::{AppIndex, AppIndexer, IconRgba, IndexedApp, LaunchError};

use std::sync::Arc;

use async_trait::async_trait;

use crate::extension::{Extension, Manifest, Service};
use crate::search::{Candidate, RootProvider, Source};

pub const EXTENSION_ID: &str = "dango.applications";

/// What a chosen action produced. The frontend hides on a plain success, shows
/// the message on failure, and writes text to the clipboard for a copy action.
#[derive(Debug, PartialEq, Eq)]
pub enum ActionOutcome {
    Launched,
    Revealed,
    CopyToClipboard(String),
    Failed(String),
}

pub const ACTION_LAUNCH: &str = "launch";
pub const ACTION_REVEAL: &str = "reveal";
pub const ACTION_COPY_PATH: &str = "copy-path";

pub struct ApplicationsExtension {
    manifest: Manifest,
    index: Arc<AppIndex>,
    icons: Arc<IconCache>,
    provider: Arc<AppProvider>,
}

impl ApplicationsExtension {
    pub fn new(index: Arc<AppIndex>, icons: Arc<IconCache>) -> Self {
        let provider = Arc::new(AppProvider {
            index: index.clone(),
            icons: icons.clone(),
        });
        Self {
            manifest: Manifest {
                manifest_version: 1,
                id: EXTENSION_ID.into(),
                name: "Applications".into(),
                icon: None,
                commands: vec![],
                preferences: vec![],
                root_items: true,
                services: true,
            },
            index,
            icons,
            provider,
        }
    }

    pub fn provider(&self) -> Arc<dyn RootProvider> {
        self.provider.clone()
    }

    /// Runs a chosen action, removing an entry that turns out to be gone.
    pub fn perform(&self, app_id: &str, action: &str) -> ActionOutcome {
        let Some(app) = self.index.get(app_id) else {
            return ActionOutcome::Failed("application is no longer in the index".into());
        };
        match action {
            ACTION_LAUNCH => match self.index.launch(&app) {
                Ok(()) => ActionOutcome::Launched,
                Err(LaunchError::NotFound) => {
                    self.index.remove(app_id);
                    ActionOutcome::Failed("application no longer exists".into())
                }
                Err(LaunchError::Failed(message)) => ActionOutcome::Failed(message),
            },
            ACTION_REVEAL => match self.index.reveal(&app) {
                Ok(()) => ActionOutcome::Revealed,
                Err(error) => ActionOutcome::Failed(error.to_string()),
            },
            ACTION_COPY_PATH => match app.target {
                Some(path) => ActionOutcome::CopyToClipboard(path),
                None => ActionOutcome::Failed("this application has no file path".into()),
            },
            other => ActionOutcome::Failed(format!("unknown action '{other}'")),
        }
    }
}

impl Extension for ApplicationsExtension {
    fn manifest(&self) -> &Manifest {
        &self.manifest
    }

    fn services(&self) -> Vec<Arc<dyn Service>> {
        vec![Arc::new(IndexingService {
            index: self.index.clone(),
            icons: self.icons.clone(),
            running: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        })]
    }
}

/// Feeds every indexed application into the search pipeline. Filtering and
/// ranking happen downstream, so the provider ignores the query and returns the
/// current snapshot, which is cheap because it is an in-memory clone.
struct AppProvider {
    index: Arc<AppIndex>,
    icons: Arc<IconCache>,
}

#[async_trait]
impl RootProvider for AppProvider {
    async fn items(&self, _query: String) -> Vec<Candidate> {
        self.index
            .snapshot()
            .into_iter()
            .map(|app| Candidate {
                icon: self.icons.path(&app.id),
                subtitle: app.target.clone(),
                title: app.name,
                id: app.id,
                keywords: vec![],
                alias: None,
                source: Source::RootItem,
                match_positions: vec![],
            })
            .collect()
    }
}

/// How often the index is rebuilt so installs and uninstalls appear without a
/// restart. A plain schedule rather than a native change watcher, which is
/// enough for a personal launcher.
const REFRESH_INTERVAL: std::time::Duration = std::time::Duration::from_secs(180);

/// The background service: builds the index and extracts icons off the
/// activation path, so the launcher stays responsive, then re-indexes on a
/// schedule. Stops cleanly when the extension is disabled.
struct IndexingService {
    index: Arc<AppIndex>,
    icons: Arc<IconCache>,
    running: Arc<std::sync::atomic::AtomicBool>,
}

impl Service for IndexingService {
    fn start(&self) -> crate::extension::ActivationResult {
        use std::sync::atomic::Ordering;
        self.running.store(true, Ordering::SeqCst);
        let index = self.index.clone();
        let icons = self.icons.clone();
        let running = self.running.clone();
        std::thread::spawn(move || {
            while running.load(Ordering::SeqCst) {
                index.rebuild();
                for app in index.snapshot() {
                    if !running.load(Ordering::SeqCst) {
                        return;
                    }
                    icons.ensure(&app);
                }
                // Sleep in short slices so disabling stops the loop promptly.
                let mut waited = std::time::Duration::ZERO;
                while waited < REFRESH_INTERVAL && running.load(Ordering::SeqCst) {
                    std::thread::sleep(std::time::Duration::from_millis(200));
                    waited += std::time::Duration::from_millis(200);
                }
            }
        });
        Ok(())
    }

    fn stop(&self) {
        self.running
            .store(false, std::sync::atomic::Ordering::SeqCst);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use index::tests::FakeIndexer;

    fn extension() -> (ApplicationsExtension, Arc<FakeIndexer>) {
        let indexer = Arc::new(FakeIndexer::with_apps(&[(
            "code",
            "VS Code",
            Some("/a/code"),
        )]));
        let index = AppIndex::new(indexer.clone(), None);
        index.rebuild();
        let icons = IconCache::in_temp(indexer.clone());
        (ApplicationsExtension::new(index, icons), indexer)
    }

    #[test]
    fn manifest_contributes_root_items_and_services() {
        let (ext, _) = extension();
        assert!(ext.manifest().root_items);
        assert!(ext.manifest().services);
        assert!(ext.manifest().validate().is_ok());
    }

    #[tokio::test]
    async fn the_provider_returns_indexed_apps() {
        let (ext, _) = extension();
        let items = ext.provider().items(String::new()).await;
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].title, "VS Code");
        assert_eq!(items[0].subtitle.as_deref(), Some("/a/code"));
    }

    #[test]
    fn copy_path_returns_the_target() {
        let (ext, _) = extension();
        assert_eq!(
            ext.perform("code", ACTION_COPY_PATH),
            ActionOutcome::CopyToClipboard("/a/code".into())
        );
    }

    #[test]
    fn launching_a_vanished_app_removes_it() {
        let (ext, indexer) = extension();
        indexer.make_launch_fail_not_found();
        let outcome = ext.perform("code", ACTION_LAUNCH);
        assert!(matches!(outcome, ActionOutcome::Failed(_)));
        assert!(ext.index.get("code").is_none());
    }

    #[test]
    fn an_unknown_action_fails_cleanly() {
        let (ext, _) = extension();
        assert!(matches!(
            ext.perform("code", "nope"),
            ActionOutcome::Failed(_)
        ));
    }
}
