//! The extension proper: the manifest, the command that opens the history, and
//! the policy that turns preferences into the bounds and exclusions the watcher
//! asks for on every change.

use std::sync::Arc;

use crate::extension::{
    ActionOutcome, CommandDecl, Extension, InvocationMode, Manifest, PreferenceDecl,
    PreferenceKind, Preferences, Service, NAMED_ICON,
};
use crate::invocation::{Command, InvocationContext};
use crate::protocol::{
    Action, EmptyState, Filtering, ListItem, ListView, Modifier, Shortcut, View, ViewTree,
    PROTOCOL_VERSION,
};

use super::history::{Bounds, Entry, History, Kind};
use super::watcher::{Policy, WatchService, Watcher};
use super::DEFAULT_EXCLUDED_APPLICATIONS;

pub const EXTENSION_ID: &str = "dango.clipboard";
pub const COMMAND_HISTORY: &str = "history";

pub const ACTION_RESTORE: &str = "restore";
pub const ACTION_REMOVE: &str = "remove";

pub const PREF_ENTRIES: &str = "entries";
pub const PREF_ENTRY_MEGABYTES: &str = "entry-megabytes";
pub const PREF_TOTAL_MEGABYTES: &str = "total-megabytes";
pub const PREF_EXCLUDED: &str = "excluded-applications";

const MEGABYTE: f64 = 1024.0 * 1024.0;

/// How much of a text entry is worth showing on one line.
const PREVIEW_CHARS: usize = 120;

pub struct ClipboardExtension {
    manifest: Manifest,
    history: Arc<History>,
    watcher: Arc<Watcher>,
}

impl ClipboardExtension {
    pub fn new(history: Arc<History>, watcher: Arc<Watcher>) -> Self {
        Self {
            manifest: manifest(),
            history,
            watcher,
        }
    }

    fn list(&self) -> ViewTree {
        match self.history.entries() {
            Ok(entries) => history_view(&entries),
            Err(error) => {
                eprintln!("[dango] could not read the clipboard history: {error}");
                history_view(&[])
            }
        }
    }
}

impl Extension for ClipboardExtension {
    fn manifest(&self) -> &Manifest {
        &self.manifest
    }

    fn services(&self) -> Vec<Arc<dyn Service>> {
        vec![Arc::new(WatchService::new(self.watcher.clone()))]
    }

    fn command(&self, command_id: &str) -> Option<Arc<dyn Command>> {
        match command_id {
            COMMAND_HISTORY => Some(Arc::new(HistoryCommand {
                history: self.history.clone(),
            })),
            _ => None,
        }
    }

    fn perform_action(&self, item_id: &str, action_id: &str) -> ActionOutcome {
        match action_id {
            ACTION_RESTORE => match self.history.content(item_id) {
                Ok(content) => {
                    self.watcher.restore(&content);
                    ActionOutcome::Done
                }
                Err(error) => ActionOutcome::Failed(error.to_string()),
            },
            // The list is handed back rebuilt, so clearing several entries does
            // not mean reopening the history between each one.
            ACTION_REMOVE => match self.history.remove(item_id) {
                Ok(()) => ActionOutcome::Replaced(Box::new(self.list())),
                Err(error) => ActionOutcome::Failed(error.to_string()),
            },
            other => ActionOutcome::Failed(format!("unknown action '{other}'")),
        }
    }
}

/// The preference declarations, for building a reader before the extension
/// itself exists. Without these the reader has nothing to supply defaults from,
/// so every value would read as absent and a hand-edited exclusion list would
/// silently do nothing.
pub fn preference_declarations() -> Vec<PreferenceDecl> {
    manifest().preferences
}

fn manifest() -> Manifest {
    Manifest {
        manifest_version: 1,
        id: EXTENSION_ID.into(),
        name: "Clipboard History".into(),
        icon: Some(format!("{NAMED_ICON}clipboard-list")),
        commands: vec![CommandDecl {
            id: COMMAND_HISTORY.into(),
            title: "Clipboard History".into(),
            mode: InvocationMode::View,
            subtitle: Some("Clipboard".into()),
            icon: Some(format!("{NAMED_ICON}clipboard-list")),
            keywords: ["clipboard", "history", "paste", "copied"]
                .iter()
                .map(|k| (*k).to_string())
                .collect(),
            alias: None,
        }],
        preferences: vec![
            number(PREF_ENTRIES, 500.0),
            number(PREF_ENTRY_MEGABYTES, 16.0),
            number(PREF_TOTAL_MEGABYTES, 256.0),
            PreferenceDecl {
                command_id: None,
                key: PREF_EXCLUDED.into(),
                kind: PreferenceKind::String,
                default: Some(DEFAULT_EXCLUDED_APPLICATIONS.join(", ").into()),
                required: false,
            },
        ],
        root_items: false,
        services: true,
    }
}

fn number(key: &str, default: f64) -> PreferenceDecl {
    PreferenceDecl {
        command_id: None,
        key: key.into(),
        kind: PreferenceKind::Number,
        default: Some(default.into()),
        required: false,
    }
}

/// Turns preferences into what the watcher asks for on every change, so editing
/// a value takes effect without a restart.
pub struct PreferencePolicy(pub Preferences);

impl Policy for PreferencePolicy {
    fn bounds(&self) -> Bounds {
        let fallback = Bounds::default();
        Bounds {
            entries: self
                .0
                .count(PREF_ENTRIES)
                .map(|value| value as usize)
                .unwrap_or(fallback.entries),
            entry_bytes: megabytes(&self.0, PREF_ENTRY_MEGABYTES, fallback.entry_bytes),
            total_bytes: megabytes(&self.0, PREF_TOTAL_MEGABYTES, fallback.total_bytes),
        }
    }

    fn excluded_applications(&self) -> Vec<String> {
        match self.0.string(PREF_EXCLUDED) {
            Some(value) => value
                .split(',')
                .map(str::trim)
                .filter(|name| !name.is_empty())
                .map(str::to_string)
                .collect(),
            None => DEFAULT_EXCLUDED_APPLICATIONS
                .iter()
                .map(|name| (*name).to_string())
                .collect(),
        }
    }
}

fn megabytes(preferences: &Preferences, key: &str, fallback: usize) -> usize {
    preferences
        .number(key)
        .filter(|value| *value > 0.0)
        .map(|value| (value * MEGABYTE) as usize)
        .unwrap_or(fallback)
}

struct HistoryCommand {
    history: Arc<History>,
}

impl Command for HistoryCommand {
    fn invoke(&self, ctx: &InvocationContext) {
        match self.history.entries() {
            Ok(entries) => ctx.push_view(history_view(&entries)),
            Err(error) => ctx.fail(error.to_string()),
        }
    }
}

pub fn history_view(entries: &[Entry]) -> ViewTree {
    ViewTree {
        protocol_version: PROTOCOL_VERSION,
        view: View::List(ListView {
            filtering: Filtering::Launcher,
            loading: false,
            empty_state: Some(EmptyState {
                title: "Nothing copied yet".into(),
                description: Some(
                    "What you copy from now on will show up here, unless it came from an excluded application."
                        .into(),
                ),
            }),
            items: entries.iter().map(item).collect(),
        }),
    }
}

fn item(entry: &Entry) -> ListItem {
    let (title, icon) = match entry.kind {
        Kind::Text => (
            preview(entry.text.as_deref().unwrap_or_default()),
            Some(format!("{NAMED_ICON}clipboard-list")),
        ),
        // The thumbnail is the entry's own file, which is the only way to tell
        // one copied image from another.
        Kind::Image => ("Image".to_string(), entry.path.clone()),
    };
    ListItem {
        id: entry.id.clone(),
        title,
        subtitle: None,
        icon,
        actions: vec![
            Action {
                id: ACTION_RESTORE.into(),
                title: "Copy to Clipboard".into(),
                shortcut: None,
            },
            Action {
                id: ACTION_REMOVE.into(),
                title: "Remove from History".into(),
                shortcut: Some(Shortcut {
                    key: "x".into(),
                    modifiers: vec![Modifier::Ctrl],
                }),
            },
        ],
    }
}

/// One line, so entries can be told apart at a glance. Runs of whitespace
/// become single spaces, which is what makes a copied block of indented code
/// legible in a list row rather than a stripe of empty space.
fn preview(text: &str) -> String {
    let flattened: String = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if flattened.chars().count() <= PREVIEW_CHARS {
        return flattened;
    }
    format!(
        "{}…",
        flattened.chars().take(PREVIEW_CHARS).collect::<String>()
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::extension::PreferenceStore;
    use crate::extensions::clipboard::{ClipboardSource, Content};
    use std::collections::HashMap;
    use std::sync::Mutex;

    #[derive(Default)]
    struct MemoryPreferences(Mutex<HashMap<String, String>>);

    impl PreferenceStore for MemoryPreferences {
        fn get(&self, _extension_id: &str, _command_id: Option<&str>, key: &str) -> Option<String> {
            self.0.lock().unwrap().get(key).cloned()
        }
        fn set(&self, _extension_id: &str, _command_id: Option<&str>, key: &str, value: &str) {
            self.0.lock().unwrap().insert(key.into(), value.into());
        }
    }

    #[derive(Default)]
    struct StubClipboard(Mutex<Option<Content>>);

    impl ClipboardSource for StubClipboard {
        fn sequence(&self) -> i64 {
            0
        }
        fn is_excluded(&self) -> bool {
            false
        }
        fn candidate_applications(&self) -> Vec<String> {
            Vec::new()
        }
        fn text(&self) -> Option<String> {
            None
        }
        fn image(&self) -> Option<Vec<u8>> {
            None
        }
        fn set_text(&self, text: &str) {
            *self.0.lock().unwrap() = Some(Content::Text(text.into()));
        }
        fn set_image(&self, png: &[u8]) {
            *self.0.lock().unwrap() = Some(Content::Image(png.to_vec()));
        }
    }

    fn policy(store: Arc<dyn PreferenceStore>) -> PreferencePolicy {
        PreferencePolicy(Preferences::new(
            EXTENSION_ID,
            manifest().preferences,
            store,
        ))
    }

    fn setup() -> (ClipboardExtension, Arc<History>, Arc<StubClipboard>) {
        let dir = std::env::temp_dir().join(format!("dango-ext-{}", uuid::Uuid::new_v4()));
        let history = Arc::new(History::new(
            Arc::new(crate::store::Store::in_memory().unwrap()),
            dir,
        ));
        let clipboard = Arc::new(StubClipboard::default());
        let watcher = Arc::new(Watcher::new(
            clipboard.clone(),
            history.clone(),
            Arc::new(policy(Arc::new(MemoryPreferences::default()))),
        ));
        (
            ClipboardExtension::new(history.clone(), watcher),
            history,
            clipboard,
        )
    }

    fn list_of(tree: &ViewTree) -> &ListView {
        match &tree.view {
            View::List(list) => list,
            other => panic!("expected a list, got {other:?}"),
        }
    }

    #[test]
    fn the_manifest_validates_and_declares_its_contributions() {
        let manifest = manifest();
        assert!(manifest.validate().is_ok());
        assert_eq!(manifest.commands.len(), 1);
        assert!(manifest.services, "the watcher is a service");
        assert!(
            !manifest.root_items,
            "the history opens through its command"
        );
        assert_eq!(manifest.preferences.len(), 4);
    }

    #[test]
    fn the_declared_command_is_invocable() {
        let (extension, _, _) = setup();
        assert!(extension.command(COMMAND_HISTORY).is_some());
        assert!(extension.command("nope").is_none());
    }

    #[test]
    fn the_declarations_handed_to_a_reader_are_the_manifests_own() {
        let declared: Vec<_> = preference_declarations()
            .into_iter()
            .map(|d| d.key)
            .collect();
        assert_eq!(
            declared,
            [
                PREF_ENTRIES,
                PREF_ENTRY_MEGABYTES,
                PREF_TOTAL_MEGABYTES,
                PREF_EXCLUDED
            ],
            "a reader built without these reads every value as absent"
        );
    }

    #[test]
    fn the_bounds_come_from_preferences() {
        let store = Arc::new(MemoryPreferences::default());
        let policy = policy(store.clone());
        let bounds = policy.bounds();
        assert_eq!(bounds.entries, 500);
        assert_eq!(bounds.entry_bytes, 16 * 1024 * 1024);
        assert_eq!(bounds.total_bytes, 256 * 1024 * 1024);

        store.set(EXTENSION_ID, None, PREF_ENTRIES, "20");
        store.set(EXTENSION_ID, None, PREF_TOTAL_MEGABYTES, "8");
        let bounds = policy.bounds();
        assert_eq!(bounds.entries, 20);
        assert_eq!(bounds.total_bytes, 8 * 1024 * 1024);
    }

    #[test]
    fn the_exclusion_list_defaults_to_the_known_managers() {
        let policy = policy(Arc::new(MemoryPreferences::default()));
        let excluded = policy.excluded_applications();
        assert!(excluded.contains(&"Proton Pass".to_string()));
        assert_eq!(excluded.len(), DEFAULT_EXCLUDED_APPLICATIONS.len());
    }

    #[test]
    fn the_exclusion_list_is_parsed_from_a_written_value() {
        let store = Arc::new(MemoryPreferences::default());
        let policy = policy(store.clone());
        store.set(EXTENSION_ID, None, PREF_EXCLUDED, " Proton Pass ,, Notes ");
        assert_eq!(
            policy.excluded_applications(),
            ["Proton Pass".to_string(), "Notes".to_string()],
            "blank entries and stray spacing are the normal shape of a hand-edited list"
        );
    }

    #[test]
    fn the_history_lists_newest_first() {
        let (extension, history, _) = setup();
        history
            .record(Content::Text("one".into()), Bounds::default(), 1)
            .unwrap();
        history
            .record(Content::Text("two".into()), Bounds::default(), 2)
            .unwrap();

        let tree = extension.list();
        let list = list_of(&tree);
        assert!(matches!(list.filtering, Filtering::Launcher));
        let titles: Vec<_> = list.items.iter().map(|i| i.title.as_str()).collect();
        assert_eq!(titles, ["two", "one"]);
    }

    #[test]
    fn an_empty_history_explains_itself() {
        let (extension, _, _) = setup();
        let tree = extension.list();
        let list = list_of(&tree);
        assert!(list.items.is_empty());
        assert!(list.empty_state.is_some());
    }

    #[test]
    fn a_text_entry_is_flattened_to_one_line() {
        assert_eq!(preview("  hello\n\n  world  "), "hello world");
        assert_eq!(
            preview("fn main() {\n    println!(\"hi\");\n}"),
            "fn main() { println!(\"hi\"); }"
        );
    }

    #[test]
    fn a_long_text_entry_is_cut_rather_than_wrapped() {
        let long = "x".repeat(PREVIEW_CHARS * 2);
        let preview = preview(&long);
        assert_eq!(preview.chars().count(), PREVIEW_CHARS + 1);
        assert!(preview.ends_with('…'));
    }

    #[test]
    fn an_image_entry_shows_its_own_file_as_its_thumbnail() {
        let (extension, history, _) = setup();
        history
            .record(Content::Image(vec![1, 2, 3]), Bounds::default(), 1)
            .unwrap();

        let tree = extension.list();
        let item = &list_of(&tree).items[0];
        let path = item.icon.clone().expect("an image entry has a thumbnail");
        assert!(path.ends_with(".png"));
        assert!(std::path::Path::new(&path).exists());
    }

    #[test]
    fn choosing_a_text_entry_puts_it_back_on_the_clipboard() {
        let (extension, history, clipboard) = setup();
        history
            .record(Content::Text("hello".into()), Bounds::default(), 1)
            .unwrap();
        let id = history.entries().unwrap()[0].id.clone();

        assert_eq!(
            extension.perform_action(&id, ACTION_RESTORE),
            ActionOutcome::Done
        );
        assert_eq!(
            *clipboard.0.lock().unwrap(),
            Some(Content::Text("hello".into()))
        );
    }

    #[test]
    fn choosing_an_image_entry_puts_it_back_on_the_clipboard() {
        let (extension, history, clipboard) = setup();
        history
            .record(Content::Image(vec![9, 9, 9]), Bounds::default(), 1)
            .unwrap();
        let id = history.entries().unwrap()[0].id.clone();

        assert_eq!(
            extension.perform_action(&id, ACTION_RESTORE),
            ActionOutcome::Done
        );
        assert_eq!(
            *clipboard.0.lock().unwrap(),
            Some(Content::Image(vec![9, 9, 9]))
        );
    }

    #[test]
    fn removing_an_entry_hands_back_the_list_without_it() {
        let (extension, history, _) = setup();
        history
            .record(Content::Text("one".into()), Bounds::default(), 1)
            .unwrap();
        history
            .record(Content::Text("two".into()), Bounds::default(), 2)
            .unwrap();
        let id = history.entries().unwrap()[0].id.clone();

        let outcome = extension.perform_action(&id, ACTION_REMOVE);
        let ActionOutcome::Replaced(tree) = outcome else {
            panic!("removing must leave the user in the list, got {outcome:?}");
        };
        let titles: Vec<_> = list_of(&tree)
            .items
            .iter()
            .map(|i| i.title.clone())
            .collect();
        assert_eq!(titles, ["one"]);
    }

    #[test]
    fn choosing_an_entry_that_has_gone_reports_it() {
        let (extension, _, _) = setup();
        assert!(matches!(
            extension.perform_action("nope", ACTION_RESTORE),
            ActionOutcome::Failed(_)
        ));
    }

    #[test]
    fn an_unknown_action_fails_cleanly() {
        let (extension, _, _) = setup();
        assert!(matches!(
            extension.perform_action("whatever", "nope"),
            ActionOutcome::Failed(_)
        ));
    }

    #[test]
    fn restoring_marks_the_write_as_dangos_own() {
        let (extension, history, _) = setup();
        history
            .record(Content::Text("hello".into()), Bounds::default(), 1)
            .unwrap();
        let id = history.entries().unwrap()[0].id.clone();
        extension.perform_action(&id, ACTION_RESTORE);
        assert_eq!(
            history.entries().unwrap().len(),
            1,
            "putting an entry back must not add another"
        );
    }

    #[test]
    fn an_image_entry_whose_file_has_gone_is_reported_and_dropped() {
        let (extension, history, _) = setup();
        history
            .record(Content::Image(vec![4, 5]), Bounds::default(), 1)
            .unwrap();
        let entry = history.entries().unwrap().remove(0);
        std::fs::remove_file(entry.path.as_deref().unwrap()).unwrap();

        assert!(matches!(
            extension.perform_action(&entry.id, ACTION_RESTORE),
            ActionOutcome::Failed(_)
        ));
        assert!(history.entries().unwrap().is_empty());
    }
}
