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
    ActionOutcome, ActivationResult, CommandDecl, Extension, FormValues, InvocationMode, Manifest,
    Service, NAMED_ICON,
};
use crate::invocation::{Command, InvocationContext, Runner};
use crate::protocol::{Action, DetailView, Modifier, Shortcut, Status, View, ViewTree};
use crate::search::{Candidate, RootProvider, Source};
use crate::tasks::{self, TaskHandle, TaskState, TaskStatus, Tasks};

pub use source::{production, UpdateError, UpdateSource};

pub const EXTENSION_ID: &str = "dango.updates";

pub const COMMAND_CHECK: &str = "check";
pub const ACTION_INSTALL: &str = "install";
const ACTION_SHOW: &str = "show";
const ACTION_CANCEL: &str = "cancel";
const ACTION_BACKGROUND: &str = "background";
const ACTION_RESTART: &str = "restart";
const ACTION_LATER: &str = "later";

const ITEM_ID: &str = "update";

const FIRST_CHECK_AFTER: Duration = Duration::from_secs(30);
const CHECK_EVERY: Duration = Duration::from_secs(6 * 60 * 60);
const WAKE_EVERY: Duration = Duration::from_secs(10 * 60);

const CHECKING: &str = "Checking for updates";
const UP_TO_DATE: &str = "You have the latest version";
const DEVELOPMENT: &str = "Updates are off in development builds.";
const STEPS: [&str; 3] = ["Download", "Verify signature", "Restart"];
const READY: &str = "Ready to restart";

pub(crate) fn manifest() -> Manifest {
    Manifest {
        manifest_version: 1,
        id: EXTENSION_ID.into(),
        name: "Updates".into(),
        icon: None,
        commands: vec![CommandDecl {
            id: COMMAND_CHECK.into(),
            title: "Check for Updates".into(),
            mode: InvocationMode::View,
            icon: Some(format!("{NAMED_ICON}refresh-cw")),
            keywords: vec!["update".into(), "upgrade".into(), "version".into()],
            alias: None,
        }],
        root_items: true,
        services: true,
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
    tasks: Arc<Tasks>,
    available: Mutex<Option<String>>,
    /// The download in progress or waiting to restart.
    task: Mutex<Option<TaskHandle>>,
    /// A verified download, kept until "Restart now".
    ready: Mutex<Option<Vec<u8>>>,
    /// Where the task shows its progress. Replaced when the user opens it
    /// again, so reopening it finds the download.
    view: Mutex<Option<Arc<InvocationContext>>>,
}

impl UpdatesExtension {
    pub fn new(
        source: Arc<dyn UpdateSource>,
        current: impl Into<String>,
        enabled: bool,
        runtime: Handle,
        runner: Option<Arc<dyn Runner>>,
        tasks: Arc<Tasks>,
    ) -> Self {
        Self {
            manifest: manifest(),
            inner: Arc::new(Inner {
                source,
                current: current.into(),
                enabled,
                runtime,
                runner,
                tasks,
                available: Mutex::new(None),
                task: Mutex::new(None),
                ready: Mutex::new(None),
                view: Mutex::new(None),
            }),
        }
    }
}

/// Checks shortly after activation and then every few hours, until stopped.
/// Starts nothing in a development build.
struct Schedule {
    inner: Arc<Inner>,
    stopped: Arc<AtomicBool>,
}

impl Service for Schedule {
    fn start(&self) -> ActivationResult {
        if !self.inner.enabled {
            return Ok(());
        }
        let inner = self.inner.clone();
        let stopped = self.stopped.clone();
        self.inner.runtime.spawn(async move {
            tokio::time::sleep(FIRST_CHECK_AFTER).await;
            while !stopped.load(Ordering::SeqCst) {
                if let Err(error) = inner.check().await {
                    crate::log::append(&format!("updates: {}", error.detail()));
                }
                // Measured on the wall clock, because a monotonic sleep stops
                // while the machine sleeps and would push the next check back.
                let checked = SystemTime::now();
                while !stopped.load(Ordering::SeqCst)
                    && checked.elapsed().is_ok_and(|elapsed| elapsed < CHECK_EVERY)
                {
                    tokio::time::sleep(WAKE_EVERY).await;
                }
            }
        });
        Ok(())
    }

    fn stop(&self) {
        self.stopped.store(true, Ordering::SeqCst);
    }
}

impl Extension for UpdatesExtension {
    fn manifest(&self) -> &Manifest {
        &self.manifest
    }

    fn services(&self) -> Vec<Arc<dyn Service>> {
        vec![Arc::new(Schedule {
            inner: self.inner.clone(),
            stopped: Arc::new(AtomicBool::new(false)),
        })]
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
            ACTION_INSTALL | ACTION_SHOW => self.inner.start_install(),
            ACTION_CANCEL => self.inner.cancel(),
            ACTION_BACKGROUND | ACTION_LATER => ActionOutcome::Back,
            ACTION_RESTART => self.inner.restart(),
            _ => ActionOutcome::Failed("That action isn't available".into()),
        }
    }
}

impl Inner {
    fn available(&self) -> Option<String> {
        self.available.lock().unwrap().clone()
    }

    fn task_state(&self) -> Option<TaskState> {
        self.task.lock().unwrap().as_ref()?.state()
    }

    /// A download that runs or waits keeps the version it found: a newer check
    /// would change what "Restart now" installs.
    async fn check(&self) -> Result<Option<String>, UpdateError> {
        if self.task_state().is_some() {
            return Ok(self.available());
        }
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

    /// Starts the download, or shows the one that runs or waits.
    fn start_install(self: &Arc<Self>) -> ActionOutcome {
        let Some(version) = self.available() else {
            return ActionOutcome::Failed("That action isn't available".into());
        };
        let Some(ctx) = self.runner.as_ref().and_then(|runner| runner.start()) else {
            return ActionOutcome::Failed("The update can't start right now".into());
        };
        *self.view.lock().unwrap() = Some(ctx);
        if let Some(state) = self.task_state() {
            self.show(&state);
            return ActionOutcome::Started;
        }
        let task = self
            .tasks
            .start(EXTENSION_ID, ITEM_ID, &format!("Dango {version}"));
        let state = task.report(|state| {
            state.subtitle = format!("You have {}", self.current);
            state.line = downloading_line(&version, None);
            state.steps = tasks::steps(&STEPS, 0, Status::Working);
            state.cancel = Some(ACTION_CANCEL.into());
            state.retry = Some(ACTION_INSTALL.into());
        });
        if let Some(state) = state {
            self.show(&state);
        }
        *self.task.lock().unwrap() = Some(task.clone());
        let inner = self.clone();
        self.runtime
            .spawn(async move { inner.download(task, &version).await });
        ActionOutcome::Started
    }

    /// Runs to the end even when the launcher is dismissed: only what it shows
    /// is dropped then, not the download.
    async fn download(&self, task: TaskHandle, version: &str) {
        let mut received = 0;
        let mut shown = 0;
        let mut progress = |chunk: u64, total: Option<u64>| {
            received += chunk;
            if received - shown >= progress_step(total) {
                shown = received;
                let state = task.report(|state| {
                    state.progress = total.map(|total| received as f32 / total.max(1) as f32);
                    state.detail = bytes_line(received, total);
                    state.line = downloading_line(version, state.progress);
                });
                if let Some(state) = state {
                    self.show(&state);
                }
            }
        };
        let mut cancelled = task.cancelled();
        let downloaded = tokio::select! {
            downloaded = self.source.download(&mut progress) => downloaded,
            _ = cancelled.wait_for(|cancelled| *cancelled) => {
                task.report(|state| state.status = TaskStatus::Cancelled);
                self.task.lock().unwrap().take();
                return;
            }
        };
        let state = match downloaded {
            Ok(bytes) => {
                *self.ready.lock().unwrap() = Some(bytes);
                task.report(|state| {
                    state.status = TaskStatus::NeedsInput {
                        sentence: format!("Dango {version} is ready to restart"),
                    };
                    state.progress = Some(1.0);
                    state.steps = tasks::steps(&STEPS, 2, Status::NeedsInput);
                    state.detail = READY.into();
                })
            }
            Err(error) => {
                crate::log::append(&format!("updates: {}", error.detail()));
                let failed = match error {
                    UpdateError::Signature(_) => 1,
                    _ => 0,
                };
                let state = task.report(|state| {
                    state.status = TaskStatus::Failed {
                        sentence: error.to_string(),
                    };
                    state.steps = tasks::steps(&STEPS, failed, Status::Failed);
                    state.detail = error.to_string();
                });
                self.task.lock().unwrap().take();
                state
            }
        };
        if let Some(state) = state {
            self.show(&state);
        }
    }

    fn cancel(&self) -> ActionOutcome {
        if let Some(task) = self.task.lock().unwrap().as_ref() {
            task.cancel();
        }
        ActionOutcome::Back
    }

    fn restart(&self) -> ActionOutcome {
        let Some(bytes) = self.ready.lock().unwrap().take() else {
            return ActionOutcome::Failed("That action isn't available".into());
        };
        // On Windows this exits, and the installer starts Dango again.
        match self.source.install(&bytes) {
            Ok(()) => ActionOutcome::Done,
            Err(error) => {
                crate::log::append(&format!("updates: {}", error.detail()));
                *self.ready.lock().unwrap() = Some(bytes);
                ActionOutcome::Failed(error.to_string())
            }
        }
    }

    fn show(&self, state: &TaskState) {
        if let Some(ctx) = self.view.lock().unwrap().as_ref() {
            ctx.push_view(tasks::progress_view(state, actions_for(&state.status)));
        }
    }
}

fn actions_for(status: &TaskStatus) -> Vec<Action> {
    let action = |id: &str, title: &str| Action {
        id: id.into(),
        title: title.into(),
        ..Default::default()
    };
    match status {
        TaskStatus::Running => vec![
            action(ACTION_BACKGROUND, "Continue in background"),
            Action {
                shortcut: Some(Shortcut {
                    key: ".".into(),
                    modifiers: vec![Modifier::Cmd],
                }),
                ..action(ACTION_CANCEL, "Cancel")
            },
        ],
        TaskStatus::NeedsInput { .. } => vec![
            action(ACTION_RESTART, "Restart now"),
            action(ACTION_LATER, "Install later"),
        ],
        TaskStatus::Failed { .. } => vec![action(ACTION_INSTALL, "Retry")],
        TaskStatus::Done { .. } | TaskStatus::Cancelled => Vec::new(),
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
        title: "Install update".into(),
        shortcut: None,
        ..Default::default()
    }
}

fn bytes_line(received: u64, total: Option<u64>) -> String {
    let size = |bytes| ByteSize::b(bytes).display().si().to_string();
    match total {
        Some(total) => format!("{} of {}", size(received), size(total)),
        None => size(received),
    }
}

fn downloading_line(version: &str, progress: Option<f32>) -> String {
    match progress {
        Some(share) => format!("Downloading Dango {version} · {}%", (share * 100.0).round()),
        None => format!("Downloading Dango {version}"),
    }
}

fn detail(markdown: String, loading: bool, actions: Vec<Action>) -> ViewTree {
    ViewTree::new(View::Detail(DetailView {
        markdown,
        loading,
        actions,
        ..Default::default()
    }))
}

struct UpdateProvider(Arc<Inner>);

#[async_trait]
impl RootProvider for UpdateProvider {
    async fn items(&self, _query: String) -> Vec<Candidate> {
        let Some(version) = self.0.available() else {
            return vec![];
        };
        let action = |id: &str, title: &str| Action {
            id: id.into(),
            title: title.into(),
            ..Default::default()
        };
        let (title, primary) = match self.0.task_state().map(|state| state.status) {
            Some(TaskStatus::NeedsInput { .. }) => (
                format!("Dango {version} is ready to restart"),
                action(ACTION_RESTART, "Restart now"),
            ),
            Some(TaskStatus::Running) => (
                available_title(&version),
                action(ACTION_SHOW, "Show progress"),
            ),
            _ => (available_title(&version), install_action()),
        };
        vec![Candidate {
            extension_id: EXTENSION_ID.into(),
            id: ITEM_ID.into(),
            title,
            status: Some(format!("You have {}", self.0.current)),
            icon: Some(format!("{NAMED_ICON}circle-arrow-down")),
            keywords: vec!["update".into(), "dango".into()],
            alias: None,
            source: Source::RootItem,
            actions: vec![primary],
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

        fn progress(&self) -> Vec<crate::protocol::ProgressView> {
            self.0
                .lock()
                .unwrap()
                .iter()
                .filter_map(|tree| match &tree.view {
                    View::Progress(progress) => Some(progress.clone()),
                    _ => None,
                })
                .collect()
        }

        fn last_progress(&self) -> crate::protocol::ProgressView {
            self.progress().pop().expect("a progress view was shown")
        }
    }

    fn statuses(view: &crate::protocol::ProgressView) -> Vec<Status> {
        view.steps.iter().map(|step| step.status).collect()
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
            Tasks::new(|_| {}),
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

        fn act(&self, action: &str) -> ActionOutcome {
            self.extension
                .perform_action(ITEM_ID, action, &FormValues::new())
        }

        fn status(&self) -> Option<TaskStatus> {
            self.extension.inner.task_state().map(|state| state.status)
        }

        async fn settle(&self) {
            while self.status() == Some(TaskStatus::Running) {
                tokio::task::yield_now().await;
            }
        }

        async fn download_to_the_end(&self) {
            self.extension.inner.check().await.unwrap();
            assert_eq!(self.act(ACTION_INSTALL), ActionOutcome::Started);
            self.settle().await;
        }
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
        assert_eq!(items[0].status.as_deref(), Some("You have 0.4.0"));
        assert!(items[0].pinned);
        assert!(items[0].keywords.contains(&"update".to_string()));
        assert!(items[0].keywords.contains(&"dango".to_string()));
        assert_eq!(items[0].actions[0].title, "Install update");
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
    async fn dismissing_the_launcher_does_not_stop_the_download() {
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
            Tasks::new(|_| {}),
        );
        extension.inner.check().await.unwrap();
        extension.inner.start_install();
        while extension
            .inner
            .task_state()
            .is_some_and(|state| state.status == TaskStatus::Running)
        {
            tokio::task::yield_now().await;
        }
        assert!(matches!(
            extension.inner.task_state().map(|state| state.status),
            Some(TaskStatus::NeedsInput { .. })
        ));
        assert!(source.installed.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn the_download_shows_its_steps_and_waits_to_restart() {
        let fixture = fixture(FakeSource {
            chunks: vec![10_000_000, 10_000_000],
            total: Some(20_000_000),
            ..FakeSource::finding("0.5.0")
        });
        fixture.download_to_the_end().await;

        let views = fixture.views.progress();
        assert_eq!(views[0].title, "Dango 0.5.0");
        assert_eq!(views[0].subtitle, "You have 0.4.0");
        assert_eq!(
            statuses(&views[0]),
            [Status::Working, Status::Queued, Status::Queued]
        );
        assert!(views
            .iter()
            .any(|view| view.detail == "10.0 MB of 20.0 MB" && view.value == Some(0.5)));
        let last = fixture.views.last_progress();
        assert_eq!(
            statuses(&last),
            [Status::Done, Status::Done, Status::NeedsInput]
        );
        assert_eq!(last.detail, "Ready to restart");
        assert_eq!(last.actions[0].title, "Restart now");
        assert_eq!(last.actions[1].title, "Install later");
        assert!(fixture.source.installed.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn restart_now_installs_once() {
        let fixture = fixture(FakeSource::finding("0.5.0"));
        fixture.download_to_the_end().await;
        assert_eq!(fixture.act(ACTION_RESTART), ActionOutcome::Done);
        assert_eq!(
            *fixture.source.installed.lock().unwrap(),
            [b"update".to_vec()]
        );
        assert!(matches!(
            fixture.act(ACTION_RESTART),
            ActionOutcome::Failed(_)
        ));
        assert_eq!(fixture.source.installed.lock().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn no_check_runs_while_an_update_waits() {
        let fixture = fixture(FakeSource::finding("0.5.0"));
        fixture.download_to_the_end().await;
        let checks = fixture.source.checks();
        *fixture.source.found.lock().unwrap() = Some("0.5.1".into());
        assert_eq!(
            fixture.extension.inner.check().await.unwrap().as_deref(),
            Some("0.5.0")
        );
        assert_eq!(fixture.source.checks(), checks);
    }

    #[tokio::test]
    async fn cancelling_ends_the_task_and_installs_nothing() {
        let gate = Arc::new(tokio::sync::Notify::new());
        let fixture = fixture(FakeSource {
            gate: Some(gate.clone()),
            ..FakeSource::finding("0.5.0")
        });
        fixture.extension.inner.check().await.unwrap();
        fixture.act(ACTION_INSTALL);
        tokio::task::yield_now().await;
        assert_eq!(fixture.act(ACTION_CANCEL), ActionOutcome::Back);
        while fixture.status().is_some() {
            tokio::task::yield_now().await;
        }
        gate.notify_waiters();
        assert!(fixture.source.installed.lock().unwrap().is_empty());
        assert!(fixture.extension.inner.ready.lock().unwrap().is_none());
        assert_eq!(fixture.items().await[0].actions[0].title, "Install update");
    }

    #[tokio::test]
    async fn a_bad_signature_installs_nothing_and_says_so() {
        let fixture = fixture(FakeSource {
            download_error: Some(UpdateError::Signature("bad signature".into())),
            ..FakeSource::finding("0.5.0")
        });
        fixture.download_to_the_end().await;

        let last = fixture.views.last_progress();
        assert_eq!(
            last.detail,
            "Couldn't verify the update, so it wasn't installed. Try again later."
        );
        assert_eq!(
            statuses(&last),
            [Status::Done, Status::Failed, Status::Queued]
        );
        assert_eq!(last.actions[0].title, "Retry");
        assert!(fixture.source.installed.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn a_failed_download_installs_nothing_and_keeps_the_update() {
        let fixture = fixture(FakeSource {
            download_error: Some(UpdateError::Download("reset by peer".into())),
            ..FakeSource::finding("0.5.0")
        });
        fixture.download_to_the_end().await;

        let last = fixture.views.last_progress();
        assert_eq!(
            last.detail,
            "Couldn't download the update. Check your connection and try again."
        );
        assert_eq!(statuses(&last)[0], Status::Failed);
        assert!(fixture.source.installed.lock().unwrap().is_empty());
        assert_eq!(fixture.items().await.len(), 1);
        assert_eq!(fixture.items().await[0].actions[0].title, "Install update");
    }

    #[tokio::test]
    async fn the_update_item_follows_the_download() {
        let gate = Arc::new(tokio::sync::Notify::new());
        let fixture = fixture(FakeSource {
            gate: Some(gate.clone()),
            ..FakeSource::finding("0.5.0")
        });
        fixture.extension.inner.check().await.unwrap();
        fixture.act(ACTION_INSTALL);
        let running = &fixture.items().await[0];
        assert_eq!(running.title, "Dango 0.5.0 is available");
        assert_eq!(running.actions[0].title, "Show progress");

        assert_eq!(fixture.act(ACTION_SHOW), ActionOutcome::Started);
        assert_eq!(
            fixture.status(),
            Some(TaskStatus::Running),
            "no second task"
        );

        while fixture.status() == Some(TaskStatus::Running) {
            gate.notify_waiters();
            tokio::task::yield_now().await;
        }
        assert_eq!(fixture.act(ACTION_LATER), ActionOutcome::Back);
        let ready = &fixture.items().await[0];
        assert_eq!(ready.title, "Dango 0.5.0 is ready to restart");
        assert_eq!(ready.actions[0].title, "Restart now");
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
        assert_eq!(last.actions[0].title, "Install update");
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

    #[tokio::test(start_paused = true)]
    async fn the_schedule_checks_until_stopped() {
        let fixture = fixture(FakeSource::default());
        let service = fixture.extension.services().remove(0);
        service.start().unwrap();
        tokio::time::sleep(FIRST_CHECK_AFTER * 2).await;
        assert_eq!(fixture.source.checks(), 1);
        service.stop();

        let service = fixture.extension.services().remove(0);
        service.start().unwrap();
        service.stop();
        tokio::time::sleep(FIRST_CHECK_AFTER * 2).await;
        assert_eq!(fixture.source.checks(), 1);
    }

    #[tokio::test(start_paused = true)]
    async fn a_development_build_never_asks() {
        let fixture = build(FakeSource::finding("0.5.0"), false);
        fixture.extension.services()[0].start().unwrap();
        tokio::time::sleep(FIRST_CHECK_AFTER * 2).await;
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
