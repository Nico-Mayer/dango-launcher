//! The `dango.updates` built-in extension: finds a newer release, shows it at
//! the top of root search, and installs it when the user confirms.

mod source;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};

use async_trait::async_trait;
use bytesize::ByteSize;
use tokio::runtime::Handle;

use crate::extension::{
    ActionOutcome, CommandDecl, Extension, FormValues, InvocationMode, Manifest, NAMED_ICON,
};
use crate::invocation::{Command, InvocationContext, Runner};
use crate::protocol::{Action, DetailView, View, ViewTree};
use crate::search::{Candidate, RootProvider, Source};

pub use source::{production, DisabledSource, UpdateError, UpdateSource};

pub const EXTENSION_ID: &str = "dango.updates";

pub const COMMAND_CHECK: &str = "check";
pub const ACTION_INSTALL: &str = "install";

const ITEM_ID: &str = "update";

const FIRST_CHECK_AFTER: Duration = Duration::from_secs(30);
const CHECK_EVERY: Duration = Duration::from_secs(6 * 60 * 60);
const WAKE_EVERY: Duration = Duration::from_secs(10 * 60);

const CHECKING: &str = "Checking for updates";
const UP_TO_DATE: &str = "You have the latest version";
const DEVELOPMENT: &str = "Updates are off in development builds.";

pub(crate) fn manifest() -> Manifest {
    Manifest {
        manifest_version: 1,
        id: EXTENSION_ID.into(),
        name: "Updates".into(),
        icon: None,
        tint: Some("green".into()),
        commands: vec![CommandDecl {
            id: COMMAND_CHECK.into(),
            title: "Check for Updates".into(),
            mode: InvocationMode::View,
            subtitle: Some("Dango".into()),
            icon: Some(format!("{NAMED_ICON}refresh-cw")),
            keywords: vec!["update".into(), "upgrade".into(), "version".into()],
            alias: None,
        }],
        preferences: vec![],
        root_items: true,
        services: false,
    }
}

pub struct UpdatesExtension {
    manifest: Manifest,
    inner: Arc<Inner>,
}

struct Inner {
    source: Arc<dyn UpdateSource>,
    current: String,
    /// False in a development build, which never asks for an update.
    enabled: bool,
    runtime: Handle,
    runner: Option<Arc<dyn Runner>>,
    available: Mutex<Option<String>>,
    installing: AtomicBool,
    /// Where the install shows its progress. Replaced when the user starts the
    /// install again while it runs, so reopening it finds the download.
    view: Mutex<Option<Arc<InvocationContext>>>,
    shown: Mutex<Option<ViewTree>>,
}

impl UpdatesExtension {
    pub fn new(
        source: Arc<dyn UpdateSource>,
        current: impl Into<String>,
        enabled: bool,
        runtime: Handle,
        runner: Option<Arc<dyn Runner>>,
    ) -> Self {
        Self {
            manifest: manifest(),
            inner: Arc::new(Inner {
                source,
                current: current.into(),
                enabled,
                runtime,
                runner,
                available: Mutex::new(None),
                installing: AtomicBool::new(false),
                view: Mutex::new(None),
                shown: Mutex::new(None),
            }),
        }
    }

    /// Checks shortly after startup and then every few hours. Starts nothing
    /// in a development build, and says whether it started.
    pub fn schedule(&self) -> bool {
        if !self.inner.enabled {
            return false;
        }
        let inner = self.inner.clone();
        self.inner.runtime.spawn(async move {
            tokio::time::sleep(FIRST_CHECK_AFTER).await;
            loop {
                if let Err(error) = inner.check().await {
                    crate::log::append(&format!("updates: {}", error.detail()));
                }
                // Measured on the wall clock, because a monotonic sleep stops
                // while the machine sleeps and would push the next check back.
                let checked = SystemTime::now();
                while checked.elapsed().is_ok_and(|elapsed| elapsed < CHECK_EVERY) {
                    tokio::time::sleep(WAKE_EVERY).await;
                }
            }
        });
        true
    }
}

impl Extension for UpdatesExtension {
    fn manifest(&self) -> &Manifest {
        &self.manifest
    }

    fn root_provider(&self) -> Option<Arc<dyn RootProvider>> {
        Some(Arc::new(UpdateProvider(self.inner.clone())))
    }

    fn command(&self, command_id: &str) -> Option<Arc<dyn Command>> {
        (command_id == COMMAND_CHECK).then(|| Arc::new(Check(self.inner.clone())) as _)
    }

    fn perform_action(
        &self,
        _item_id: &str,
        action_id: &str,
        _values: &FormValues,
    ) -> ActionOutcome {
        match action_id {
            ACTION_INSTALL => self.inner.start_install(),
            _ => ActionOutcome::Failed("That action isn't available.".into()),
        }
    }
}

impl Inner {
    fn available(&self) -> Option<String> {
        self.available.lock().unwrap().clone()
    }

    async fn check(&self) -> Result<Option<String>, UpdateError> {
        let found = self.source.check().await?;
        self.available.lock().unwrap().clone_from(&found);
        Ok(found)
    }

    async fn check_view(&self, ctx: &InvocationContext) {
        if !self.enabled {
            ctx.push_view(detail(DEVELOPMENT.into(), false, vec![]));
            return;
        }
        ctx.push_view(detail(CHECKING.into(), true, vec![]));
        let tree = match self.check().await {
            Ok(Some(version)) => detail(
                format!("{}\n\nYou have {}", available_title(&version), self.current),
                false,
                vec![install_action()],
            ),
            Ok(None) => detail(
                format!("{UP_TO_DATE}\n\nDango {}", self.current),
                false,
                vec![],
            ),
            Err(error) => {
                crate::log::append(&format!("updates: {}", error.detail()));
                detail(error.to_string(), false, vec![])
            }
        };
        ctx.push_view(tree);
    }

    fn start_install(self: &Arc<Self>) -> ActionOutcome {
        let Some(version) = self.available() else {
            return ActionOutcome::Failed("That action isn't available.".into());
        };
        let Some(ctx) = self.runner.as_ref().and_then(|runner| runner.start()) else {
            return ActionOutcome::Failed("The update can't start right now. Try again.".into());
        };
        if let Some(tree) = self.shown.lock().unwrap().clone() {
            ctx.push_view(tree);
        }
        *self.view.lock().unwrap() = Some(ctx);
        if !self.installing.swap(true, Ordering::SeqCst) {
            let inner = self.clone();
            self.runtime
                .spawn(async move { inner.install(&version).await });
        }
        ActionOutcome::Started
    }

    /// Runs to the end even when the launcher is dismissed: only what it shows
    /// is dropped then, not the download.
    async fn install(&self, version: &str) {
        self.show(detail(downloading_title(version), true, vec![]));
        let mut received = 0;
        let mut shown = 0;
        let downloaded = self
            .source
            .download(&mut |chunk, total| {
                received += chunk;
                if received - shown >= progress_step(total) {
                    shown = received;
                    self.show(downloading(version, received, total));
                }
            })
            .await;
        let installed = downloaded.and_then(|bytes| self.source.install(&bytes));
        if let Err(error) = installed {
            crate::log::append(&format!("updates: {}", error.detail()));
            self.show(detail(error.to_string(), false, vec![]));
        }
        self.installing.store(false, Ordering::SeqCst);
        self.shown.lock().unwrap().take();
    }

    fn show(&self, tree: ViewTree) {
        *self.shown.lock().unwrap() = Some(tree.clone());
        if let Some(ctx) = self.view.lock().unwrap().as_ref() {
            ctx.push_view(tree);
        }
    }
}

/// A view per twentieth of the download, or per megabyte when the size is
/// unknown, rather than one per network chunk.
fn progress_step(total: Option<u64>) -> u64 {
    total.map_or(1_000_000, |total| (total / 20).max(1))
}

fn available_title(version: &str) -> String {
    format!("Dango {version} is available")
}

fn install_action() -> Action {
    Action {
        id: ACTION_INSTALL.into(),
        title: "Install and restart".into(),
        shortcut: None,
    }
}

fn downloading(version: &str, received: u64, total: Option<u64>) -> ViewTree {
    let size = |bytes| ByteSize::b(bytes).display().si().to_string();
    let progress = match total {
        Some(total) => format!("{} of {}", size(received), size(total)),
        None => size(received),
    };
    detail(
        format!("{}\n\n{progress}", downloading_title(version)),
        true,
        vec![],
    )
}

fn downloading_title(version: &str) -> String {
    format!("Downloading Dango {version}")
}

fn detail(markdown: String, loading: bool, actions: Vec<Action>) -> ViewTree {
    ViewTree::new(View::Detail(DetailView {
        markdown,
        loading,
        actions,
    }))
}

struct UpdateProvider(Arc<Inner>);

#[async_trait]
impl RootProvider for UpdateProvider {
    async fn items(&self, _query: String) -> Vec<Candidate> {
        let Some(version) = self.0.available() else {
            return vec![];
        };
        vec![Candidate {
            extension_id: EXTENSION_ID.into(),
            id: ITEM_ID.into(),
            title: available_title(&version),
            subtitle: Some(format!("You have {}", self.0.current)),
            icon: Some(format!("{NAMED_ICON}circle-arrow-down")),
            keywords: vec!["update".into(), "dango".into()],
            alias: None,
            source: Source::RootItem,
            actions: vec![install_action()],
            match_positions: vec![],
            suggested: false,
            pinned: true,
        }]
    }
}

struct Check(Arc<Inner>);

impl Command for Check {
    fn invoke(&self, ctx: &InvocationContext) {
        self.0.runtime.block_on(self.0.check_view(ctx));
    }
}

#[cfg(test)]
mod tests {
    use super::source::testing::FakeSource;
    use super::*;
    use crate::invocation::{Outcome, Sink};

    #[derive(Default)]
    struct Recorded(Mutex<Vec<ViewTree>>);

    impl Sink for Recorded {
        fn push_view(&self, tree: ViewTree) {
            self.0.lock().unwrap().push(tree);
        }
        fn finish(&self, _outcome: Outcome) {}
    }

    impl Recorded {
        fn details(&self) -> Vec<DetailView> {
            self.0
                .lock()
                .unwrap()
                .iter()
                .filter_map(|tree| match &tree.view {
                    View::Detail(detail) => Some(detail.clone()),
                    _ => None,
                })
                .collect()
        }

        fn last(&self) -> DetailView {
            self.details().pop().expect("a view was shown")
        }
    }

    struct FixedRunner(Arc<Recorded>);

    impl Runner for FixedRunner {
        fn start(&self) -> Option<Arc<InvocationContext>> {
            Some(Arc::new(InvocationContext::for_test(
                EXTENSION_ID,
                self.0.clone(),
            )))
        }
    }

    struct Fixture {
        extension: UpdatesExtension,
        source: Arc<FakeSource>,
        views: Arc<Recorded>,
    }

    fn fixture(source: FakeSource) -> Fixture {
        build(source, true)
    }

    fn build(source: FakeSource, enabled: bool) -> Fixture {
        let source = Arc::new(source);
        let views = Arc::new(Recorded::default());
        let extension = UpdatesExtension::new(
            source.clone(),
            "0.4.0",
            enabled,
            Handle::current(),
            Some(Arc::new(FixedRunner(views.clone()))),
        );
        Fixture {
            extension,
            source,
            views,
        }
    }

    impl Fixture {
        async fn items(&self) -> Vec<Candidate> {
            self.extension
                .root_provider()
                .unwrap()
                .items(String::new())
                .await
        }

        async fn check_view(&self) -> Arc<Recorded> {
            let sink = Arc::new(Recorded::default());
            let ctx = InvocationContext::for_test(COMMAND_CHECK, sink.clone());
            self.extension.inner.check_view(&ctx).await;
            sink
        }

        async fn install_to_the_end(&self) {
            self.extension.inner.check().await.unwrap();
            let outcome =
                self.extension
                    .perform_action(ITEM_ID, ACTION_INSTALL, &FormValues::new());
            assert_eq!(outcome, ActionOutcome::Started);
            while self.extension.inner.installing.load(Ordering::SeqCst) {
                tokio::task::yield_now().await;
            }
        }
    }

    #[tokio::test]
    async fn disabled_source_never_reports_an_update() {
        let extension = UpdatesExtension::new(
            Arc::new(DisabledSource),
            "0.4.0",
            true,
            Handle::current(),
            None,
        );
        extension.inner.check().await.unwrap();
        let items = extension
            .root_provider()
            .unwrap()
            .items(String::new())
            .await;
        assert!(items.is_empty());
    }

    #[tokio::test]
    async fn no_update_lists_nothing() {
        let fixture = fixture(FakeSource::default());
        fixture.extension.inner.check().await.unwrap();
        assert!(fixture.items().await.is_empty());
    }

    #[tokio::test]
    async fn an_update_is_one_pinned_item_naming_both_versions() {
        let fixture = fixture(FakeSource::finding("0.5.0"));
        fixture.extension.inner.check().await.unwrap();

        let items = fixture.items().await;
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].title, "Dango 0.5.0 is available");
        assert_eq!(items[0].subtitle.as_deref(), Some("You have 0.4.0"));
        assert!(items[0].pinned);
        assert!(items[0].keywords.contains(&"update".to_string()));
        assert!(items[0].keywords.contains(&"dango".to_string()));
        assert_eq!(items[0].actions[0].title, "Install and restart");
    }

    #[tokio::test]
    async fn the_update_follows_a_permission_warning() {
        use crate::extension::{EnabledStore, ExtensionHost};
        use crate::extensions::permissions::PermissionsExtension;
        use crate::extensions::snippets::OpenUrl;
        use crate::platform::{Permission, Permissions};
        use crate::ranking::{DangoRanker, FrecencyTable, InputHistory};
        use crate::search::{SearchPipeline, StaticCommandSource};

        struct AllEnabled;
        impl EnabledStore for AllEnabled {
            fn is_enabled(&self, _id: &str) -> bool {
                true
            }
            fn set_enabled(&self, _id: &str, _enabled: bool) {}
        }
        struct AccessibilityOff;
        impl Permissions for AccessibilityOff {
            fn missing(&self) -> Vec<Permission> {
                vec![Permission::Accessibility]
            }
            fn request(&self, _permission: Permission) {}
            fn settings_url(&self, _permission: Permission) -> Option<&'static str> {
                None
            }
        }
        struct NoOpener;
        impl OpenUrl for NoOpener {
            fn open(&self, _url: &str) -> Result<(), String> {
                Ok(())
            }
        }

        let fixture = fixture(FakeSource::finding("0.5.0"));
        fixture.extension.inner.check().await.unwrap();
        let mut host = ExtensionHost::new(Arc::new(AllEnabled));
        host.register(Arc::new(PermissionsExtension::new(
            Arc::new(AccessibilityOff),
            Arc::new(NoOpener),
        )))
        .unwrap();
        host.register(Arc::new(fixture.extension)).unwrap();

        let pipeline = SearchPipeline::new(
            Arc::new(StaticCommandSource(host.command_candidates())),
            host.root_providers(),
            Arc::new(DangoRanker::new(
                Arc::new(FrecencyTable::in_memory()),
                Arc::new(InputHistory::in_memory()),
            )),
            10,
        );
        let mut results = pipeline.query(String::new());
        let mut last = None;
        while let Some(update) = results.recv().await {
            let complete = update.complete;
            last = Some(update);
            if complete {
                break;
            }
        }
        let ids: Vec<String> = last.unwrap().items.into_iter().map(|c| c.id).collect();
        assert_eq!(ids[..2], ["accessibility", ITEM_ID]);
    }

    #[tokio::test]
    async fn a_failed_check_keeps_what_was_found_before() {
        let fixture = fixture(FakeSource::finding("0.5.0"));
        fixture.extension.inner.check().await.unwrap();

        *fixture.source.check_error.lock().unwrap() = Some(UpdateError::Check("offline".into()));
        assert!(fixture.extension.inner.check().await.is_err());
        assert_eq!(fixture.items().await.len(), 1);
    }

    #[tokio::test]
    async fn dismissing_the_launcher_does_not_stop_the_install() {
        struct Dismissed;
        impl Sink for Dismissed {
            fn push_view(&self, _tree: ViewTree) {}
            fn finish(&self, _outcome: Outcome) {}
            fn superseded(&self) -> bool {
                true
            }
        }
        struct DismissedRunner;
        impl Runner for DismissedRunner {
            fn start(&self) -> Option<Arc<InvocationContext>> {
                Some(Arc::new(InvocationContext::for_test(
                    EXTENSION_ID,
                    Arc::new(Dismissed),
                )))
            }
        }

        let source = Arc::new(FakeSource::finding("0.5.0"));
        let extension = UpdatesExtension::new(
            source.clone(),
            "0.4.0",
            true,
            Handle::current(),
            Some(Arc::new(DismissedRunner)),
        );
        extension.inner.check().await.unwrap();
        extension.inner.start_install();
        while extension.inner.installing.load(Ordering::SeqCst) {
            tokio::task::yield_now().await;
        }
        assert_eq!(source.installed.lock().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn installing_shows_the_download_then_installs_it() {
        let fixture = fixture(FakeSource {
            chunks: vec![10_000_000, 10_000_000],
            total: Some(20_000_000),
            ..FakeSource::finding("0.5.0")
        });
        fixture.install_to_the_end().await;

        let views = fixture.views.details();
        assert_eq!(views[0].markdown, "Downloading Dango 0.5.0");
        assert!(views.iter().all(|view| view.loading));
        assert!(views
            .iter()
            .any(|view| view.markdown.ends_with("10.0 MB of 20.0 MB")));
        assert_eq!(
            *fixture.source.installed.lock().unwrap(),
            [b"update".to_vec()]
        );
    }

    #[tokio::test]
    async fn a_bad_signature_installs_nothing_and_says_so() {
        let fixture = fixture(FakeSource {
            download_error: Some(UpdateError::Signature("bad signature".into())),
            ..FakeSource::finding("0.5.0")
        });
        fixture.install_to_the_end().await;

        let last = fixture.views.last();
        assert_eq!(
            last.markdown,
            "Couldn't verify the update, so it wasn't installed. Try again later."
        );
        assert!(!last.loading);
        assert!(fixture.source.installed.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn a_failed_download_installs_nothing_and_keeps_the_update() {
        let fixture = fixture(FakeSource {
            download_error: Some(UpdateError::Download("reset by peer".into())),
            ..FakeSource::finding("0.5.0")
        });
        fixture.install_to_the_end().await;

        assert_eq!(
            fixture.views.last().markdown,
            "Couldn't download the update. Check your connection and try again."
        );
        assert!(fixture.source.installed.lock().unwrap().is_empty());
        assert_eq!(fixture.items().await.len(), 1);
    }

    #[tokio::test]
    async fn nothing_to_install_is_refused() {
        let fixture = fixture(FakeSource::default());
        let outcome = fixture
            .extension
            .perform_action(ITEM_ID, ACTION_INSTALL, &FormValues::new());
        assert!(matches!(outcome, ActionOutcome::Failed(_)));
    }

    #[tokio::test]
    async fn checking_shows_progress_then_the_update() {
        let fixture = fixture(FakeSource::finding("0.5.0"));
        let views = fixture.check_view().await.details();

        assert_eq!(views[0].markdown, "Checking for updates");
        assert!(views[0].loading);
        let last = views.last().unwrap();
        assert_eq!(last.markdown, "Dango 0.5.0 is available\n\nYou have 0.4.0");
        assert_eq!(last.actions[0].title, "Install and restart");
        assert_eq!(fixture.items().await.len(), 1);
    }

    #[tokio::test]
    async fn checking_when_up_to_date_names_the_running_version() {
        let fixture = fixture(FakeSource::default());
        let last = fixture.check_view().await.last();
        assert_eq!(last.markdown, "You have the latest version\n\nDango 0.4.0");
        assert!(last.actions.is_empty());
    }

    #[tokio::test]
    async fn a_failed_check_says_so() {
        let fixture = fixture(FakeSource {
            check_error: Mutex::new(Some(UpdateError::Check("offline".into()))),
            ..Default::default()
        });
        assert_eq!(
            fixture.check_view().await.last().markdown,
            "Couldn't check for updates. Check your connection and try again."
        );
    }

    #[tokio::test]
    async fn a_development_build_never_asks() {
        let fixture = build(FakeSource::finding("0.5.0"), false);
        assert!(!fixture.extension.schedule());
        assert_eq!(
            fixture.check_view().await.last().markdown,
            "Updates are off in development builds."
        );
        assert_eq!(fixture.source.checks(), 0);
    }

    #[test]
    fn the_manifest_validates() {
        assert!(manifest().validate().is_ok());
    }
}
