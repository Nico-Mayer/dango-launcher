//! Reading the games Steam has installed from the files Steam keeps on disk,
//! which are the same on Windows and macOS.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};

use steamlocate::app::StateFlag;
use steamlocate::{App, SteamDir};

/// Installed beside games, but not games. The real app type is only in the
/// binary `appinfo.vdf`, whose format changes with Steam updates.
const NOT_GAMES: &[u32] = &[
    228980, // Steamworks Common Redistributables
];

const COVERS: [&str; 2] = ["library_600x900.jpg", "library_capsule.jpg"];

#[derive(Clone, Debug, PartialEq)]
pub struct Game {
    pub app_id: u32,
    pub name: String,
    pub icon: Option<String>,
    pub cover: Option<String>,
}

pub trait GameSource: Send + Sync {
    fn games(&self) -> Vec<Game>;

    /// A signature of the library folders, cheap enough to poll. `None` when
    /// there is nothing to watch, which leaves freshness to the sweep.
    fn fingerprint(&self) -> Option<u64>;
}

/// Steam found where it installs itself, looked up on every read so a Steam
/// installed while Dango runs is picked up.
#[derive(Default)]
pub struct SteamSource {
    root: Option<PathBuf>,
}

impl SteamSource {
    #[cfg(test)]
    pub fn at(root: PathBuf) -> Self {
        Self { root: Some(root) }
    }

    fn steam(&self) -> Option<SteamDir> {
        match &self.root {
            Some(root) => SteamDir::from_dir(root).ok(),
            None => steamlocate::locate().ok(),
        }
    }
}

impl GameSource for SteamSource {
    fn games(&self) -> Vec<Game> {
        let Some(steam) = self.steam() else {
            return Vec::new();
        };
        let Ok(libraries) = steam.libraries() else {
            return Vec::new();
        };
        let art = steam.path().join("appcache").join("librarycache");
        let mut games: Vec<Game> = libraries
            .filter_map(Result::ok)
            .flat_map(|library| library.apps().filter_map(Result::ok).collect::<Vec<_>>())
            .filter(|app| !NOT_GAMES.contains(&app.app_id) && fully_installed(app))
            .filter_map(|app| {
                let art = art.join(app.app_id.to_string());
                Some(Game {
                    app_id: app.app_id,
                    name: app.name?,
                    icon: icon(&art),
                    cover: cover(&art),
                })
            })
            .collect();
        games.sort_by_key(|game| game.name.to_lowercase());
        games
    }

    /// Steam adds or removes an `appmanifest_*.acf` in a library's `steamapps`
    /// folder on install and uninstall, which changes that folder's mtime.
    fn fingerprint(&self) -> Option<u64> {
        let steam = self.steam()?;
        let folders = steam.path().join("steamapps").join("libraryfolders.vdf");
        let libraries = steam.library_paths().unwrap_or_default();
        let watched = std::iter::once(folders).chain(libraries.iter().map(|l| l.join("steamapps")));
        let mut hasher = DefaultHasher::new();
        for path in watched {
            if let Ok(modified) = std::fs::metadata(&path).and_then(|m| m.modified()) {
                path.hash(&mut hasher);
                modified.hash(&mut hasher);
            }
        }
        Some(hasher.finish())
    }
}

fn fully_installed(app: &App) -> bool {
    app.state_flags
        .is_some_and(|state| state.flags().any(|flag| flag == StateFlag::FullyInstalled))
}

/// Steam names the icon after its SHA-1, beside the named library images.
fn icon(art: &Path) -> Option<String> {
    std::fs::read_dir(art)
        .ok()?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .find(|path| {
            path.extension().is_some_and(|ext| ext == "jpg")
                && path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .is_some_and(is_sha1)
        })
        .map(|path| path.to_string_lossy().into_owned())
}

/// Older clients keep the cover at the top level, newer ones in a hash-named
/// subfolder.
fn cover(art: &Path) -> Option<String> {
    let subfolders = std::fs::read_dir(art)
        .into_iter()
        .flatten()
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.is_dir());
    std::iter::once(art.to_path_buf())
        .chain(subfolders)
        .flat_map(|dir| COVERS.map(|name| dir.join(name)))
        .find(|path| path.is_file())
        .map(|path| path.to_string_lossy().into_owned())
}

fn is_sha1(stem: &str) -> bool {
    stem.len() == 40 && stem.bytes().all(|b| b.is_ascii_hexdigit())
}

#[cfg(test)]
mod tests {
    use super::*;

    const ICON: &str = "6897c3848f3e0350d512f59d5bae174a1e3739f9.jpg";
    const SUBFOLDER: &str = "6399de67cce3cecadce7900e03f6e09ff910a235";

    struct Fixture {
        steam: PathBuf,
        second: PathBuf,
    }

    impl Fixture {
        fn new() -> Self {
            let root = std::env::temp_dir().join(format!("dango-steam-{}", uuid::Uuid::new_v4()));
            let steam = root.join("Steam");
            let second = root.join("SteamLibrary");
            std::fs::create_dir_all(steam.join("steamapps")).unwrap();
            std::fs::create_dir_all(second.join("steamapps")).unwrap();
            let entry = |n: usize, path: &Path| {
                let path = path.to_string_lossy().replace('\\', "\\\\");
                format!("\t\"{n}\"\n\t{{\n\t\t\"path\"\t\t\"{path}\"\n\t}}\n")
            };
            std::fs::write(
                steam.join("steamapps").join("libraryfolders.vdf"),
                format!(
                    "\"libraryfolders\"\n{{\n{}{}}}\n",
                    entry(0, &steam),
                    entry(1, &second)
                ),
            )
            .unwrap();
            Self { steam, second }
        }

        fn install(&self, library: &Path, app_id: u32, name: &str, state: u32) {
            std::fs::write(
                library
                    .join("steamapps")
                    .join(format!("appmanifest_{app_id}.acf")),
                format!(
                    "\"AppState\"\n{{\n\t\"appid\"\t\t\"{app_id}\"\n\t\"installdir\"\t\t\"{name}\"\n\t\"name\"\t\t\"{name}\"\n\t\"StateFlags\"\t\t\"{state}\"\n}}\n"
                ),
            )
            .unwrap();
        }

        fn art(&self, app_id: u32, files: &[&str]) {
            let dir = self
                .steam
                .join("appcache")
                .join("librarycache")
                .join(app_id.to_string());
            for file in files {
                let path = dir.join(file);
                std::fs::create_dir_all(path.parent().unwrap()).unwrap();
                std::fs::write(path, b"jpg").unwrap();
            }
        }

        fn source(&self) -> SteamSource {
            SteamSource::at(self.steam.clone())
        }
    }

    fn names(games: &[Game]) -> Vec<&str> {
        games.iter().map(|g| g.name.as_str()).collect()
    }

    #[test]
    fn games_from_every_library_are_listed() {
        let steam = Fixture::new();
        steam.install(&steam.steam, 1, "Brawlhalla", 4);
        steam.install(&steam.second, 2, "Cyberpunk 2077", 4);
        assert_eq!(
            names(&steam.source().games()),
            ["Brawlhalla", "Cyberpunk 2077"]
        );
    }

    #[test]
    fn redistributables_are_not_games() {
        let steam = Fixture::new();
        steam.install(
            &steam.steam,
            228980,
            "Steamworks Common Redistributables",
            4,
        );
        steam.install(&steam.steam, 1, "Brawlhalla", 4);
        assert_eq!(names(&steam.source().games()), ["Brawlhalla"]);
    }

    #[test]
    fn a_game_still_downloading_is_left_out() {
        let steam = Fixture::new();
        steam.install(&steam.steam, 1, "Brawlhalla", 1026);
        steam.install(&steam.steam, 2, "Cyberpunk 2077", 6);
        assert_eq!(names(&steam.source().games()), ["Cyberpunk 2077"]);
    }

    #[test]
    fn icon_and_cover_come_from_the_library_cache() {
        let steam = Fixture::new();
        steam.install(&steam.steam, 1, "Brawlhalla", 4);
        steam.art(1, &[ICON, "library_600x900.jpg", "logo.png"]);
        let game = &steam.source().games()[0];
        assert!(game.icon.as_deref().unwrap().ends_with(ICON));
        assert!(game
            .cover
            .as_deref()
            .unwrap()
            .ends_with("library_600x900.jpg"));
    }

    #[test]
    fn a_cover_in_a_subfolder_is_found() {
        let steam = Fixture::new();
        steam.install(&steam.steam, 1, "Cyberpunk 2077", 4);
        steam.art(1, &[&format!("{SUBFOLDER}/library_capsule.jpg")]);
        let game = &steam.source().games()[0];
        assert!(game
            .cover
            .as_deref()
            .unwrap()
            .ends_with("library_capsule.jpg"));
    }

    #[test]
    fn a_game_without_cached_art_has_neither() {
        let steam = Fixture::new();
        steam.install(&steam.steam, 1, "Brawlhalla", 4);
        steam.art(1, &["header.jpg"]);
        let game = &steam.source().games()[0];
        assert_eq!((&game.icon, &game.cover), (&None, &None));
    }

    #[test]
    fn no_steam_means_no_games_and_nothing_to_watch() {
        let source = SteamSource::at(std::env::temp_dir().join("dango-no-steam-7f3a"));
        assert!(source.games().is_empty());
        assert!(source.fingerprint().is_none());
    }

    #[test]
    fn the_fingerprint_changes_when_a_game_is_installed() {
        let steam = Fixture::new();
        let source = steam.source();
        let before = source.fingerprint();
        assert_eq!(before, source.fingerprint());
        std::thread::sleep(std::time::Duration::from_millis(20));
        steam.install(&steam.second, 2, "Cyberpunk 2077", 4);
        assert_ne!(before, source.fingerprint());
    }
}
