//! The `applications` built-in extension: the platform-neutral core that turns
//! an `AppIndexer` into searchable, launchable results with cached icons. The
//! per-platform indexers live in `crate::platform`.

mod icons;
mod index;

pub use icons::{IconCache, ICON_SIZE};
pub use index::{AppIndex, AppIndexer, IconRgba, IndexedApp, LaunchError};

use std::sync::Arc;

use async_trait::async_trait;

use crate::extension::{ActionOutcome, Extension, FormValues, Manifest, Service};
use crate::protocol::{Action, Modifier, Shortcut};
use crate::search::{Candidate, RootProvider, Source};

pub const EXTENSION_ID: &str = "dango.applications";

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
    fn perform(&self, app_id: &str, action: &str) -> ActionOutcome {
        let Some(app) = self.index.get(app_id) else {
            return ActionOutcome::Failed("application is no longer in the index".into());
        };
        match action {
            ACTION_LAUNCH => match self.index.launch(&app) {
                Ok(()) => ActionOutcome::Done,
                Err(LaunchError::NotFound) => {
                    self.index.remove(app_id);
                    ActionOutcome::Failed("application no longer exists".into())
                }
                Err(LaunchError::Failed(message)) => ActionOutcome::Failed(message),
            },
            ACTION_REVEAL => match self.index.reveal(&app) {
                Ok(()) => ActionOutcome::Done,
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

    fn perform_action(
        &self,
        item_id: &str,
        action_id: &str,
        _values: &FormValues,
    ) -> ActionOutcome {
        self.perform(item_id, action_id)
    }

    fn services(&self) -> Vec<Arc<dyn Service>> {
        vec![Arc::new(IndexingService {
            index: self.index.clone(),
            icons: self.icons.clone(),
            indexer: self.index.indexer(),
            running: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        })]
    }

    fn root_provider(&self) -> Option<Arc<dyn RootProvider>> {
        Some(self.provider.clone())
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
            .map(|app| {
                let has_path = app.target.is_some();
                Candidate {
                    extension_id: EXTENSION_ID.into(),
                    icon: self.icons.path(&app.id),
                    subtitle: app.target.clone(),
                    title: app.name,
                    id: app.id,
                    keywords: vec![],
                    alias: None,
                    source: Source::RootItem,
                    actions: app_actions(has_path),
                    match_positions: vec![],
                }
            })
            .collect()
    }
}

#[cfg(target_os = "macos")]
const REVEAL_TITLE: &str = "Reveal in Finder";
#[cfg(not(target_os = "macos"))]
const REVEAL_TITLE: &str = "Reveal in File Explorer";

/// The action panel for an application result. Launch is primary; reveal and
/// copy are offered only when the app has a filesystem path.
fn app_actions(has_path: bool) -> Vec<Action> {
    let mut actions = vec![Action {
        id: ACTION_LAUNCH.into(),
        title: "Launch".into(),
        shortcut: None,
    }];
    if has_path {
        actions.push(Action {
            id: ACTION_REVEAL.into(),
            title: REVEAL_TITLE.into(),
            shortcut: Some(Shortcut {
                key: "r".into(),
                modifiers: vec![Modifier::Ctrl],
            }),
        });
        actions.push(Action {
            id: ACTION_COPY_PATH.into(),
            title: "Copy Path".into(),
            shortcut: Some(Shortcut {
                key: "c".into(),
                modifiers: vec![Modifier::Ctrl],
            }),
        });
    }
    actions
}

/// The longest the index may go without being rebuilt, for platforms that
/// cannot signal a change and as a backstop for those that can.
const REFRESH_INTERVAL: std::time::Duration = std::time::Duration::from_secs(180);

/// How often the loop wakes to check whether it should stop or re-index.
const TICK: std::time::Duration = std::time::Duration::from_millis(200);

/// Installing an application writes many files; waiting a moment after the
/// first sign of change avoids re-indexing over a half-copied bundle.
const SETTLE: std::time::Duration = std::time::Duration::from_secs(1);

/// The background service: builds the index and extracts icons off the
/// activation path, so the launcher stays responsive, then re-indexes on a
/// schedule. Stops cleanly when the extension is disabled.
struct IndexingService {
    index: Arc<AppIndex>,
    icons: Arc<IconCache>,
    indexer: Arc<dyn AppIndexer>,
    running: Arc<std::sync::atomic::AtomicBool>,
}

impl Service for IndexingService {
    fn start(&self) -> crate::extension::ActivationResult {
        use std::sync::atomic::Ordering;
        self.running.store(true, Ordering::SeqCst);
        let index = self.index.clone();
        let icons = self.icons.clone();
        let indexer = self.indexer.clone();
        let running = self.running.clone();
        std::thread::spawn(move || {
            while running.load(Ordering::SeqCst) {
                index.rebuild();
                for app in index.snapshot() {
                    if !running.load(Ordering::SeqCst) {
                        return;
                    }
                    icons.ensure_with(&app.id, || indexer.icon(&app, ICON_SIZE));
                }
                wait_for_refresh(&index, &running);
            }
        });
        Ok(())
    }

    fn stop(&self) {
        self.running
            .store(false, std::sync::atomic::Ordering::SeqCst);
    }
}

/// Waits until the applications on disk look different or the sweep is due,
/// whichever comes first. Sleeps in short slices so disabling the extension
/// stops the loop promptly.
fn wait_for_refresh(index: &AppIndex, running: &std::sync::atomic::AtomicBool) {
    use std::sync::atomic::Ordering;
    let baseline = index.fingerprint();
    let started = std::time::Instant::now();
    while running.load(Ordering::SeqCst) && started.elapsed() < REFRESH_INTERVAL {
        std::thread::sleep(TICK);
        if baseline.is_some() && index.fingerprint() != baseline {
            std::thread::sleep(SETTLE);
            return;
        }
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
        let icons = IconCache::in_temp();
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
