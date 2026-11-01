//! The `games` built-in extension: installed Steam games in root search and in
//! a grid of cover art, launched through Steam.

pub mod steam;

pub use steam::{Game, GameSource, SteamSource};

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};

use async_trait::async_trait;

use crate::extension::{
    ActionOutcome, ActivationResult, CommandDecl, Extension, FormValues, InvocationMode, Manifest,
    Service, NAMED_ICON,
};
use crate::extensions::snippets::OpenUrl;
use crate::invocation::{Command, InvocationContext};
use crate::protocol::{
    Action, EmptyState, GridItem, GridView, TileContent, View, ViewTree, PROTOCOL_VERSION,
};
use crate::search::{Candidate, RootProvider, Source};

pub const EXTENSION_ID: &str = "dango.games";

pub const COMMAND_SEARCH: &str = "search";

pub const ACTION_LAUNCH: &str = "launch";

const FALLBACK_ICON: &str = "gamepad-2";

#[derive(Debug, thiserror::Error)]
pub enum LaunchError {
    #[error("That game is no longer installed")]
    Gone,
    #[error("Couldn't start the game through Steam")]
    Steam,
}

pub(crate) fn manifest() -> Manifest {
    Manifest {
        manifest_version: 1,
        id: EXTENSION_ID.into(),
        name: "Games".into(),
        icon: Some(format!("{NAMED_ICON}{FALLBACK_ICON}")),
        commands: vec![CommandDecl {
            id: COMMAND_SEARCH.into(),
            title: "Search Games".into(),
            mode: InvocationMode::View,
            subtitle: Some("Games".into()),
            icon: Some(fallback_icon()),
            keywords: ["steam", "games", "play", "library"]
                .iter()
                .map(|k| (*k).to_string())
                .collect(),
            alias: None,
        }],
        preferences: vec![],
        root_items: true,
        services: true,
    }
}

/// The games as last read. Starts empty: reading is cheap, so the list is not
/// stored and root search shows games once the first read finishes.
pub struct GameList {
    games: RwLock<Vec<Game>>,
    source: Arc<dyn GameSource>,
}

impl GameList {
    pub fn new(source: Arc<dyn GameSource>) -> Arc<Self> {
        Arc::new(Self {
            games: RwLock::new(Vec::new()),
            source,
        })
    }

    pub fn snapshot(&self) -> Vec<Game> {
        self.games.read().unwrap().clone()
    }

    pub fn get(&self, app_id: &str) -> Option<Game> {
        self.games
            .read()
            .unwrap()
            .iter()
            .find(|game| game.app_id.to_string() == app_id)
            .cloned()
    }

    pub fn reload(&self) {
        let games = self.source.games();
        *self.games.write().unwrap() = games;
    }

    fn fingerprint(&self) -> Option<u64> {
        self.source.fingerprint()
    }
}

pub struct GamesExtension {
    manifest: Manifest,
    list: Arc<GameList>,
    opener: Arc<dyn OpenUrl>,
    provider: Arc<GameProvider>,
}

impl GamesExtension {
    pub fn new(list: Arc<GameList>, opener: Arc<dyn OpenUrl>) -> Self {
        Self {
            manifest: manifest(),
            provider: Arc::new(GameProvider(list.clone())),
            list,
            opener,
        }
    }

    /// Reads the list first: Steam takes a link to an uninstalled game as a
    /// request to install it, so a result shown before an uninstall must not
    /// reach it.
    fn launch(&self, app_id: &str) -> Result<(), LaunchError> {
        self.list.reload();
        let game = self.list.get(app_id).ok_or(LaunchError::Gone)?;
        self.opener
            .open(&format!("steam://rungameid/{}", game.app_id))
            .map_err(|error| {
                eprintln!(
                    "[dango] could not open {} through Steam: {error}",
                    game.name
                );
                LaunchError::Steam
            })
    }
}

impl Extension for GamesExtension {
    fn manifest(&self) -> &Manifest {
        &self.manifest
    }

    fn services(&self) -> Vec<Arc<dyn Service>> {
        vec![Arc::new(RefreshService {
            list: self.list.clone(),
            running: Arc::new(AtomicBool::new(false)),
        })]
    }

    fn root_provider(&self) -> Option<Arc<dyn RootProvider>> {
        Some(self.provider.clone())
    }

    fn command(&self, command_id: &str) -> Option<Arc<dyn Command>> {
        (command_id == COMMAND_SEARCH)
            .then(|| Arc::new(SearchCommand(self.list.clone())) as Arc<dyn Command>)
    }

    fn perform_action(
        &self,
        item_id: &str,
        action_id: &str,
        _values: &FormValues,
    ) -> ActionOutcome {
        if action_id != ACTION_LAUNCH {
            return ActionOutcome::Failed("That action isn't available".into());
        }
        match self.launch(item_id) {
            Ok(()) => ActionOutcome::Done,
            Err(error) => ActionOutcome::Failed(error.to_string()),
        }
    }
}

struct GameProvider(Arc<GameList>);

#[async_trait]
impl RootProvider for GameProvider {
    async fn items(&self, _query: String) -> Vec<Candidate> {
        self.0
            .snapshot()
            .into_iter()
            .map(|game| Candidate {
                extension_id: EXTENSION_ID.into(),
                id: game.app_id.to_string(),
                title: game.name,
                subtitle: Some("Steam".into()),
                icon: Some(game.icon.unwrap_or_else(fallback_icon)),
                keywords: vec![],
                alias: None,
                source: Source::RootItem,
                actions: vec![open()],
                match_positions: vec![],
                suggested: false,
                pinned: false,
            })
            .collect()
    }
}

struct SearchCommand(Arc<GameList>);

impl Command for SearchCommand {
    fn invoke(&self, ctx: &InvocationContext) {
        ctx.push_view(grid(&self.0.snapshot()));
    }
}

/// Five columns of Steam's 2/3 portrait capsules.
pub fn grid(games: &[Game]) -> ViewTree {
    ViewTree {
        protocol_version: PROTOCOL_VERSION,
        view: View::Grid(GridView {
            columns: 5,
            aspect: 2.0 / 3.0,
            empty_state: Some(EmptyState {
                title: "No Steam games installed".into(),
                description: Some("Games you install with Steam will show up here".into()),
                ..Default::default()
            }),
            items: games
                .iter()
                .map(|game| GridItem {
                    id: game.app_id.to_string(),
                    title: game.name.clone(),
                    content: TileContent::Image {
                        image: game.cover.clone().unwrap_or_else(fallback_icon),
                    },
                    actions: vec![open()],
                    ..Default::default()
                })
                .collect(),
            ..Default::default()
        }),
    }
}

fn open() -> Action {
    Action {
        id: ACTION_LAUNCH.into(),
        title: "Open".into(),
        ..Default::default()
    }
}

fn fallback_icon() -> String {
    format!("{NAMED_ICON}{FALLBACK_ICON}")
}

/// Reads the list off the activation path, then again whenever the library
/// folders change or the sweep is due.
struct RefreshService {
    list: Arc<GameList>,
    running: Arc<AtomicBool>,
}

impl Service for RefreshService {
    fn start(&self) -> ActivationResult {
        self.running.store(true, Ordering::SeqCst);
        let list = self.list.clone();
        let running = self.running.clone();
        std::thread::spawn(move || {
            while running.load(Ordering::SeqCst) {
                list.reload();
                super::refresh::wait_for_change(|| list.fingerprint(), &running);
            }
        });
        Ok(())
    }

    fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);
    }
}

#[cfg(test)]
pub mod fake {
    use super::*;
    use std::sync::atomic::AtomicU64;
    use std::sync::Mutex;

    #[derive(Default)]
    pub struct FakeOpener {
        pub opened: Mutex<Vec<String>>,
        pub fails: AtomicBool,
    }

    impl OpenUrl for FakeOpener {
        fn open(&self, url: &str) -> Result<(), String> {
            if self.fails.load(Ordering::SeqCst) {
                return Err("no handler".into());
            }
            self.opened.lock().unwrap().push(url.into());
            Ok(())
        }
    }

    #[derive(Default)]
    pub struct FakeSource {
        games: RwLock<Vec<Game>>,
        fingerprint: AtomicU64,
    }

    impl FakeSource {
        pub fn with(games: &[(u32, &str)]) -> Arc<Self> {
            let source = Arc::new(Self::default());
            source.set(games);
            source
        }

        /// Replaces the games and moves the fingerprint, as an install does.
        pub fn set(&self, games: &[(u32, &str)]) {
            *self.games.write().unwrap() = games
                .iter()
                .map(|(app_id, name)| Game {
                    app_id: *app_id,
                    name: (*name).into(),
                    icon: Some(format!("/cache/{app_id}/icon.jpg")),
                    cover: Some(format!("/cache/{app_id}/library_600x900.jpg")),
                })
                .collect();
            self.fingerprint.fetch_add(1, Ordering::SeqCst);
        }

        pub fn strip_art(&self) {
            for game in self.games.write().unwrap().iter_mut() {
                game.icon = None;
                game.cover = None;
            }
        }
    }

    impl GameSource for FakeSource {
        fn games(&self) -> Vec<Game> {
            self.games.read().unwrap().clone()
        }

        fn fingerprint(&self) -> Option<u64> {
            Some(self.fingerprint.load(Ordering::SeqCst))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::fake::{FakeOpener, FakeSource};
    use super::*;
    use std::time::{Duration, Instant};

    fn extension(source: Arc<FakeSource>) -> GamesExtension {
        launcher(source).0
    }

    fn launcher(source: Arc<FakeSource>) -> (GamesExtension, Arc<FakeOpener>) {
        let list = GameList::new(source);
        list.reload();
        let opener = Arc::new(FakeOpener::default());
        (GamesExtension::new(list, opener.clone()), opener)
    }

    fn launch(extension: &GamesExtension, app_id: &str) -> ActionOutcome {
        extension.perform_action(app_id, ACTION_LAUNCH, &FormValues::new())
    }

    async fn root_items(extension: &GamesExtension) -> Vec<Candidate> {
        extension
            .root_provider()
            .unwrap()
            .items(String::new())
            .await
    }

    #[tokio::test]
    async fn every_game_is_a_root_result_from_steam() {
        let ext = extension(FakeSource::with(&[(1091500, "Cyberpunk 2077")]));
        let items = root_items(&ext).await;
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].id, "1091500");
        assert_eq!(items[0].title, "Cyberpunk 2077");
        assert_eq!(items[0].subtitle.as_deref(), Some("Steam"));
        assert_eq!(items[0].icon.as_deref(), Some("/cache/1091500/icon.jpg"));
        assert_eq!(items[0].actions[0].title, "Open");
    }

    #[test]
    fn the_service_reads_again_when_the_library_changes() {
        let source = FakeSource::with(&[(1, "Brawlhalla")]);
        let list = GameList::new(source.clone());
        let opener = Arc::new(FakeOpener::default());
        let service = &GamesExtension::new(list.clone(), opener).services()[0];
        service.start().unwrap();
        let read = |count: usize| {
            let deadline = Instant::now() + Duration::from_secs(5);
            while list.snapshot().len() != count && Instant::now() < deadline {
                std::thread::sleep(Duration::from_millis(20));
            }
            list.snapshot().len() == count
        };
        assert!(read(1));
        source.set(&[(1, "Brawlhalla"), (2, "Cyberpunk 2077")]);
        assert!(read(2));
        service.stop();
    }

    #[tokio::test]
    async fn a_game_without_an_icon_shows_a_gamepad() {
        let source = FakeSource::with(&[(1, "Brawlhalla")]);
        source.strip_art();
        let ext = extension(source);
        assert_eq!(
            root_items(&ext).await[0].icon.as_deref(),
            Some("icon:gamepad-2")
        );
    }

    #[test]
    fn opening_a_game_asks_steam_to_run_it() {
        let (ext, opener) = launcher(FakeSource::with(&[(1091500, "Cyberpunk 2077")]));
        assert_eq!(launch(&ext, "1091500"), ActionOutcome::Done);
        assert_eq!(
            *opener.opened.lock().unwrap(),
            ["steam://rungameid/1091500"]
        );
    }

    #[test]
    fn a_game_installed_since_the_last_read_still_opens() {
        let source = FakeSource::with(&[(1, "Brawlhalla")]);
        let (ext, _) = launcher(source.clone());
        source.set(&[(1, "Brawlhalla"), (2, "Cyberpunk 2077")]);
        assert_eq!(launch(&ext, "2"), ActionOutcome::Done);
    }

    #[test]
    fn an_uninstalled_game_is_read_again_and_reported_gone() {
        let source = FakeSource::with(&[(1, "Brawlhalla"), (2, "Cyberpunk 2077")]);
        let (ext, opener) = launcher(source.clone());
        source.set(&[(1, "Brawlhalla")]);
        assert_eq!(
            launch(&ext, "2"),
            ActionOutcome::Failed("That game is no longer installed".into())
        );
        assert!(ext.list.get("2").is_none());
        assert!(opener.opened.lock().unwrap().is_empty());
    }

    #[test]
    fn steam_failing_to_open_is_reported() {
        let (ext, opener) = launcher(FakeSource::with(&[(1, "Brawlhalla")]));
        opener.fails.store(true, Ordering::SeqCst);
        assert_eq!(
            launch(&ext, "1"),
            ActionOutcome::Failed("Couldn't start the game through Steam".into())
        );
    }

    #[test]
    fn every_message_is_a_sentence() {
        for error in [LaunchError::Gone, LaunchError::Steam] {
            let text = error.to_string();
            assert!(text.starts_with(char::is_uppercase), "{text}");
            assert!(!text.ends_with('.') && !text.contains(". "), "{text}");
        }
    }

    #[test]
    fn search_games_shows_a_portrait_grid_of_covers() {
        let source = FakeSource::with(&[(1, "Brawlhalla")]);
        let View::Grid(grid) = grid(&source.games()).view else {
            panic!("not a grid");
        };
        assert_eq!((grid.columns, grid.aspect), (5, 2.0 / 3.0));
        assert_eq!(grid.items[0].title, "Brawlhalla");
        assert_eq!(
            grid.items[0].content,
            TileContent::Image {
                image: "/cache/1/library_600x900.jpg".into()
            }
        );
        assert_eq!(grid.items[0].actions[0].title, "Open");
    }

    #[test]
    fn a_game_without_a_cover_shows_a_gamepad_tile() {
        let source = FakeSource::with(&[(1, "Brawlhalla")]);
        source.strip_art();
        let View::Grid(grid) = grid(&source.games()).view else {
            panic!("not a grid");
        };
        assert_eq!(
            grid.items[0].content,
            TileContent::Image {
                image: "icon:gamepad-2".into()
            }
        );
    }

    #[test]
    fn no_games_shows_how_they_get_there() {
        let View::Grid(grid) = grid(&[]).view else {
            panic!("not a grid");
        };
        let empty = grid.empty_state.unwrap();
        assert_eq!(empty.title, "No Steam games installed");
        assert_eq!(
            empty.description.as_deref(),
            Some("Games you install with Steam will show up here")
        );
    }
}
