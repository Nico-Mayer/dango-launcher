//! The extension proper: the manifest, the command that opens the history, and
//! the policy that turns the `[clipboard]` settings into the bounds and
//! exclusions the watcher asks for on every change.

use std::path::Path;
use std::sync::{Arc, RwLock};

use crate::config::Config;
pub use crate::config::PrimaryAction;
use crate::extension::{
    ActionOutcome, CommandDecl, Extension, FormValues, InvocationMode, Manifest, Service,
    NAMED_ICON,
};
use crate::icon_cache::{IconCache, IconRgba, ICON_SIZE};
use crate::invocation::{Command, InvocationContext};
use crate::protocol::{
    Action, ActionStyle, EmptyState, ItemDetail, ListItem, ListView, Metadata, Modifier, Shortcut,
    View, ViewTree, PROTOCOL_VERSION,
};
use crate::text::{TextError, TextTarget};
use time::OffsetDateTime;

use super::history::{Bounds, Entry, History, Kind, LISTED_CHARS};
use super::metadata;
use super::watcher::{Policy, WatchService, Watcher};

pub const EXTENSION_ID: &str = "dango.clipboard";
pub const COMMAND_HISTORY: &str = "history";

pub const ACTION_INSERT: &str = "insert";
pub const ACTION_RESTORE: &str = "restore";
pub const ACTION_REMOVE: &str = "remove";

const MEGABYTE: usize = 1024 * 1024;

/// How much of a text entry is worth showing on one line.
const PREVIEW_CHARS: usize = 120;

pub struct ClipboardExtension {
    manifest: Manifest,
    history: Arc<History>,
    watcher: Arc<Watcher>,
    config: Arc<RwLock<Config>>,
    text: Option<Arc<dyn TextTarget>>,
    icons: Arc<IconCache>,
    render_icon: fn(&str) -> Option<IconRgba>,
}

impl ClipboardExtension {
    pub fn new(
        history: Arc<History>,
        watcher: Arc<Watcher>,
        config: Arc<RwLock<Config>>,
        text: Option<Arc<dyn TextTarget>>,
        icons: Arc<IconCache>,
    ) -> Self {
        Self {
            manifest: manifest(),
            history,
            watcher,
            config,
            text,
            icons,
            render_icon: render_file_icon,
        }
    }

    #[cfg(test)]
    fn with_icon_renderer(self, render_icon: fn(&str) -> Option<IconRgba>) -> Self {
        Self {
            render_icon,
            ..self
        }
    }

    fn list(&self) -> ViewTree {
        let primary = primary_action(&self.config);
        match self.history.entries() {
            Ok(entries) => history_view(&entries, primary, local_now(), &self.icons),
            Err(error) => {
                crate::log::append(&format!("could not read the clipboard history: {error}"));
                history_view(&[], primary, local_now(), &self.icons)
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
                config: self.config.clone(),
                icons: self.icons.clone(),
                render_icon: self.render_icon,
            })),
            _ => None,
        }
    }

    fn perform_action(
        &self,
        item_id: &str,
        action_id: &str,
        _values: &FormValues,
    ) -> ActionOutcome {
        match action_id {
            ACTION_INSERT => match (&self.text, self.history.content(item_id)) {
                (None, _) => ActionOutcome::Failed(TextError::Unavailable.to_string()),
                (Some(text), Ok(content)) => match text.paste_content(&content) {
                    Ok(()) => ActionOutcome::Done,
                    Err(error) => ActionOutcome::Failed(error.to_string()),
                },
                (_, Err(error)) => ActionOutcome::Failed(error.to_string()),
            },
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
            _ => ActionOutcome::Failed("That action isn't available".into()),
        }
    }
}

pub(crate) fn manifest() -> Manifest {
    Manifest {
        manifest_version: 1,
        id: EXTENSION_ID.into(),
        name: "Clipboard History".into(),
        icon: Some(format!("{NAMED_ICON}clipboard-list")),
        commands: vec![CommandDecl {
            id: COMMAND_HISTORY.into(),
            title: "Clipboard History".into(),
            mode: InvocationMode::View,
            icon: Some(format!("{NAMED_ICON}clipboard-list")),
            keywords: ["clipboard", "history", "paste", "copied"]
                .iter()
                .map(|k| (*k).to_string())
                .collect(),
            alias: None,
        }],
        root_items: false,
        services: true,
    }
}

/// Turns the `[clipboard]` settings into what the watcher asks for on every
/// change, so editing a value takes effect without a restart.
pub struct ConfigPolicy(pub Arc<RwLock<Config>>);

impl Policy for ConfigPolicy {
    fn bounds(&self) -> Bounds {
        let fallback = Bounds::default();
        let config = self.0.read().unwrap();
        let settings = &config.clipboard;
        Bounds {
            entries: settings.entries as usize,
            entry_bytes: megabytes(settings.entry_megabytes, fallback.entry_bytes),
            total_bytes: megabytes(settings.total_megabytes, fallback.total_bytes),
        }
    }

    fn excluded_applications(&self) -> Vec<String> {
        self.0
            .read()
            .unwrap()
            .clipboard
            .excluded_applications
            .clone()
    }
}

/// Zero megabytes would refuse every entry, so it reads as the default.
fn megabytes(value: u32, fallback: usize) -> usize {
    match value {
        0 => fallback,
        value => value as usize * MEGABYTE,
    }
}

fn primary_action(config: &RwLock<Config>) -> PrimaryAction {
    config.read().unwrap().clipboard.primary_action
}

struct HistoryCommand {
    history: Arc<History>,
    config: Arc<RwLock<Config>>,
    icons: Arc<IconCache>,
    render_icon: fn(&str) -> Option<IconRgba>,
}

impl Command for HistoryCommand {
    /// Shows the history before rendering missing file icons, which can wait
    /// on a busy shell, then shows it again with them. The second view reads
    /// the history again, so an entry removed in between stays removed.
    fn invoke(&self, ctx: &InvocationContext) {
        let primary = primary_action(&self.config);
        let entries = match self.history.entries() {
            Ok(entries) => entries,
            Err(error) => return ctx.fail(error.to_string()),
        };
        ctx.push_view(history_view(&entries, primary, local_now(), &self.icons));
        if !self.icons.fill(file_icons(&entries), self.render_icon, || {
            ctx.is_cancelled()
        }) {
            return;
        }
        if let Ok(entries) = self.history.entries() {
            ctx.push_view(history_view(&entries, primary, local_now(), &self.icons));
        }
    }
}

fn local_now() -> OffsetDateTime {
    OffsetDateTime::now_local().unwrap_or_else(|_| OffsetDateTime::now_utc())
}

pub fn history_view(
    entries: &[Entry],
    primary: PrimaryAction,
    now: OffsetDateTime,
    icons: &IconCache,
) -> ViewTree {
    ViewTree {
        protocol_version: PROTOCOL_VERSION,
        view: View::List(ListView {
            loading: false,
            empty_state: Some(EmptyState {
                title: "Nothing copied yet".into(),
                description: Some("Text, images, and files you copy will show up here".into()),
                ..Default::default()
            }),
            items: entries
                .iter()
                .map(|entry| item(entry, primary, now, icons))
                .collect(),
            detail_pane: true,
            ..Default::default()
        }),
    }
}

fn item(entry: &Entry, primary: PrimaryAction, now: OffsetDateTime, icons: &IconCache) -> ListItem {
    let files = entry.files();
    let (title, icon) = match entry.kind {
        Kind::Text => (
            preview(entry.text.as_deref().unwrap_or_default()),
            Some(format!("{NAMED_ICON}clipboard-list")),
        ),
        // The thumbnail is the entry's own file, which is the only way to tell
        // one copied image from another.
        Kind::Image => (
            "Image".to_string(),
            entry.thumbnail.clone().or_else(|| entry.path.clone()),
        ),
        Kind::Files => (files_title(&files), Some(file_icon(&files, icons))),
    };
    let paste = Action {
        id: ACTION_INSERT.into(),
        title: "Paste".into(),
        shortcut: None,
        ..Default::default()
    };
    let copy = Action {
        id: ACTION_RESTORE.into(),
        title: "Copy".into(),
        shortcut: None,
        ..Default::default()
    };
    let remove = Action {
        id: ACTION_REMOVE.into(),
        title: "Delete from history".into(),
        shortcut: Some(Shortcut {
            key: "x".into(),
            modifiers: vec![Modifier::Ctrl],
        }),
        style: ActionStyle::Danger,
        ..Default::default()
    };
    let actions = match primary {
        PrimaryAction::Paste => vec![paste, copy, remove],
        PrimaryAction::Copy => vec![copy, paste, remove],
    };
    ListItem {
        id: entry.id.clone(),
        title,
        subtitle: None,
        icon,
        keywords: files
            .iter()
            .map(|path| file_name(path).to_string())
            .collect(),
        detail: Some(detail(entry, now)),
        actions,
        ..Default::default()
    }
}

fn detail(entry: &Entry, now: OffsetDateTime) -> ItemDetail {
    let pair = |label: &str, value: String| Metadata {
        label: label.into(),
        value,
    };
    let mut metadata = Vec::new();
    if let Some(source) = &entry.source {
        metadata.push(pair("Source", source.clone()));
    }
    metadata.push(pair("Copied", metadata::copied(entry.copied_at, now)));
    match entry.kind {
        Kind::Text => {
            let text = entry.text.as_deref().unwrap_or_default();
            metadata.push(pair("Type", metadata::text_kind(text).into()));
            if let Some(lines) = entry.lines.filter(|lines| *lines > 1) {
                metadata.push(pair("Lines", lines.to_string()));
            }
            let characters = entry.characters.unwrap_or_default();
            metadata.push(pair("Characters", characters.to_string()));
            let cut = characters as usize > LISTED_CHARS;
            ItemDetail {
                text: Some(if cut {
                    format!("{text}…")
                } else {
                    text.into()
                }),
                metadata,
                ..Default::default()
            }
        }
        Kind::Files => {
            let files = entry.files();
            let several = files.len() > 1;
            metadata.push(pair("Type", if several { "Files" } else { "File" }.into()));
            if several {
                metadata.push(pair("Count", files.len().to_string()));
            }
            ItemDetail {
                text: Some(files.join("\n")),
                metadata,
                ..Default::default()
            }
        }
        Kind::Image => {
            metadata.push(pair("Type", "Image".into()));
            if let Some((width, height)) = entry.size {
                metadata.push(pair("Size", format!("{width} × {height}")));
            }
            ItemDetail {
                image: entry.pane_image(),
                metadata,
                ..Default::default()
            }
        }
    }
}

fn files_title(files: &[&str]) -> String {
    let first = files
        .first()
        .map(|path| file_name(path))
        .unwrap_or_default();
    match files.len() {
        0 | 1 => first.to_string(),
        count => format!("{first} and {} more", count - 1),
    }
}

fn file_name(path: &str) -> &str {
    Path::new(path)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(path)
}

fn file_icon(files: &[&str], icons: &IconCache) -> String {
    files
        .first()
        .and_then(|path| icons.path(&icon_key(path)))
        .unwrap_or_else(|| format!("{NAMED_ICON}file"))
}

fn file_icons(entries: &[Entry]) -> Vec<(String, String)> {
    entries
        .iter()
        .filter(|entry| entry.kind == Kind::Files)
        .filter_map(|entry| {
            let path = entry.files().first()?.to_string();
            Some((icon_key(&path), path))
        })
        .collect()
}

/// Only runs for a type not cached yet, because checking a file on an offline
/// network share can take seconds.
fn render_file_icon(path: &str) -> Option<IconRgba> {
    Path::new(path)
        .exists()
        .then(|| crate::platform::icon_for(path, ICON_SIZE))
        .flatten()
}

/// Plain files share their type's icon, so the cache grows with the types
/// copied rather than with every path. Folders (usually without an
/// extension), applications and shortcuts have icons of their own.
fn icon_key(path: &str) -> String {
    const OWN_ICON: [&str; 3] = ["app", "exe", "lnk"];
    match Path::new(path).extension().and_then(|ext| ext.to_str()) {
        Some(ext) if !OWN_ICON.iter().any(|own| own.eq_ignore_ascii_case(ext)) => {
            format!("file-type-{}", ext.to_lowercase())
        }
        _ => path.to_string(),
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
    use crate::extensions::clipboard::{ClipboardSource, Content, DEFAULT_EXCLUDED_APPLICATIONS};
    use std::sync::Mutex;

    /// Records what was put back on the clipboard, which is all the extension's
    /// tests need from one.
    #[derive(Default)]
    struct StubClipboard(Mutex<Option<Content>>);

    impl ClipboardSource for StubClipboard {
        fn formats(&self) -> Option<Vec<String>> {
            Some(Vec::new())
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
        fn files(&self) -> Option<Vec<String>> {
            None
        }
        fn set_image(&self, png: &[u8]) {
            *self.0.lock().unwrap() = Some(Content::Image(png.to_vec()));
        }
        fn set_files(&self, paths: &[String]) {
            *self.0.lock().unwrap() = Some(Content::Files(paths.to_vec()));
        }
    }

    struct NoAttribution;

    impl crate::extensions::clipboard::Attribution for NoAttribution {
        fn candidate_applications(&self) -> Vec<String> {
            Vec::new()
        }
    }

    fn shared(text: &str) -> Arc<RwLock<Config>> {
        Arc::new(RwLock::new(Config::parse(text).unwrap()))
    }

    #[derive(Default)]
    struct FakeTarget {
        pasted: Mutex<Vec<Content>>,
        fails: bool,
    }

    impl TextTarget for FakeTarget {
        fn insert(&self, _text: &str, _caret: Option<usize>) -> Result<(), TextError> {
            unreachable!("the history never inserts rendered text")
        }
        fn expand(
            &self,
            _backspaces: usize,
            _text: &str,
            _caret: Option<usize>,
        ) -> Result<(), TextError> {
            unreachable!("the history never expands a keyword")
        }
        fn paste_content(&self, content: &Content) -> Result<(), TextError> {
            if self.fails {
                return Err(TextError::PermissionMissing);
            }
            self.pasted.lock().unwrap().push(content.clone());
            Ok(())
        }
        fn selection(&self) -> Result<Option<String>, TextError> {
            Ok(None)
        }
        fn clipboard_text(&self) -> Option<String> {
            None
        }
    }

    struct Fixture {
        extension: ClipboardExtension,
        history: Arc<History>,
        clipboard: Arc<StubClipboard>,
        config: Arc<RwLock<Config>>,
        target: Arc<FakeTarget>,
    }

    fn build(target: Option<Arc<FakeTarget>>) -> Fixture {
        let dir = std::env::temp_dir().join(format!("dango-ext-{}", uuid::Uuid::new_v4()));
        let history = Arc::new(History::new(
            Arc::new(crate::store::Store::in_memory().unwrap()),
            dir,
        ));
        let clipboard = Arc::new(StubClipboard::default());
        let config = shared("");
        let watcher = Arc::new(Watcher::new(
            clipboard.clone(),
            Arc::new(NoAttribution),
            history.clone(),
            Arc::new(ConfigPolicy(config.clone())),
        ));
        watcher.start();
        let recorded = target.clone().unwrap_or_default();
        let text = target.map(|target| target as Arc<dyn TextTarget>);
        Fixture {
            extension: ClipboardExtension::new(
                history.clone(),
                watcher,
                config.clone(),
                text,
                IconCache::in_temp(),
            ),
            history,
            clipboard,
            config,
            target: recorded,
        }
    }

    fn setup() -> (ClipboardExtension, Arc<History>, Arc<StubClipboard>) {
        let f = build(Some(Arc::new(FakeTarget::default())));
        (f.extension, f.history, f.clipboard)
    }

    fn record_text(history: &History, text: &str) -> String {
        history
            .record(Content::Text(text.into()), None, Bounds::default(), 1)
            .unwrap();
        history.entries().unwrap()[0].id.clone()
    }

    fn action_ids(extension: &ClipboardExtension) -> Vec<String> {
        let tree = extension.list();
        list_of(&tree).items[0]
            .actions
            .iter()
            .map(|action| action.id.clone())
            .collect()
    }

    fn open_history(extension: &ClipboardExtension) -> Vec<ViewTree> {
        let sink = Arc::new(RecordingSink::default());
        let command = extension.command(COMMAND_HISTORY).unwrap();
        command.invoke(&InvocationContext::for_test(COMMAND_HISTORY, sink.clone()));
        let pushed = sink.0.lock().unwrap().clone();
        assert!(!pushed.is_empty(), "a view was pushed");
        pushed
    }

    fn list_of(tree: &ViewTree) -> &ListView {
        match &tree.view {
            View::List(list) => list,
            other => panic!("expected a list, got {other:?}"),
        }
    }

    #[test]
    fn the_declared_command_is_invocable() {
        let (extension, _, _) = setup();
        assert!(extension.command(COMMAND_HISTORY).is_some());
        assert!(extension.command("nope").is_none());
    }

    #[test]
    fn the_bounds_come_from_the_clipboard_table() {
        let policy = ConfigPolicy(shared(""));
        let bounds = policy.bounds();
        assert_eq!(bounds.entries, 500);
        assert_eq!(bounds.entry_bytes, 16 * 1024 * 1024);
        assert_eq!(bounds.total_bytes, 256 * 1024 * 1024);

        let policy = ConfigPolicy(shared(
            "[clipboard]
entries = 20
total-megabytes = 8",
        ));
        let bounds = policy.bounds();
        assert_eq!(bounds.entries, 20);
        assert_eq!(bounds.total_bytes, 8 * 1024 * 1024);
    }

    #[test]
    fn the_exclusion_list_defaults_to_the_known_managers() {
        let excluded = ConfigPolicy(shared("")).excluded_applications();
        assert!(excluded.contains(&"Proton Pass".to_string()));
        assert_eq!(excluded.len(), DEFAULT_EXCLUDED_APPLICATIONS.len());
    }

    #[test]
    fn the_exclusion_list_is_read_as_a_list() {
        let policy = ConfigPolicy(shared(
            r#"clipboard = { excluded-applications = ["Proton Pass", "Notes"] }"#,
        ));
        assert_eq!(
            policy.excluded_applications(),
            ["Proton Pass".to_string(), "Notes".to_string()]
        );
    }

    #[test]
    fn a_separated_string_is_not_an_exclusion_list() {
        let error = Config::parse(r#"clipboard = { excluded-applications = "1Password, Notes" }"#)
            .unwrap_err();
        assert!(
            error.to_string().contains("excluded-applications"),
            "{error}"
        );
    }

    #[test]
    fn an_unknown_primary_action_is_a_config_error() {
        let error = Config::parse(r#"clipboard = { primary-action = "drop" }"#).unwrap_err();
        assert!(error.to_string().contains("drop"), "{error}");
    }

    #[test]
    fn the_history_lists_newest_first() {
        let (extension, history, _) = setup();
        history
            .record(Content::Text("one".into()), None, Bounds::default(), 1)
            .unwrap();
        history
            .record(Content::Text("two".into()), None, Bounds::default(), 2)
            .unwrap();

        let tree = extension.list();
        let list = list_of(&tree);
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
    fn an_image_entry_shows_its_own_thumbnail() {
        let (extension, history, _) = setup();
        let png = std::fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/icons/icon.png")).unwrap();
        history
            .record(Content::Image(png), None, Bounds::default(), 1)
            .unwrap();

        let tree = extension.list();
        let item = &list_of(&tree).items[0];
        let path = item.icon.clone().expect("an image entry has a thumbnail");
        assert!(path.ends_with(".thumb.png"));
        assert!(std::path::Path::new(&path).exists());
    }

    #[test]
    fn an_image_entry_without_a_thumbnail_shows_the_full_image() {
        let (extension, history, _) = setup();
        history
            .record(Content::Image(vec![1, 2, 3]), None, Bounds::default(), 1)
            .unwrap();

        let tree = extension.list();
        let item = &list_of(&tree).items[0];
        let path = item.icon.clone().expect("an image entry has an icon");
        assert!(path.ends_with(".png") && !path.ends_with(".thumb.png"));
        assert!(std::path::Path::new(&path).exists());
    }

    fn record_files(history: &History, paths: &[&str]) -> String {
        let paths = paths.iter().map(|path| path.to_string()).collect();
        history
            .record(Content::Files(paths), None, Bounds::default(), 1)
            .unwrap();
        history.entries().unwrap()[0].id.clone()
    }

    fn rendered_icon(_path: &str) -> Option<IconRgba> {
        Some(IconRgba {
            width: 8,
            height: 8,
            rgba: vec![64; 8 * 8 * 4],
        })
    }

    #[test]
    fn a_file_entry_of_one_file_shows_its_name_and_system_icon() {
        let (extension, history, _) = setup();
        let extension = extension.with_icon_renderer(rendered_icon);
        record_files(&history, &["/a/report.pdf"]);

        let pushed = open_history(&extension);
        let first = &list_of(&pushed[0]).items[0];
        assert_eq!(first.title, "report.pdf");
        assert_eq!(
            first.icon.as_deref(),
            Some(format!("{NAMED_ICON}file").as_str()),
            "shown before the icon is rendered"
        );
        assert_eq!(pushed.len(), 2, "shown again once the icon exists");
        let icon = list_of(&pushed[1]).items[0].icon.clone().unwrap();
        assert!(Path::new(&icon).exists(), "the system icon: {icon}");
    }

    #[test]
    fn a_file_entry_of_several_files_is_found_by_any_name() {
        let (extension, history, _) = setup();
        record_files(
            &history,
            &["/no/such/report.pdf", "/no/such/a.txt", "/no/such/b.txt"],
        );

        let tree = extension.list();
        let item = &list_of(&tree).items[0];
        assert_eq!(item.title, "report.pdf and 2 more");
        assert_eq!(item.keywords, ["report.pdf", "a.txt", "b.txt"]);
    }

    #[test]
    fn files_of_one_type_share_an_icon_and_the_rest_keep_their_own() {
        assert_eq!(icon_key("/a/report.PDF"), "file-type-pdf");
        assert_eq!(icon_key("/b/other.pdf"), "file-type-pdf");
        assert_eq!(
            icon_key("/Applications/Safari.app"),
            "/Applications/Safari.app"
        );
        assert_eq!(icon_key(r"C:\Tools\app.EXE"), r"C:\Tools\app.EXE");
        assert_eq!(icon_key("/a/report_pdf"), "/a/report_pdf");
    }

    #[test]
    fn a_cached_type_icon_is_used_without_touching_the_file() {
        let (extension, history, _) = setup();
        let file = std::env::temp_dir().join(format!("dango-{}.dangotest", uuid::Uuid::new_v4()));
        std::fs::write(&file, "x").unwrap();
        record_files(&history, &[file.to_str().unwrap()]);
        let pushed = open_history(&extension);
        let cached = list_of(pushed.last().unwrap()).items[0]
            .icon
            .clone()
            .unwrap();

        std::fs::remove_file(&file).unwrap();
        let tree = extension.list();
        assert_eq!(
            list_of(&tree).items[0].icon.as_deref(),
            Some(cached.as_str())
        );
    }

    #[test]
    fn a_file_entry_whose_file_has_gone_shows_the_generic_icon() {
        let (extension, history, _) = setup();
        record_files(&history, &["/no/such/report.pdf"]);
        let tree = extension.list();
        assert_eq!(
            list_of(&tree).items[0].icon.as_deref(),
            Some(format!("{NAMED_ICON}file").as_str())
        );
    }

    #[test]
    fn copying_a_file_entry_puts_the_files_back() {
        let (extension, history, clipboard) = setup();
        let file = std::env::temp_dir().join(format!("dango-{}.txt", uuid::Uuid::new_v4()));
        std::fs::write(&file, "x").unwrap();
        let path = file.to_string_lossy().into_owned();
        let id = record_files(&history, &[&path]);

        assert!(matches!(
            extension.perform_action(&id, ACTION_RESTORE, &FormValues::new()),
            ActionOutcome::Done
        ));
        assert_eq!(
            *clipboard.0.lock().unwrap(),
            Some(Content::Files(vec![path]))
        );
    }

    #[test]
    fn a_file_entry_whose_files_have_gone_is_reported_and_dropped() {
        let f = build(Some(Arc::new(FakeTarget::default())));
        let id = record_files(&f.history, &["/no/such/one", "/no/such/two"]);

        for action in [ACTION_RESTORE, ACTION_INSERT] {
            let id = if action == ACTION_INSERT {
                record_files(&f.history, &["/no/such/three"])
            } else {
                id.clone()
            };
            let outcome = f.extension.perform_action(&id, action, &FormValues::new());
            let ActionOutcome::Failed(message) = outcome else {
                panic!("{action} must fail, got {outcome:?}");
            };
            assert!(message.starts_with("The copied file"), "{message}");
        }
        assert!(f.history.entries().unwrap().is_empty());
        assert_eq!(*f.clipboard.0.lock().unwrap(), None);
        assert!(f.target.pasted.lock().unwrap().is_empty());
    }

    const NOW: OffsetDateTime = time::macros::datetime!(2026-11-01 16:00 UTC);

    fn entry(kind: Kind) -> Entry {
        Entry {
            id: "e".into(),
            kind,
            text: None,
            path: None,
            thumbnail: None,
            source: Some("Notepad".into()),
            size: None,
            characters: None,
            lines: None,
            copied_at: (NOW.unix_timestamp() - 60) * 1000,
        }
    }

    fn pairs(detail: &ItemDetail) -> Vec<(&str, &str)> {
        detail
            .metadata
            .iter()
            .map(|pair| (pair.label.as_str(), pair.value.as_str()))
            .collect()
    }

    #[test]
    fn a_file_entry_shows_its_paths_and_count() {
        let several = Entry {
            text: Some("/a/report.pdf\n/a/notes.txt\n/a/c".into()),
            ..entry(Kind::Files)
        };
        let detail = detail(&several, NOW);
        assert_eq!(
            detail.text.as_deref(),
            Some("/a/report.pdf\n/a/notes.txt\n/a/c")
        );
        assert_eq!(
            pairs(&detail),
            [
                ("Source", "Notepad"),
                ("Copied", "Today, 15:59"),
                ("Type", "Files"),
                ("Count", "3")
            ]
        );

        let one = Entry {
            text: Some("/a/report.pdf".into()),
            ..entry(Kind::Files)
        };
        let detail = super::detail(&one, NOW);
        assert!(pairs(&detail).contains(&("Type", "File")));
        assert!(!pairs(&detail).iter().any(|(label, _)| *label == "Count"));
    }

    #[test]
    fn the_empty_history_mentions_files() {
        let tree = history_view(&[], PrimaryAction::Paste, NOW, &IconCache::in_temp());
        let description = list_of(&tree).empty_state.clone().unwrap().description;
        assert_eq!(
            description.as_deref(),
            Some("Text, images, and files you copy will show up here")
        );
    }

    #[test]
    fn the_history_shows_a_detail_pane() {
        let tree = history_view(
            &[entry(Kind::Text)],
            PrimaryAction::Paste,
            NOW,
            &IconCache::in_temp(),
        );
        let list = list_of(&tree);
        assert!(list.detail_pane);
        assert!(list.items[0].detail.is_some());
    }

    #[test]
    fn a_text_entry_shows_its_text_and_metadata() {
        let text = Entry {
            text: Some("one\n  two\nthree".into()),
            characters: Some(15),
            lines: Some(3),
            ..entry(Kind::Text)
        };
        let detail = detail(&text, NOW);
        assert_eq!(detail.text.as_deref(), Some("one\n  two\nthree"));
        assert_eq!(detail.image, None);
        assert_eq!(
            pairs(&detail),
            [
                ("Source", "Notepad"),
                ("Copied", "Today, 15:59"),
                ("Type", "Text"),
                ("Lines", "3"),
                ("Characters", "15"),
            ]
        );
    }

    #[test]
    fn a_single_line_has_no_line_count() {
        let link = Entry {
            text: Some("https://example.com".into()),
            characters: Some(19),
            lines: Some(1),
            ..entry(Kind::Text)
        };
        let detail = detail(&link, NOW);
        assert!(pairs(&detail).contains(&("Type", "Link")));
        assert!(!pairs(&detail).iter().any(|(label, _)| *label == "Lines"));
    }

    #[test]
    fn a_cut_text_ends_with_an_ellipsis_and_counts_in_full() {
        let long = Entry {
            text: Some("x".repeat(LISTED_CHARS)),
            characters: Some(5000),
            lines: Some(1),
            ..entry(Kind::Text)
        };
        let detail = detail(&long, NOW);
        assert!(detail.text.as_deref().unwrap().ends_with("x…"));
        assert!(pairs(&detail).contains(&("Characters", "5000")));
    }

    #[test]
    fn markup_stays_plain_text() {
        let markup = Entry {
            text: Some("# heading *stars*".into()),
            characters: Some(17),
            lines: Some(1),
            ..entry(Kind::Text)
        };
        let detail = detail(&markup, NOW);
        assert_eq!(detail.text.as_deref(), Some("# heading *stars*"));
        assert!(detail.markdown.is_empty());
    }

    #[test]
    fn an_image_entry_shows_its_image_and_size() {
        let image = Entry {
            path: Some("C:/img/a.png".into()),
            size: Some((1920, 1080)),
            ..entry(Kind::Image)
        };
        let detail = detail(&image, NOW);
        assert!(detail.image.as_deref().unwrap().ends_with("a.preview.png"));
        assert_eq!(detail.text, None);
        assert_eq!(
            pairs(&detail),
            [
                ("Source", "Notepad"),
                ("Copied", "Today, 15:59"),
                ("Type", "Image"),
                ("Size", "1920 × 1080"),
            ]
        );
    }

    #[test]
    fn an_entry_without_a_source_leaves_it_out() {
        let unknown = Entry {
            source: None,
            text: Some("hi".into()),
            characters: Some(2),
            lines: Some(1),
            ..entry(Kind::Text)
        };
        let detail = detail(&unknown, NOW);
        assert_eq!(pairs(&detail)[0], ("Copied", "Today, 15:59"));
    }

    #[test]
    fn enter_pastes_unless_told_otherwise() {
        let f = build(Some(Arc::new(FakeTarget::default())));
        record_text(&f.history, "hello");

        assert_eq!(
            action_ids(&f.extension),
            [ACTION_INSERT, ACTION_RESTORE, ACTION_REMOVE]
        );

        f.config.write().unwrap().clipboard.primary_action = PrimaryAction::Copy;
        assert_eq!(
            action_ids(&f.extension),
            [ACTION_RESTORE, ACTION_INSERT, ACTION_REMOVE],
            "read on every list build, so a config edit applies next time"
        );
    }

    #[test]
    fn the_command_builds_its_view_with_the_same_order() {
        let f = build(Some(Arc::new(FakeTarget::default())));
        record_text(&f.history, "hello");
        f.config.write().unwrap().clipboard.primary_action = PrimaryAction::Copy;

        let pushed = open_history(&f.extension);
        assert_eq!(list_of(&pushed[0]).items[0].actions[0].id, ACTION_RESTORE);
    }

    #[test]
    fn a_history_without_file_entries_is_shown_once() {
        let (extension, history, _) = setup();
        record_text(&history, "hello");
        assert_eq!(open_history(&extension).len(), 1);
    }

    #[test]
    fn pasting_a_text_entry_hands_it_to_the_target() {
        let f = build(Some(Arc::new(FakeTarget::default())));
        let id = record_text(&f.history, "hello");

        assert_eq!(
            f.extension
                .perform_action(&id, ACTION_INSERT, &FormValues::new()),
            ActionOutcome::Done
        );
        assert_eq!(
            *f.target.pasted.lock().unwrap(),
            vec![Content::Text("hello".into())]
        );
    }

    #[test]
    fn pasting_an_image_entry_hands_the_image_to_the_target() {
        let f = build(Some(Arc::new(FakeTarget::default())));
        f.history
            .record(Content::Image(vec![9, 9, 9]), None, Bounds::default(), 1)
            .unwrap();
        let id = f.history.entries().unwrap()[0].id.clone();

        assert_eq!(
            f.extension
                .perform_action(&id, ACTION_INSERT, &FormValues::new()),
            ActionOutcome::Done
        );
        assert_eq!(
            *f.target.pasted.lock().unwrap(),
            vec![Content::Image(vec![9, 9, 9])]
        );
    }

    #[test]
    fn a_paste_the_target_refuses_is_reported() {
        let f = build(Some(Arc::new(FakeTarget {
            fails: true,
            ..Default::default()
        })));
        let id = record_text(&f.history, "hello");

        assert_eq!(
            f.extension
                .perform_action(&id, ACTION_INSERT, &FormValues::new()),
            ActionOutcome::Failed(TextError::PermissionMissing.to_string())
        );
        assert_eq!(*f.clipboard.0.lock().unwrap(), None, "nothing was written");
    }

    #[test]
    fn pasting_without_key_injection_says_the_text_path_is_unavailable() {
        let f = build(None);
        let id = record_text(&f.history, "hello");

        assert_eq!(
            f.extension
                .perform_action(&id, ACTION_INSERT, &FormValues::new()),
            ActionOutcome::Failed(TextError::Unavailable.to_string())
        );
    }

    #[derive(Default)]
    struct RecordingSink(Mutex<Vec<ViewTree>>);

    impl crate::invocation::Sink for RecordingSink {
        fn push_view(&self, tree: ViewTree) {
            self.0.lock().unwrap().push(tree);
        }
        fn finish(&self, _outcome: crate::invocation::Outcome) {}
    }

    #[test]
    fn choosing_a_text_entry_puts_it_back_on_the_clipboard() {
        let (extension, history, clipboard) = setup();
        history
            .record(Content::Text("hello".into()), None, Bounds::default(), 1)
            .unwrap();
        let id = history.entries().unwrap()[0].id.clone();

        assert_eq!(
            extension.perform_action(&id, ACTION_RESTORE, &FormValues::new()),
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
            .record(Content::Image(vec![9, 9, 9]), None, Bounds::default(), 1)
            .unwrap();
        let id = history.entries().unwrap()[0].id.clone();

        assert_eq!(
            extension.perform_action(&id, ACTION_RESTORE, &FormValues::new()),
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
            .record(Content::Text("one".into()), None, Bounds::default(), 1)
            .unwrap();
        history
            .record(Content::Text("two".into()), None, Bounds::default(), 2)
            .unwrap();
        let id = history.entries().unwrap()[0].id.clone();

        let outcome = extension.perform_action(&id, ACTION_REMOVE, &FormValues::new());
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
            extension.perform_action("nope", ACTION_RESTORE, &FormValues::new()),
            ActionOutcome::Failed(_)
        ));
    }

    #[test]
    fn an_unknown_action_fails_cleanly() {
        let (extension, _, _) = setup();
        assert!(matches!(
            extension.perform_action("whatever", "nope", &FormValues::new()),
            ActionOutcome::Failed(_)
        ));
    }

    #[test]
    fn an_image_entry_whose_file_has_gone_is_reported_and_dropped() {
        let (extension, history, _) = setup();
        history
            .record(Content::Image(vec![4, 5]), None, Bounds::default(), 1)
            .unwrap();
        let entry = history.entries().unwrap().remove(0);
        std::fs::remove_file(entry.path.as_deref().unwrap()).unwrap();

        assert!(matches!(
            extension.perform_action(&entry.id, ACTION_RESTORE, &FormValues::new()),
            ActionOutcome::Failed(_)
        ));
        assert!(history.entries().unwrap().is_empty());
    }
}
