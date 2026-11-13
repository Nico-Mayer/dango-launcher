//! The `screenshots` built-in extension: the screenshots the user saved, in a
//! grid, pasted or copied from the keyboard.

mod folders;

use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

use crate::config::Config;
use crate::extension::{
    ActionOutcome, CommandDecl, Extension, FormValues, InvocationMode, Manifest, NAMED_ICON,
};
use crate::invocation::{Command, InvocationContext};
use crate::platform::{self, Label};
use crate::protocol::{
    Action, EmptyState, GridItem, GridView, Modifier, Shortcut, TileContent, View, ViewTree,
    PROTOCOL_VERSION,
};

pub use folders::{Scan, Screenshot};

pub const EXTENSION_ID: &str = "dango.screenshots";

pub const COMMAND_SEARCH: &str = "search";
pub const COMMAND_PASTE_LATEST: &str = "paste-latest";
pub const COMMAND_COPY_LATEST: &str = "copy-latest";

pub const ACTION_OPEN: &str = "open";
pub const ACTION_REVEAL: &str = "reveal";
pub const ACTION_COPY_PATH: &str = "copy-path";

const ICON: &str = "image";

/// What the extension does to a file outside Dango, so tests can stand in.
pub trait Files: Send + Sync {
    fn open(&self, path: &Path) -> Result<(), String>;
    fn reveal(&self, path: &Path) -> std::io::Result<()>;
}

pub struct SystemFiles;

impl Files for SystemFiles {
    fn open(&self, path: &Path) -> Result<(), String> {
        open::that_detached(path).map_err(|error| error.to_string())
    }

    fn reveal(&self, path: &Path) -> std::io::Result<()> {
        platform::reveal(path)
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
}

impl Shared {
    fn scan(&self) -> Scan {
        let configured = self.config.read().unwrap().screenshots.folders.clone();
        folders::scan(&folders::folders(&configured, (self.default_folder)()))
    }

    fn grid(&self) -> ViewTree {
        grid(&self.scan())
    }
}

pub struct ScreenshotsExtension {
    manifest: Manifest,
    shared: Arc<Shared>,
}

impl ScreenshotsExtension {
    pub fn new(config: Arc<RwLock<Config>>, files: Arc<dyn Files>) -> Self {
        Self::with_default_folder(config, files, platform::screenshot_folder)
    }

    fn with_default_folder(
        config: Arc<RwLock<Config>>,
        files: Arc<dyn Files>,
        default_folder: fn() -> Option<PathBuf>,
    ) -> Self {
        Self {
            manifest: manifest(),
            shared: Arc::new(Shared {
                config,
                default_folder,
                files,
            }),
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
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use folders::tests::file;
    use std::sync::Mutex;

    #[derive(Default)]
    struct FakeFiles {
        opened: Mutex<Vec<PathBuf>>,
        revealed: Mutex<Vec<PathBuf>>,
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
    }

    fn config_with(folders: &[&Path]) -> Arc<RwLock<Config>> {
        let mut config = Config::default();
        config.screenshots.folders = folders
            .iter()
            .map(|folder| folder.to_string_lossy().into_owned())
            .collect();
        Arc::new(RwLock::new(config))
    }

    fn extension(folders: &[&Path], files: Arc<FakeFiles>) -> ScreenshotsExtension {
        ScreenshotsExtension::with_default_folder(config_with(folders), files, || None)
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
        let tree = extension(&[dir.path()], Arc::default()).shared.grid();
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
        let tree = extension(&[dir.path()], Arc::default()).shared.grid();
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
        let tree = extension(&[&gone], Arc::default()).shared.grid();
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
        let extension = ScreenshotsExtension::with_default_folder(
            Arc::default(),
            Arc::new(FakeFiles::default()),
            || FOLDER.with(|folder| folder.borrow().clone()),
        );
        assert_eq!(extension.shared.scan().screenshots.len(), 1);
    }

    #[test]
    #[ignore = "reads this machine's screenshot setting"]
    fn the_system_screenshot_folder_exists() {
        let folder = platform::screenshot_folder().expect("a screenshot folder");
        assert!(folder.is_dir(), "{}", folder.display());
    }

    #[test]
    fn each_tile_offers_open_show_and_copy_path() {
        let titles: Vec<_> = actions().into_iter().map(|action| action.title).collect();
        let show = if cfg!(target_os = "macos") {
            "Show in Finder"
        } else {
            "Show in File Explorer"
        };
        assert_eq!(titles, ["Open", show, "Copy path"]);
    }

    #[test]
    fn open_and_show_reach_the_file() {
        let files = Arc::new(FakeFiles::default());
        let extension = extension(&[], files.clone());
        let path = "/shots/a.png";
        let values = FormValues::default();
        assert!(matches!(
            extension.perform_action(path, ACTION_OPEN, &values),
            ActionOutcome::Done
        ));
        assert!(matches!(
            extension.perform_action(path, ACTION_REVEAL, &values),
            ActionOutcome::Done
        ));
        assert_eq!(*files.opened.lock().unwrap(), [PathBuf::from(path)]);
        assert_eq!(*files.revealed.lock().unwrap(), [PathBuf::from(path)]);
    }

    #[test]
    fn copy_path_copies_the_full_path() {
        let outcome = extension(&[], Arc::default()).perform_action(
            "/shots/a.png",
            ACTION_COPY_PATH,
            &FormValues::default(),
        );
        assert!(matches!(outcome, ActionOutcome::CopyToClipboard(path) if path == "/shots/a.png"));
    }
}
