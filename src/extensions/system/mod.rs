//! The `system` built-in extension: the operating-system actions a launcher is
//! expected to have. The first extension whose whole contribution is commands,
//! so it is what proves the invocation path.

use std::path::Path;
use std::sync::Arc;

use crate::config::FileConfig;
use crate::extension::{
    ActionOutcome, Extension, FormValues, InvocationMode, Manifest, NAMED_ICON,
};
use crate::extensions::applications::{IconCache, ICON_SIZE};
use crate::invocation::{Command, InvocationContext};
use crate::platform::{SystemControl, SystemError};
use crate::protocol::{
    Action, DetailView, Filtering, ListItem, ListView, View, ViewTree, PROTOCOL_VERSION,
};

mod commands;

use commands::{EmptyTrash, Lock, OpenConfigFile, QuitApplication, Sleep};

pub const EXTENSION_ID: &str = "dango.system";

pub const COMMAND_LOCK: &str = "lock";
pub const COMMAND_SLEEP: &str = "sleep";
pub const COMMAND_EMPTY_TRASH: &str = "empty-trash";
pub const COMMAND_QUIT: &str = "quit-application";
pub const COMMAND_OPEN_CONFIG: &str = "open-config";

pub const ACTION_CONFIRM: &str = "confirm";
pub const ACTION_CANCEL: &str = "cancel";
pub const ACTION_QUIT: &str = "quit";

/// Hands a file to whatever the operating system associates with its type.
/// Implemented on the app handle, where the opener plugin lives, so the
/// extension never sees Tauri.
pub trait OpenFile: Send + Sync {
    fn open(&self, path: &Path) -> Result<(), String>;
}

pub struct SystemExtension {
    manifest: Manifest,
    control: Arc<dyn SystemControl>,
    icons: Arc<IconCache>,
    config: Arc<FileConfig>,
    opener: Arc<dyn OpenFile>,
}

impl SystemExtension {
    pub fn new(
        control: Arc<dyn SystemControl>,
        icons: Arc<IconCache>,
        config: Arc<FileConfig>,
        opener: Arc<dyn OpenFile>,
    ) -> Self {
        Self {
            manifest: manifest(),
            control,
            icons,
            config,
            opener,
        }
    }
}

impl Extension for SystemExtension {
    fn manifest(&self) -> &Manifest {
        &self.manifest
    }

    fn command(&self, command_id: &str) -> Option<Arc<dyn Command>> {
        let control = self.control.clone();
        match command_id {
            COMMAND_LOCK => Some(Arc::new(Lock(control))),
            COMMAND_SLEEP => Some(Arc::new(Sleep(control))),
            COMMAND_EMPTY_TRASH => Some(Arc::new(EmptyTrash(control))),
            COMMAND_QUIT => Some(Arc::new(QuitApplication(control, self.icons.clone()))),
            COMMAND_OPEN_CONFIG => Some(Arc::new(OpenConfigFile(
                self.config.clone(),
                self.opener.clone(),
            ))),
            _ => None,
        }
    }

    /// Actions arriving from a view one of these commands pushed. The item id
    /// says what was chosen; nothing is remembered between pushing the view and
    /// hearing back, because neither command needs to be.
    fn perform_action(
        &self,
        item_id: &str,
        action_id: &str,
        _values: &FormValues,
    ) -> ActionOutcome {
        match action_id {
            ACTION_CONFIRM => match self.control.empty_trash() {
                Ok(()) => ActionOutcome::Done,
                Err(error) => ActionOutcome::Failed(error.to_string()),
            },
            ACTION_CANCEL => ActionOutcome::Done,
            ACTION_QUIT => match self.control.quit(item_id) {
                Ok(()) => ActionOutcome::Done,
                Err(error) => ActionOutcome::Failed(error.to_string()),
            },
            _ => ActionOutcome::Failed("That action isn't available.".into()),
        }
    }
}

pub(crate) fn manifest() -> Manifest {
    Manifest {
        manifest_version: 1,
        id: EXTENSION_ID.into(),
        name: "System".into(),
        icon: None,
        tint: Some("red".into()),
        commands: vec![
            command(
                COMMAND_LOCK,
                "Lock Screen",
                "lock",
                InvocationMode::NoView,
                &["lock", "screen", "session", "secure"],
            ),
            command(
                COMMAND_SLEEP,
                "Sleep",
                "moon",
                InvocationMode::NoView,
                &["sleep", "suspend", "standby"],
            ),
            command(
                COMMAND_EMPTY_TRASH,
                empty_trash_title(),
                "trash-2",
                InvocationMode::View,
                &["trash", "bin", "delete", "empty", "recycle"],
            ),
            command(
                COMMAND_QUIT,
                "Quit Application",
                "circle-power",
                InvocationMode::View,
                &["quit", "close", "exit", "kill"],
            ),
            command(
                COMMAND_OPEN_CONFIG,
                "Open Config File",
                "settings",
                InvocationMode::NoView,
                &[
                    "config",
                    "configuration",
                    "settings",
                    "preferences",
                    "keybinds",
                    "hotkeys",
                    "keys",
                    "edit",
                    "json",
                ],
            ),
        ],
        preferences: vec![],
        root_items: false,
        services: false,
    }
}

#[cfg(target_os = "macos")]
fn empty_trash_title() -> &'static str {
    "Empty Trash"
}
#[cfg(not(target_os = "macos"))]
fn empty_trash_title() -> &'static str {
    "Empty Recycle Bin"
}

fn command(
    id: &str,
    title: &str,
    icon: &str,
    mode: InvocationMode,
    keywords: &[&str],
) -> crate::extension::CommandDecl {
    crate::extension::CommandDecl {
        id: id.into(),
        title: title.into(),
        mode,
        subtitle: Some("System".into()),
        icon: Some(format!("{NAMED_ICON}{icon}")),
        keywords: keywords.iter().map(|k| (*k).to_string()).collect(),
        alias: None,
    }
}

/// The confirmation for a destructive, unrecoverable action. Cancel is first,
/// so it is the primary action and a reflexive second Enter does nothing.
fn confirmation(count: usize) -> ViewTree {
    let noun = if count == 1 { "item" } else { "items" };
    ViewTree {
        protocol_version: PROTOCOL_VERSION,
        view: View::Detail(DetailView {
            markdown: format!("Permanently delete {count} {noun}?\n\nThis cannot be undone."),
            loading: false,
            actions: vec![
                Action {
                    id: ACTION_CANCEL.into(),
                    title: "Cancel".into(),
                    shortcut: None,
                },
                Action {
                    id: ACTION_CONFIRM.into(),
                    title: format!("Delete {count} {noun}"),
                    shortcut: None,
                },
            ],
        }),
    }
}

/// The running applications, filtered by the launcher because the list is small
/// and complete when it is pushed. Icons are rendered here rather than in the
/// background, because the list is short and it is on screen only once asked
/// for, unlike root search. They are cached by locator rather than by the
/// application's id, because the cache outlives the session and process ids
/// are reused.
fn running_list(apps: Vec<crate::platform::RunningApp>, icons: &IconCache) -> ViewTree {
    ViewTree {
        protocol_version: PROTOCOL_VERSION,
        view: View::List(ListView {
            filtering: Filtering::Launcher,
            loading: false,
            empty_state: Some(crate::protocol::EmptyState {
                title: "No apps to quit".into(),
                description: None,
            }),
            items: apps
                .into_iter()
                .map(|app| ListItem {
                    icon: app.icon.and_then(|locator| {
                        icons.ensure_with(&locator, || {
                            crate::platform::icon_for(&locator, ICON_SIZE)
                        });
                        icons.path(&locator)
                    }),
                    id: app.id,
                    title: app.name,
                    subtitle: None,
                    actions: vec![Action {
                        id: ACTION_QUIT.into(),
                        title: "Quit".into(),
                        shortcut: None,
                    }],
                })
                .collect(),
        }),
    }
}

fn report(ctx: &InvocationContext, result: Result<(), SystemError>) {
    match result {
        Ok(()) => ctx.succeed(),
        Err(error) => ctx.fail(error.to_string()),
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::extension::Host;
    use crate::invocation::{CommandResolver, Invoker, Outcome, Output, Resolved};
    use std::path::PathBuf;
    use std::sync::Mutex;

    /// A `SystemControl` that records what it was asked to do and answers
    /// however the test needs.
    #[derive(Default)]
    pub struct FakeControl {
        pub calls: Mutex<Vec<String>>,
        pub trash: Mutex<usize>,
        pub apps: Mutex<Vec<crate::platform::RunningApp>>,
        pub quit_gone: Mutex<bool>,
        pub fail: Mutex<bool>,
    }

    impl FakeControl {
        pub fn new() -> Arc<Self> {
            Arc::new(Self::default())
        }
        fn note(&self, what: &str) {
            self.calls.lock().unwrap().push(what.to_string());
        }
        fn outcome(&self) -> Result<(), SystemError> {
            if *self.fail.lock().unwrap() {
                Err(SystemError::Failed("the system said no".into()))
            } else {
                Ok(())
            }
        }
    }

    impl SystemControl for FakeControl {
        fn lock(&self) -> Result<(), SystemError> {
            self.note("lock");
            self.outcome()
        }
        fn sleep(&self) -> Result<(), SystemError> {
            self.note("sleep");
            self.outcome()
        }
        fn trash_count(&self) -> Result<usize, SystemError> {
            Ok(*self.trash.lock().unwrap())
        }
        fn empty_trash(&self) -> Result<(), SystemError> {
            self.note("empty_trash");
            self.outcome()
        }
        fn running_apps(&self) -> Vec<crate::platform::RunningApp> {
            self.apps.lock().unwrap().clone()
        }
        fn quit(&self, app_id: &str) -> Result<(), SystemError> {
            self.note(&format!("quit:{app_id}"));
            if *self.quit_gone.lock().unwrap() {
                Err(SystemError::Gone)
            } else {
                Ok(())
            }
        }
    }

    /// An `OpenFile` that records the paths it was handed.
    #[derive(Default)]
    pub struct FakeOpener {
        pub opened: Mutex<Vec<PathBuf>>,
        pub fail: Mutex<bool>,
    }

    impl OpenFile for FakeOpener {
        fn open(&self, path: &Path) -> Result<(), String> {
            self.opened.lock().unwrap().push(path.to_path_buf());
            if *self.fail.lock().unwrap() {
                Err("no handler".into())
            } else {
                Ok(())
            }
        }
    }

    /// A config file in a fresh temp directory, not yet on disk.
    fn temp_config() -> Arc<FileConfig> {
        let dir = std::env::temp_dir().join(format!("dango-system-{}", uuid::Uuid::new_v4()));
        Arc::new(FileConfig::new(
            dir.join("config.json"),
            crate::config::Config::default(),
        ))
    }

    fn extension(control: Arc<FakeControl>) -> SystemExtension {
        SystemExtension::new(
            control,
            IconCache::in_temp(),
            temp_config(),
            Arc::new(FakeOpener::default()),
        )
    }

    struct Direct(Arc<SystemExtension>);

    impl CommandResolver for Direct {
        fn resolve(&self, qualified_id: &str) -> Option<Resolved> {
            let id = qualified_id.strip_prefix(&format!("{EXTENSION_ID}."))?;
            Some(Resolved {
                command: self.0.command(id)?,
                host: Host::Builtin,
            })
        }
    }

    fn setup(control: Arc<FakeControl>) -> (Arc<SystemExtension>, Invoker) {
        let extension = Arc::new(extension(control));
        let invoker = Invoker::new(Arc::new(Direct(extension.clone())));
        (extension, invoker)
    }

    async fn run(invoker: &Invoker, command: &str) -> Vec<Output> {
        let mut rx = invoker
            .invoke(&format!("{EXTENSION_ID}.{command}"))
            .unwrap()
            .output;
        let mut output = Vec::new();
        while let Some(item) = rx.recv().await {
            output.push(item);
        }
        output
    }

    fn app(id: &str, name: &str) -> crate::platform::RunningApp {
        crate::platform::RunningApp {
            id: id.into(),
            name: name.into(),
            icon: None,
        }
    }

    #[test]
    fn every_command_carries_a_named_icon() {
        for declared in &manifest().commands {
            let icon = declared.icon.as_deref().unwrap_or_default();
            assert!(
                icon.starts_with(NAMED_ICON),
                "{} has no named icon",
                declared.id
            );
        }
    }

    #[test]
    fn lock_and_sleep_are_no_view_and_the_rest_are_views() {
        let manifest = manifest();
        let mode = |id: &str| manifest.commands.iter().find(|c| c.id == id).unwrap().mode;
        assert_eq!(mode(COMMAND_LOCK), InvocationMode::NoView);
        assert_eq!(mode(COMMAND_SLEEP), InvocationMode::NoView);
        assert_eq!(mode(COMMAND_OPEN_CONFIG), InvocationMode::NoView);
        assert_eq!(mode(COMMAND_EMPTY_TRASH), InvocationMode::View);
        assert_eq!(mode(COMMAND_QUIT), InvocationMode::View);
    }

    #[test]
    fn every_command_declared_has_an_implementation() {
        let extension = extension(FakeControl::new());
        for declared in &manifest().commands {
            assert!(
                extension.command(&declared.id).is_some(),
                "{} is declared but not invocable",
                declared.id
            );
        }
    }

    #[tokio::test]
    async fn lock_asks_the_system_to_lock_and_succeeds() {
        let control = FakeControl::new();
        let (_, invoker) = setup(control.clone());
        let output = run(&invoker, COMMAND_LOCK).await;
        assert_eq!(*control.calls.lock().unwrap(), ["lock"]);
        assert!(matches!(
            output.as_slice(),
            [Output::Finished(Outcome::Success)]
        ));
    }

    #[tokio::test]
    async fn sleep_reports_a_refusal_as_a_failure() {
        let control = FakeControl::new();
        *control.fail.lock().unwrap() = true;
        let (_, invoker) = setup(control.clone());
        let output = run(&invoker, COMMAND_SLEEP).await;
        assert_eq!(*control.calls.lock().unwrap(), ["sleep"]);
        assert!(matches!(
            output.as_slice(),
            [Output::Finished(Outcome::Failure(_))]
        ));
    }

    #[tokio::test]
    async fn empty_trash_asks_before_deleting_anything() {
        let control = FakeControl::new();
        *control.trash.lock().unwrap() = 3;
        let (_, invoker) = setup(control.clone());
        let output = run(&invoker, COMMAND_EMPTY_TRASH).await;

        assert!(
            control.calls.lock().unwrap().is_empty(),
            "nothing deleted yet"
        );
        let [Output::View(tree)] = output.as_slice() else {
            panic!("expected a confirmation view, got {output:?}");
        };
        let View::Detail(detail) = &tree.view else {
            panic!("expected a detail view");
        };
        assert!(detail.markdown.contains('3'), "the count must be named");
        assert_eq!(
            detail.actions[0].id, ACTION_CANCEL,
            "cancel must be primary"
        );
    }

    #[tokio::test]
    async fn an_already_empty_trash_succeeds_without_asking() {
        let control = FakeControl::new();
        let (_, invoker) = setup(control.clone());
        let output = run(&invoker, COMMAND_EMPTY_TRASH).await;
        assert!(control.calls.lock().unwrap().is_empty());
        assert!(matches!(
            output.as_slice(),
            [Output::Finished(Outcome::Success)]
        ));
    }

    #[test]
    fn confirming_empties_the_trash_and_declining_does_not() {
        let control = FakeControl::new();
        let extension = extension(control.clone());

        assert_eq!(
            extension.perform_action("", ACTION_CANCEL, &FormValues::new()),
            ActionOutcome::Done
        );
        assert!(control.calls.lock().unwrap().is_empty());

        assert_eq!(
            extension.perform_action("", ACTION_CONFIRM, &FormValues::new()),
            ActionOutcome::Done
        );
        assert_eq!(*control.calls.lock().unwrap(), ["empty_trash"]);
    }

    #[tokio::test]
    async fn quit_lists_the_running_applications() {
        let control = FakeControl::new();
        *control.apps.lock().unwrap() = vec![app("1", "Safari"), app("2", "Mail")];
        let (_, invoker) = setup(control);
        let output = run(&invoker, COMMAND_QUIT).await;

        let [Output::View(tree)] = output.as_slice() else {
            panic!("expected a list view, got {output:?}");
        };
        let View::List(list) = &tree.view else {
            panic!("expected a list view");
        };
        assert!(matches!(list.filtering, Filtering::Launcher));
        let titles: Vec<_> = list.items.iter().map(|i| i.title.as_str()).collect();
        assert_eq!(titles, ["Safari", "Mail"]);
        assert_eq!(list.items[0].actions[0].id, ACTION_QUIT);
    }

    #[test]
    fn choosing_an_application_asks_that_one_to_quit() {
        let control = FakeControl::new();
        let extension = extension(control.clone());
        assert_eq!(
            extension.perform_action("42", ACTION_QUIT, &FormValues::new()),
            ActionOutcome::Done
        );
        assert_eq!(*control.calls.lock().unwrap(), ["quit:42"]);
    }

    #[test]
    fn an_application_that_already_exited_is_reported() {
        let control = FakeControl::new();
        *control.quit_gone.lock().unwrap() = true;
        let extension = extension(control);
        assert!(matches!(
            extension.perform_action("42", ACTION_QUIT, &FormValues::new()),
            ActionOutcome::Failed(_)
        ));
    }

    #[test]
    fn an_unknown_action_fails_cleanly() {
        let extension = extension(FakeControl::new());
        assert!(matches!(
            extension.perform_action("", "nope", &FormValues::new()),
            ActionOutcome::Failed(_)
        ));
    }

    fn setup_open_config(
        config: Arc<FileConfig>,
        opener: Arc<FakeOpener>,
    ) -> (Arc<FakeOpener>, Invoker) {
        let extension = Arc::new(SystemExtension::new(
            FakeControl::new(),
            IconCache::in_temp(),
            config,
            opener.clone(),
        ));
        (opener, Invoker::new(Arc::new(Direct(extension))))
    }

    #[tokio::test]
    async fn open_config_creates_a_missing_file_and_opens_it() {
        let config = temp_config();
        let (opener, invoker) = setup_open_config(config.clone(), Arc::new(FakeOpener::default()));
        let output = run(&invoker, COMMAND_OPEN_CONFIG).await;

        let path = config.ensure_file().unwrap();
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            "{
  \"$schema\": \"./config.schema.json\",
  \"version\": 1
}"
        );
        assert_eq!(*opener.opened.lock().unwrap(), std::slice::from_ref(&path));
        assert!(matches!(
            output.as_slice(),
            [Output::Finished(Outcome::Success)]
        ));
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }

    #[tokio::test]
    async fn open_config_opens_an_existing_file_untouched() {
        let config = temp_config();
        let path = config.ensure_file().unwrap();
        std::fs::write(&path, "{ not even json").unwrap();
        let (opener, invoker) = setup_open_config(config, Arc::new(FakeOpener::default()));
        let output = run(&invoker, COMMAND_OPEN_CONFIG).await;

        assert_eq!(std::fs::read_to_string(&path).unwrap(), "{ not even json");
        assert_eq!(*opener.opened.lock().unwrap(), std::slice::from_ref(&path));
        assert!(matches!(
            output.as_slice(),
            [Output::Finished(Outcome::Success)]
        ));
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }

    #[tokio::test]
    async fn open_config_reports_a_handler_that_would_not_start() {
        let config = temp_config();
        let opener = Arc::new(FakeOpener::default());
        *opener.fail.lock().unwrap() = true;
        let (_, invoker) = setup_open_config(config.clone(), opener);
        let output = run(&invoker, COMMAND_OPEN_CONFIG).await;

        let [Output::Finished(Outcome::Failure(message))] = output.as_slice() else {
            panic!("expected a failure, got {output:?}");
        };
        assert_eq!(message, commands::OPEN_FAILED);
        let _ = std::fs::remove_dir_all(config.ensure_file().unwrap().parent().unwrap());
    }

    #[tokio::test]
    async fn open_config_reports_a_file_it_cannot_create() {
        // A regular file where the directory should be, so nothing can be
        // created under it.
        let dir = std::env::temp_dir().join(format!("dango-system-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let blocker = dir.join("blocker");
        std::fs::write(&blocker, "").unwrap();
        let config = Arc::new(FileConfig::new(
            blocker.join("config.json"),
            crate::config::Config::default(),
        ));
        let (opener, invoker) = setup_open_config(config, Arc::new(FakeOpener::default()));
        let output = run(&invoker, COMMAND_OPEN_CONFIG).await;

        let [Output::Finished(Outcome::Failure(message))] = output.as_slice() else {
            panic!("expected a failure, got {output:?}");
        };
        assert_eq!(message, commands::CREATE_FAILED);
        assert!(opener.opened.lock().unwrap().is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
