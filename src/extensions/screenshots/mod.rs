//! The `screenshots` built-in extension: the screenshots the user saved, in a
//! grid, pasted or copied from the keyboard.

mod folders;

use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

use crate::config::Config;
use crate::extension::{
    ActionOutcome, CommandDecl, Extension, FormValues, InvocationMode, Manifest, NAMED_ICON,
};
use crate::extensions::clipboard::ClipboardSource;
use crate::invocation::{Command, InvocationContext};
use crate::platform::{self, Label};
use crate::protocol::{
    Action, ActionStyle, EmptyState, GridItem, GridView, Modifier, Shortcut, TileContent, View,
    ViewTree, PROTOCOL_VERSION,
};
use crate::text::{TextError, TextTarget};

pub use folders::{Scan, Screenshot};

pub const EXTENSION_ID: &str = "dango.screenshots";

pub const COMMAND_SEARCH: &str = "search";
pub const COMMAND_PASTE_LATEST: &str = "paste-latest";
pub const COMMAND_COPY_LATEST: &str = "copy-latest";

pub const ACTION_PASTE: &str = "paste";
pub const ACTION_COPY: &str = "copy";
pub const ACTION_OPEN: &str = "open";
pub const ACTION_REVEAL: &str = "reveal";
pub const ACTION_COPY_PATH: &str = "copy-path";
pub const ACTION_TRASH: &str = "trash";

const ICON: &str = "image";

/// What the extension does to a file outside Dango, so tests can stand in.
pub trait Files: Send + Sync {
    fn open(&self, path: &Path) -> Result<(), String>;
    fn reveal(&self, path: &Path) -> std::io::Result<()>;
    fn trash(&self, path: &Path) -> Result<(), String>;
}

struct SystemFiles;

impl Files for SystemFiles {
    fn open(&self, path: &Path) -> Result<(), String> {
        open::that_detached(path).map_err(|error| error.to_string())
    }

    fn reveal(&self, path: &Path) -> std::io::Result<()> {
        platform::reveal(path)
    }

    fn trash(&self, path: &Path) -> Result<(), String> {
        trash::delete(path).map_err(|error| error.to_string())
    }
}

pub(crate) fn manifest() -> Manifest {
    let command = |id: &str, title: &str, mode, keywords: &[&str]| CommandDecl {
        id: id.into(),
        title: title.into(),
        mode,
        icon: Some(format!("{NAMED_ICON}{ICON}")),
        keywords: keywords.iter().map(|k| (*k).to_string()).collect(),
        alias: None,
        repeats: false,
    };
    Manifest {
        manifest_version: 1,
        id: EXTENSION_ID.into(),
        name: "Screenshots".into(),
        icon: Some(format!("{NAMED_ICON}{ICON}")),
        commands: vec![
            command(
                COMMAND_SEARCH,
                "Search Screenshots",
                InvocationMode::View,
                &["screenshot", "screen", "capture", "image"],
            ),
            command(
                COMMAND_PASTE_LATEST,
                "Paste Latest Screenshot",
                InvocationMode::NoView,
                &["screenshot", "screen", "capture", "image", "paste"],
            ),
            command(
                COMMAND_COPY_LATEST,
                "Copy Latest Screenshot",
                InvocationMode::NoView,
                &["screenshot", "screen", "capture", "image", "copy"],
            ),
        ],
        root_items: false,
        services: false,
    }
}

/// Everything a command or action needs, shared by the extension and the
/// commands it hands out.
struct Shared {
    config: Arc<RwLock<Config>>,
    default_folder: fn() -> Option<PathBuf>,
    files: Arc<dyn Files>,
    clipboard: Option<Arc<dyn ClipboardSource>>,
    text: Option<Arc<dyn TextTarget>>,
}

impl Shared {
    fn scan(&self) -> Scan {
        let configured = self.config.read().unwrap().screenshots.folders.clone();
        folders::scan(&folders::folders(&configured, (self.default_folder)()))
    }

    fn grid(&self) -> ViewTree {
        grid(&self.scan())
    }

    fn latest(&self) -> Result<PathBuf, String> {
        self.scan()
            .screenshots
            .into_iter()
            .next()
            .map(|screenshot| screenshot.path)
            .ok_or_else(|| "No screenshots found in your screenshot folders".into())
    }

    /// The file is read and checked before the launcher hides, so a missing
    /// or broken file fails with the launcher still up.
    fn paste(&self, path: &Path) -> Result<(), String> {
        let text = self
            .text
            .as_ref()
            .ok_or_else(|| TextError::Unavailable.to_string())?;
        let png = png_of(path)?;
        text.paste_image_file(&png, path)
            .map_err(|error| error.to_string())
    }

    fn copy(&self, path: &Path) -> Result<(), String> {
        let clipboard = self
            .clipboard
            .as_ref()
            .ok_or_else(|| TextError::Clipboard.to_string())?;
        let png = png_of(path)?;
        clipboard.set_image_file(&png, path).map_err(|error| {
            crate::log::append(&format!("could not copy {}: {error}", path.display()));
            TextError::Clipboard.to_string()
        })
    }
}

/// The file as PNG bytes. A PNG goes through as it is once its header reads;
/// any other format is converted.
fn png_of(path: &Path) -> Result<Vec<u8>, String> {
    let unreadable = |error: &dyn std::fmt::Display| {
        crate::log::append(&format!("could not read {}: {error}", path.display()));
        "Couldn't read the screenshot".to_string()
    };
    let bytes = match std::fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Err("That screenshot no longer exists".into())
        }
        Err(error) => return Err(unreadable(&error)),
    };
    let format = image::guess_format(&bytes).map_err(|error| unreadable(&error))?;
    if format == image::ImageFormat::Png {
        image::ImageReader::with_format(std::io::Cursor::new(&bytes), format)
            .into_dimensions()
            .map_err(|error| unreadable(&error))?;
        return Ok(bytes);
    }
    let image =
        image::load_from_memory_with_format(&bytes, format).map_err(|error| unreadable(&error))?;
    let mut png = Vec::new();
    image
        .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
        .map_err(|error| unreadable(&error))?;
    Ok(png)
}

pub struct ScreenshotsExtension {
    manifest: Manifest,
    shared: Arc<Shared>,
}

impl ScreenshotsExtension {
    pub fn new(
        config: Arc<RwLock<Config>>,
        clipboard: Option<Arc<dyn ClipboardSource>>,
        text: Option<Arc<dyn TextTarget>>,
    ) -> Self {
        Self::from_shared(Shared {
            config,
            default_folder: platform::screenshot_folder,
            files: Arc::new(SystemFiles),
            clipboard,
            text,
        })
    }

    fn from_shared(shared: Shared) -> Self {
        Self {
            manifest: manifest(),
            shared: Arc::new(shared),
        }
    }
}

impl Extension for ScreenshotsExtension {
    fn manifest(&self) -> &Manifest {
        &self.manifest
    }

    fn command(&self, command_id: &str) -> Option<Arc<dyn Command>> {
        match command_id {
            COMMAND_SEARCH => Some(Arc::new(SearchCommand(self.shared.clone()))),
            COMMAND_PASTE_LATEST => Some(Arc::new(LatestCommand {
                shared: self.shared.clone(),
                paste: true,
            })),
            COMMAND_COPY_LATEST => Some(Arc::new(LatestCommand {
                shared: self.shared.clone(),
                paste: false,
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
        let path = Path::new(item_id);
        let files = &self.shared.files;
        match action_id {
            ACTION_PASTE => match self.shared.paste(path) {
                Ok(()) => ActionOutcome::Done,
                Err(message) => ActionOutcome::Failed(message),
            },
            ACTION_COPY => match self.shared.copy(path) {
                Ok(()) => ActionOutcome::Copied,
                Err(message) => ActionOutcome::Failed(message),
            },
            ACTION_OPEN => match files.open(path) {
                Ok(()) => ActionOutcome::Done,
                Err(error) => {
                    crate::log::append(&format!("could not open {item_id}: {error}"));
                    ActionOutcome::Failed("Couldn't open the screenshot".into())
                }
            },
            ACTION_REVEAL => match files.reveal(path) {
                Ok(()) => ActionOutcome::Done,
                Err(error) => {
                    crate::log::append(&format!("could not reveal {item_id}: {error}"));
                    ActionOutcome::Failed(format!(
                        "Couldn't show the screenshot in {}",
                        platform::label(Label::FileManager)
                    ))
                }
            },
            ACTION_COPY_PATH => ActionOutcome::CopyToClipboard(item_id.into()),
            ACTION_TRASH => match files.trash(path) {
                Ok(()) => ActionOutcome::Replaced(Box::new(self.shared.grid())),
                Err(error) => {
                    crate::log::append(&format!("could not trash {item_id}: {error}"));
                    ActionOutcome::Failed(format!(
                        "Couldn't move the screenshot to the {}",
                        platform::label(Label::Trash)
                    ))
                }
            },
            _ => ActionOutcome::Failed("That action isn't available".into()),
        }
    }
}

struct SearchCommand(Arc<Shared>);

impl Command for SearchCommand {
    fn invoke(&self, ctx: &InvocationContext) {
        ctx.push_view(self.0.grid());
    }
}

struct LatestCommand {
    shared: Arc<Shared>,
    paste: bool,
}

impl Command for LatestCommand {
    fn invoke(&self, ctx: &InvocationContext) {
        let path = match self.shared.latest() {
            Ok(path) => path,
            Err(message) => return ctx.fail(message),
        };
        let done = if self.paste {
            self.shared.paste(&path)
        } else {
            self.shared.copy(&path)
        };
        match (done, self.paste) {
            (Ok(()), true) => ctx.succeed(),
            (Ok(()), false) => ctx.succeed_with("Copied to clipboard"),
            (Err(message), _) => ctx.fail(message),
        }
    }
}

/// Four columns of 16:10 tiles, about the shape of a screen.
pub fn grid(scan: &Scan) -> ViewTree {
    let empty_state = if scan.any_folder {
        EmptyState {
            title: "No screenshots yet".into(),
            description: Some("Screenshots you save will show up here".into()),
            ..Default::default()
        }
    } else {
        EmptyState {
            title: "Screenshot folder not found".into(),
            description: Some("Check the screenshot folders in the config file".into()),
            ..Default::default()
        }
    };
    ViewTree {
        protocol_version: PROTOCOL_VERSION,
        view: View::Grid(GridView {
            columns: 4,
            aspect: 16.0 / 10.0,
            empty_state: Some(empty_state),
            items: scan
                .screenshots
                .iter()
                .map(|screenshot| GridItem {
                    id: screenshot.path.to_string_lossy().into_owned(),
                    title: screenshot.name(),
                    content: TileContent::Image {
                        image: screenshot.path.to_string_lossy().into_owned(),
                    },
                    actions: actions(),
                    ..Default::default()
                })
                .collect(),
            ..Default::default()
        }),
    }
}

fn actions() -> Vec<Action> {
    let shortcut = |key: &str, modifiers: Vec<Modifier>| {
        Some(Shortcut {
            key: key.into(),
            modifiers,
        })
    };
    vec![
        Action {
            id: ACTION_PASTE.into(),
            title: "Paste".into(),
            ..Default::default()
        },
        Action {
            id: ACTION_COPY.into(),
            title: "Copy".into(),
            ..Default::default()
        },
        Action {
            id: ACTION_OPEN.into(),
            title: "Open".into(),
            shortcut: shortcut("o", vec![Modifier::Ctrl]),
            ..Default::default()
        },
        Action {
            id: ACTION_REVEAL.into(),
            title: format!("Show in {}", platform::label(Label::FileManager)),
            shortcut: shortcut("r", vec![Modifier::Ctrl]),
            ..Default::default()
        },
        Action {
            id: ACTION_COPY_PATH.into(),
            title: "Copy path".into(),
            shortcut: shortcut("c", vec![Modifier::Ctrl, Modifier::Shift]),
            ..Default::default()
        },
        Action {
            id: ACTION_TRASH.into(),
            title: format!("Move to {}", platform::label(Label::Trash)),
            shortcut: shortcut("x", vec![Modifier::Ctrl]),
            style: ActionStyle::Danger,
            ..Default::default()
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::invocation::{Outcome, Sink};
    use folders::tests::file;
    use std::sync::Mutex;

    #[derive(Default)]
    struct FakeFiles {
        opened: Mutex<Vec<PathBuf>>,
        revealed: Mutex<Vec<PathBuf>>,
        trash_fails: std::sync::atomic::AtomicBool,
    }

    impl Files for FakeFiles {
        fn open(&self, path: &Path) -> Result<(), String> {
            self.opened.lock().unwrap().push(path.into());
            Ok(())
        }
        fn reveal(&self, path: &Path) -> std::io::Result<()> {
            self.revealed.lock().unwrap().push(path.into());
            Ok(())
        }
        fn trash(&self, path: &Path) -> Result<(), String> {
            if self.trash_fails.load(std::sync::atomic::Ordering::SeqCst) {
                return Err("refused".into());
            }
            std::fs::remove_file(path).map_err(|error| error.to_string())
        }
    }

    type Written = Mutex<Vec<(Vec<u8>, PathBuf)>>;

    #[derive(Default)]
    struct FakeClipboard(Written);

    impl ClipboardSource for FakeClipboard {
        fn formats(&self) -> Option<Vec<String>> {
            None
        }
        fn text(&self) -> Option<String> {
            None
        }
        fn image(&self) -> Option<Vec<u8>> {
            None
        }
        fn files(&self) -> Option<Vec<String>> {
            None
        }
        fn set_text(&self, _text: &str) {}
        fn set_image(&self, _png: &[u8]) {}
        fn set_files(&self, _paths: &[String]) {}
        fn set_image_file(&self, png: &[u8], path: &Path) -> Result<(), String> {
            self.0.lock().unwrap().push((png.to_vec(), path.into()));
            Ok(())
        }
    }

    #[derive(Default)]
    struct FakeText(Written);

    impl TextTarget for FakeText {
        fn insert(&self, _text: &str, _caret: Option<usize>) -> Result<(), TextError> {
            unreachable!()
        }
        fn expand(&self, _: usize, _: &str, _: Option<usize>) -> Result<(), TextError> {
            unreachable!()
        }
        fn paste_content(
            &self,
            _content: &crate::extensions::clipboard::Content,
        ) -> Result<(), TextError> {
            unreachable!()
        }
        fn paste_image_file(&self, png: &[u8], path: &Path) -> Result<(), TextError> {
            self.0.lock().unwrap().push((png.to_vec(), path.into()));
            Ok(())
        }
        fn selection(&self) -> Result<Option<String>, TextError> {
            unreachable!()
        }
        fn clipboard_text(&self) -> Option<String> {
            None
        }
    }

    #[derive(Default)]
    struct RecordingSink(Mutex<Option<Outcome>>);

    impl Sink for RecordingSink {
        fn push_view(&self, _tree: ViewTree) {}
        fn finish(&self, outcome: Outcome) {
            *self.0.lock().unwrap() = Some(outcome);
        }
    }

    struct Fixture {
        extension: ScreenshotsExtension,
        files: Arc<FakeFiles>,
        clipboard: Arc<FakeClipboard>,
        text: Arc<FakeText>,
    }

    impl Fixture {
        fn run(&self, command_id: &str) -> Outcome {
            let sink = Arc::new(RecordingSink::default());
            let ctx = InvocationContext::for_test(command_id, sink.clone());
            self.extension.command(command_id).unwrap().invoke(&ctx);
            let outcome = sink.0.lock().unwrap().take();
            outcome.expect("the command finished")
        }

        fn act(&self, path: &Path, action_id: &str) -> ActionOutcome {
            self.extension.perform_action(
                &path.to_string_lossy(),
                action_id,
                &FormValues::default(),
            )
        }

        fn copied(&self) -> Vec<(Vec<u8>, PathBuf)> {
            self.clipboard.0.lock().unwrap().clone()
        }

        fn pasted(&self) -> Vec<(Vec<u8>, PathBuf)> {
            self.text.0.lock().unwrap().clone()
        }
    }

    fn fixture(folders: &[&Path]) -> Fixture {
        let mut config = Config::default();
        config.screenshots.folders = folders
            .iter()
            .map(|folder| folder.to_string_lossy().into_owned())
            .collect();
        let files = Arc::new(FakeFiles::default());
        let clipboard = Arc::new(FakeClipboard::default());
        let text = Arc::new(FakeText::default());
        let extension = ScreenshotsExtension::from_shared(Shared {
            config: Arc::new(RwLock::new(config)),
            default_folder: || None,
            files: files.clone(),
            clipboard: Some(clipboard.clone()),
            text: Some(text.clone()),
        });
        Fixture {
            extension,
            files,
            clipboard,
            text,
        }
    }

    fn png(folder: &Path, name: &str, age_seconds: u64) -> PathBuf {
        let path = folder.join(name);
        image::RgbImage::from_pixel(4, 3, image::Rgb([10, 20, 30]))
            .save(&path)
            .unwrap();
        let file = std::fs::File::options().write(true).open(&path).unwrap();
        file.set_modified(
            std::time::SystemTime::now() - std::time::Duration::from_secs(age_seconds),
        )
        .unwrap();
        path
    }

    fn grid_view(tree: &ViewTree) -> &GridView {
        match &tree.view {
            View::Grid(grid) => grid,
            other => panic!("expected a grid, got {other:?}"),
        }
    }

    #[test]
    fn the_grid_opens_newest_first() {
        let dir = tempfile::tempdir().unwrap();
        let old = file(dir.path(), "old.png", 60);
        let new = file(dir.path(), "new.png", 0);
        let tree = fixture(&[dir.path()]).extension.shared.grid();
        let grid = grid_view(&tree);
        assert_eq!((grid.columns, grid.aspect), (4, 16.0 / 10.0));
        let ids: Vec<_> = grid.items.iter().map(|item| item.id.clone()).collect();
        assert_eq!(ids, [new.to_string_lossy(), old.to_string_lossy()]);
        assert_eq!(grid.items[0].title, "new.png");
        assert_eq!(
            grid.items[0].content,
            TileContent::Image {
                image: new.to_string_lossy().into_owned()
            }
        );
    }

    #[test]
    fn an_empty_folder_says_no_screenshots_yet() {
        let dir = tempfile::tempdir().unwrap();
        let tree = fixture(&[dir.path()]).extension.shared.grid();
        let empty = grid_view(&tree).empty_state.clone().unwrap();
        assert_eq!(empty.title, "No screenshots yet");
        assert_eq!(
            empty.description.as_deref(),
            Some("Screenshots you save will show up here")
        );
    }

    #[test]
    fn no_folder_points_at_the_config_file() {
        let dir = tempfile::tempdir().unwrap();
        let gone = dir.path().join("gone");
        let tree = fixture(&[&gone]).extension.shared.grid();
        let empty = grid_view(&tree).empty_state.clone().unwrap();
        assert_eq!(empty.title, "Screenshot folder not found");
        assert_eq!(
            empty.description.as_deref(),
            Some("Check the screenshot folders in the config file")
        );
    }

    #[test]
    fn without_folders_the_system_folder_is_used() {
        let dir = tempfile::tempdir().unwrap();
        file(dir.path(), "shot.png", 0);
        thread_local!(static FOLDER: std::cell::RefCell<Option<PathBuf>> = Default::default());
        FOLDER.with(|folder| *folder.borrow_mut() = Some(dir.path().into()));
        let extension = ScreenshotsExtension::from_shared(Shared {
            config: Arc::default(),
            default_folder: || FOLDER.with(|folder| folder.borrow().clone()),
            files: Arc::new(FakeFiles::default()),
            clipboard: None,
            text: None,
        });
        assert_eq!(extension.shared.scan().screenshots.len(), 1);
    }

    #[test]
    #[ignore = "reads this machine's screenshot setting"]
    fn the_system_screenshot_folder_exists() {
        let folder = platform::screenshot_folder().expect("a screenshot folder");
        assert!(folder.is_dir(), "{}", folder.display());
    }

    #[test]
    fn each_tile_offers_paste_first_then_the_rest() {
        let titles: Vec<_> = actions().into_iter().map(|action| action.title).collect();
        let show = if cfg!(target_os = "macos") {
            "Show in Finder"
        } else {
            "Show in File Explorer"
        };
        let trash = if cfg!(target_os = "macos") {
            "Move to Trash"
        } else {
            "Move to Recycle Bin"
        };
        assert_eq!(titles, ["Paste", "Copy", "Open", show, "Copy path", trash]);
    }

    #[test]
    fn open_and_show_reach_the_file() {
        let f = fixture(&[]);
        let path = Path::new("/shots/a.png");
        assert!(matches!(f.act(path, ACTION_OPEN), ActionOutcome::Done));
        assert!(matches!(f.act(path, ACTION_REVEAL), ActionOutcome::Done));
        assert_eq!(*f.files.opened.lock().unwrap(), [path]);
        assert_eq!(*f.files.revealed.lock().unwrap(), [path]);
    }

    #[test]
    fn copy_path_copies_the_full_path() {
        let outcome = fixture(&[]).act(Path::new("/shots/a.png"), ACTION_COPY_PATH);
        assert!(matches!(outcome, ActionOutcome::CopyToClipboard(path) if path == "/shots/a.png"));
    }

    #[test]
    fn paste_hands_the_png_and_its_file_over() {
        let dir = tempfile::tempdir().unwrap();
        let path = png(dir.path(), "a.png", 0);
        let f = fixture(&[dir.path()]);
        assert!(matches!(f.act(&path, ACTION_PASTE), ActionOutcome::Done));
        assert_eq!(f.pasted(), [(std::fs::read(&path).unwrap(), path)]);
        assert!(f.copied().is_empty());
    }

    #[test]
    fn copy_puts_the_png_and_its_file_on_the_clipboard() {
        let dir = tempfile::tempdir().unwrap();
        let path = png(dir.path(), "a.png", 0);
        let f = fixture(&[dir.path()]);
        assert!(matches!(f.act(&path, ACTION_COPY), ActionOutcome::Copied));
        assert_eq!(f.copied(), [(std::fs::read(&path).unwrap(), path)]);
    }

    #[test]
    fn a_jpg_is_converted_to_png() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("photo.jpg");
        image::RgbImage::from_pixel(4, 3, image::Rgb([10, 20, 30]))
            .save(&path)
            .unwrap();
        let f = fixture(&[dir.path()]);
        assert!(matches!(f.act(&path, ACTION_COPY), ActionOutcome::Copied));
        let (bytes, _) = f.copied().remove(0);
        assert_eq!(
            image::guess_format(&bytes).unwrap(),
            image::ImageFormat::Png
        );
    }

    #[test]
    fn a_screenshot_deleted_meanwhile_is_reported_and_nothing_is_written() {
        let dir = tempfile::tempdir().unwrap();
        let gone = dir.path().join("gone.png");
        let f = fixture(&[dir.path()]);
        for action in [ACTION_PASTE, ACTION_COPY] {
            assert!(matches!(
                f.act(&gone, action),
                ActionOutcome::Failed(message) if message == "That screenshot no longer exists"
            ));
        }
        assert!(f.pasted().is_empty());
        assert!(f.copied().is_empty());
    }

    #[test]
    fn a_file_that_is_not_an_image_is_reported_and_nothing_is_written() {
        let dir = tempfile::tempdir().unwrap();
        let broken = file(dir.path(), "broken.png", 0);
        std::fs::write(&broken, b"not an image").unwrap();
        let f = fixture(&[dir.path()]);
        assert!(matches!(
            f.act(&broken, ACTION_PASTE),
            ActionOutcome::Failed(message) if message == "Couldn't read the screenshot"
        ));
        assert!(f.pasted().is_empty());
    }

    #[test]
    fn copy_latest_copies_the_newest_screenshot() {
        let dir = tempfile::tempdir().unwrap();
        png(dir.path(), "old.png", 60);
        let new = png(dir.path(), "new.png", 0);
        let f = fixture(&[dir.path()]);
        assert_eq!(
            f.run(COMMAND_COPY_LATEST),
            Outcome::Success(Some("Copied to clipboard".into()))
        );
        assert_eq!(f.copied()[0].1, new);
    }

    #[test]
    fn paste_latest_pastes_the_newest_screenshot() {
        let dir = tempfile::tempdir().unwrap();
        png(dir.path(), "old.png", 60);
        let new = png(dir.path(), "new.png", 0);
        let f = fixture(&[dir.path()]);
        assert_eq!(f.run(COMMAND_PASTE_LATEST), Outcome::Success(None));
        assert_eq!(f.pasted()[0].1, new);
    }

    #[test]
    fn with_no_screenshot_the_latest_commands_fail_and_write_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let f = fixture(&[dir.path()]);
        for command in [COMMAND_PASTE_LATEST, COMMAND_COPY_LATEST] {
            assert_eq!(
                f.run(command),
                Outcome::Failure("No screenshots found in your screenshot folders".into())
            );
        }
        assert!(f.pasted().is_empty());
        assert!(f.copied().is_empty());
    }

    #[test]
    fn a_trashed_screenshot_leaves_the_grid() {
        let dir = tempfile::tempdir().unwrap();
        let keep = file(dir.path(), "keep.png", 10);
        let gone = file(dir.path(), "gone.png", 0);
        let f = fixture(&[dir.path()]);
        let ActionOutcome::Replaced(tree) = f.act(&gone, ACTION_TRASH) else {
            panic!("the grid is shown again");
        };
        let ids: Vec<_> = grid_view(&tree)
            .items
            .iter()
            .map(|item| item.id.clone())
            .collect();
        assert_eq!(ids, [keep.to_string_lossy()]);
    }

    #[test]
    fn a_screenshot_that_cannot_be_trashed_stays() {
        let dir = tempfile::tempdir().unwrap();
        let path = file(dir.path(), "a.png", 0);
        let f = fixture(&[dir.path()]);
        f.files
            .trash_fails
            .store(true, std::sync::atomic::Ordering::SeqCst);
        let expected = if cfg!(target_os = "macos") {
            "Couldn't move the screenshot to the Trash"
        } else {
            "Couldn't move the screenshot to the Recycle Bin"
        };
        assert!(matches!(
            f.act(&path, ACTION_TRASH),
            ActionOutcome::Failed(message) if message == expected
        ));
        assert!(path.exists());
    }
}
