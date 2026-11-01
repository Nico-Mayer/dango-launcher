//! The `games` built-in extension: installed Steam games in root search and in
//! a grid of cover art, launched through Steam.

pub mod steam;

pub use steam::{Game, GameSource, SteamSource};

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

use async_trait::async_trait;

use crate::extension::{ActivationResult, Extension, Manifest, Service, NAMED_ICON};
use crate::protocol::Action;
use crate::search::{Candidate, RootProvider, Source};

pub const EXTENSION_ID: &str = "dango.games";

pub const ACTION_LAUNCH: &str = "launch";

const FALLBACK_ICON: &str = "gamepad-2";

pub(crate) fn manifest() -> Manifest {
    Manifest {
        manifest_version: 1,
        id: EXTENSION_ID.into(),
        name: "Games".into(),
        icon: Some(format!("{NAMED_ICON}{FALLBACK_ICON}")),
        commands: vec![],
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
    provider: Arc<GameProvider>,
}

impl GamesExtension {
    pub fn new(list: Arc<GameList>) -> Self {
        Self {
            manifest: manifest(),
            provider: Arc::new(GameProvider(list.clone())),
            list,
        }
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

/// The longest the list may go without a read, for a change the fingerprint
/// misses.
const REFRESH_INTERVAL: Duration = Duration::from_secs(180);

const TICK: Duration = Duration::from_millis(200);

/// Steam writes the manifest before it finishes the files around it.
const SETTLE: Duration = Duration::from_secs(1);

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
                wait_for_refresh(&list, &running);
            }
        });
        Ok(())
    }

    fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);
    }
}

fn wait_for_refresh(list: &GameList, running: &AtomicBool) {
    let baseline = list.fingerprint();
    let started = Instant::now();
    while running.load(Ordering::SeqCst) && started.elapsed() < REFRESH_INTERVAL {
        std::thread::sleep(TICK);
        if baseline.is_some() && list.fingerprint() != baseline {
            std::thread::sleep(SETTLE);
            return;
        }
    }
}

#[cfg(test)]
pub mod fake {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

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
    use super::fake::FakeSource;
    use super::*;

    fn extension(source: Arc<FakeSource>) -> GamesExtension {
        let list = GameList::new(source);
        list.reload();
        GamesExtension::new(list)
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
        let service = &GamesExtension::new(list.clone()).services()[0];
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
}
