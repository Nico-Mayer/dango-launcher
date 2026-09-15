//! Favicons for quicklinks: the on-disk cache, the fetch, and the service that
//! keeps the two in step.
//!
//! One icon per host, so several quicklinks into the same site share a file and
//! a request. The bytes are written exactly as they arrived, because the webview
//! decodes PNG, ICO, SVG, JPEG, WebP and GIF on its own and a decode step here
//! would only add a dependency and lose the formats it cannot read.
//!
//! The search path may not touch the network or the disk, so the cache holds an
//! in-memory map from host to file and the provider does one lookup in it.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};
use std::time::{Duration, SystemTime};

use reqwest::blocking::Client;
use tauri::Url;

use crate::extension::{ActivationResult, Service};
use crate::templates::{Template, Values};

use super::store::{Record, Records};

/// How long a cached icon is used before it is fetched again.
const ICON_MAX_AGE: Duration = Duration::from_secs(14 * 24 * 60 * 60);

/// How long a host that had no usable icon is left alone before it is tried
/// again.
const MISS_MAX_AGE: Duration = Duration::from_secs(24 * 60 * 60);

/// The largest body accepted as an icon. Well past any real favicon, and low
/// enough that a misconfigured host cannot write something large into the cache.
const MAX_BYTES: u64 = 256 * 1024;

/// Marks a host that answered without a usable icon.
const MISS_EXTENSION: &str = "miss";

/// The formats both webviews draw, and so the only ones worth keeping.
const DRAWABLE: [&str; 6] = ["png", "ico", "svg", "jpg", "webp", "gif"];

const TIMEOUT: Duration = Duration::from_secs(5);

/// How often the sweep wakes to check whether it should stop or run again.
const TICK: Duration = Duration::from_millis(200);

/// The longest the cache goes without a sweep, for icons that went stale while
/// the quicklinks themselves did not change.
const REFRESH_INTERVAL: Duration = Duration::from_secs(6 * 60 * 60);

/// Whether the user wants favicons at all, read fresh rather than at startup so
/// turning the preference off takes effect without a restart.
pub type FaviconsEnabled = Arc<dyn Fn() -> bool + Send + Sync>;

#[derive(Debug, thiserror::Error)]
pub enum FaviconError {
    #[error("{0}")]
    Request(#[from] reqwest::Error),
    #[error("{0}")]
    Discovery(#[from] favicon_picker::error::Error),
    #[error("answered {0}")]
    Status(u16),
    #[error("served {0}, which is not an image the launcher can draw")]
    UnusableType(String),
    #[error("served an icon larger than {MAX_BYTES} bytes")]
    TooLarge,
    #[error("declares no icon")]
    NoIcon,
}

/// A fetched icon, ready to be written as it arrived.
pub struct Image {
    pub bytes: Vec<u8>,
    pub extension: &'static str,
}

/// Where an icon comes from. A trait so the sweep can be tested without a
/// network.
pub trait FaviconSource: Send + Sync {
    fn fetch(&self, site: &Url) -> Result<Image, FaviconError>;
}

#[derive(Debug, PartialEq, Eq)]
enum State {
    /// Nothing known about the host.
    Unknown,
    /// An icon or a miss young enough to be trusted.
    Fresh,
    /// An icon or a miss old enough to be fetched again.
    Stale,
}

pub struct FaviconCache {
    dir: PathBuf,
    /// File key to icon file, so the search path resolves an icon without a
    /// syscall. Holds only hosts with an icon; a miss lives on disk only.
    /// Keyed by the file name rather than the host so that rebuilding the index
    /// from the directory cannot disagree with a lookup.
    index: RwLock<HashMap<String, PathBuf>>,
}

impl FaviconCache {
    /// Opens the cache directory and indexes it with one read, so the first
    /// keystroke after a start already has every icon from the last run.
    pub fn new(dir: PathBuf) -> Arc<Self> {
        let _ = std::fs::create_dir_all(&dir);
        let cache = Self {
            index: RwLock::new(index_of(&dir)),
            dir,
        };
        Arc::new(cache)
    }

    /// The cached icon for a host, or `None` for one with no icon yet.
    pub fn path(&self, host: &str) -> Option<String> {
        self.index
            .read()
            .unwrap()
            .get(&key(host))
            .map(|path| path.to_string_lossy().into_owned())
    }

    fn needs_fetch(&self, host: &str) -> bool {
        match self.state(host) {
            State::Fresh => false,
            State::Unknown | State::Stale => true,
        }
    }

    fn state(&self, host: &str) -> State {
        if let Some(path) = self.index.read().unwrap().get(&key(host)) {
            return match age(path) {
                Some(age) if age < ICON_MAX_AGE => State::Fresh,
                _ => State::Stale,
            };
        }
        match age(&self.miss_file(host)) {
            Some(age) if age < MISS_MAX_AGE => State::Fresh,
            Some(_) => State::Stale,
            None => State::Unknown,
        }
    }

    /// Writes a fetched icon and makes it visible to the search path. Replacing
    /// an icon whose type changed removes the old file, so one host cannot leave
    /// two behind.
    fn store(&self, host: &str, image: &Image) -> std::io::Result<()> {
        let path = self.dir.join(format!("{}.{}", key(host), image.extension));
        std::fs::write(&path, &image.bytes)?;
        let _ = std::fs::remove_file(self.miss_file(host));
        let previous = self.index.write().unwrap().insert(key(host), path.clone());
        if let Some(previous) = previous {
            if previous != path {
                let _ = std::fs::remove_file(previous);
            }
        }
        Ok(())
    }

    /// Records that the host has no icon worth keeping, so it is not asked again
    /// until the retry delay has passed.
    fn store_miss(&self, host: &str) {
        let _ = std::fs::write(self.miss_file(host), []);
        if let Some(path) = self.index.write().unwrap().remove(&key(host)) {
            let _ = std::fs::remove_file(path);
        }
    }

    fn miss_file(&self, host: &str) -> PathBuf {
        self.dir.join(format!("{}.{MISS_EXTENSION}", key(host)))
    }

    /// Puts an icon in the cache for a host, for tests in the sibling modules
    /// that need one without a fetch.
    #[cfg(test)]
    pub(super) fn seed(&self, host: &str) {
        self.store(
            host,
            &Image {
                bytes: vec![1],
                extension: "png",
            },
        )
        .unwrap();
    }

    #[cfg(test)]
    pub fn in_temp() -> Arc<Self> {
        let dir = std::env::temp_dir().join(format!("dango-favicons-{}", uuid::Uuid::new_v4()));
        Self::new(dir)
    }
}

fn index_of(dir: &Path) -> HashMap<String, PathBuf> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return HashMap::new();
    };
    entries
        .flatten()
        .filter_map(|entry| {
            let path = entry.path();
            let extension = path.extension()?.to_str()?;
            if !DRAWABLE.contains(&extension) {
                return None;
            }
            let key = path.file_stem()?.to_str()?.to_string();
            Some((key, path))
        })
        .collect()
}

fn age(path: &Path) -> Option<Duration> {
    let modified = std::fs::metadata(path).ok()?.modified().ok()?;
    // A clock that moved backwards leaves the file in the future; treat it as
    // brand new rather than refetching everything.
    Some(
        SystemTime::now()
            .duration_since(modified)
            .unwrap_or_default(),
    )
}

/// A host as a file name. Dots and hyphens are kept, so a normal host reads as
/// itself in the cache directory and two hosts cannot collide by losing them.
fn key(host: &str) -> String {
    host.chars()
        .map(|c| match c {
            c if c.is_alphanumeric() => c,
            '.' | '-' => c,
            _ => '_',
        })
        .collect()
}

/// The file extension for a content type, or `None` for anything the webview
/// cannot draw. The parameters after a `;` and the case are ignored, because
/// both vary by server.
fn extension_for(content_type: &str) -> Option<&'static str> {
    let kind = content_type.split(';').next()?.trim().to_ascii_lowercase();
    match kind.as_str() {
        "image/png" => Some(DRAWABLE[0]),
        "image/x-icon" | "image/vnd.microsoft.icon" | "image/ico" | "image/icon" => {
            Some(DRAWABLE[1])
        }
        "image/svg+xml" | "image/svg" => Some(DRAWABLE[2]),
        "image/jpeg" | "image/jpg" => Some(DRAWABLE[3]),
        "image/webp" => Some(DRAWABLE[4]),
        "image/gif" => Some(DRAWABLE[5]),
        _ => None,
    }
}

/// The site a record points at: its host, for the cache, and the origin, for the
/// fetch. `None` for a template that does not resolve to an http URL, such as
/// one opening a `mailto:`.
pub fn site_of(record: &Record) -> Option<(String, Url)> {
    let template = Template::parse(&record.body).ok()?;
    let rendered = template.render_url(&Values::default()).ok()?;
    let url = Url::parse(&rendered.text).ok()?;
    if !matches!(url.scheme(), "http" | "https") {
        return None;
    }
    let host = url.host_str()?.to_string();
    Some((host, url.join("/").ok()?))
}

/// The real source: asks the site which icon it declares, then fetches it.
pub struct HttpFavicons {
    client: Client,
}

impl HttpFavicons {
    pub fn new() -> Option<Arc<Self>> {
        let client = Client::builder()
            .timeout(TIMEOUT)
            .user_agent(concat!("dango/", env!("CARGO_PKG_VERSION")))
            .build()
            .ok()?;
        Some(Arc::new(Self { client }))
    }
}

impl FaviconSource for HttpFavicons {
    fn fetch(&self, site: &Url) -> Result<Image, FaviconError> {
        // The crate reads the page's `<link rel="icon">` tags and falls back to
        // /favicon.ico when it declares none. A site that refuses the page
        // itself, as some do to anything that is not a browser, still usually
        // serves that default, so its own failure is not the end of the attempt.
        let candidates = favicon_picker::get_blocking_favicons_from_url(&self.client, site)
            .or_else(|_| favicon_picker::get_default_favicon(site).map(|icon| vec![icon]))?;
        let mut last = None;
        for candidate in &candidates {
            match self.download(&candidate.href) {
                Ok(image) => return Ok(image),
                Err(error) => last = Some(error),
            }
        }
        Err(last.unwrap_or(FaviconError::NoIcon))
    }
}

impl HttpFavicons {
    fn download(&self, url: &Url) -> Result<Image, FaviconError> {
        let url = url.clone();
        let response = self.client.get(url).send()?;
        if !response.status().is_success() {
            return Err(FaviconError::Status(response.status().as_u16()));
        }
        let content_type = response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .unwrap_or_default()
            .to_string();
        let extension =
            extension_for(&content_type).ok_or(FaviconError::UnusableType(content_type))?;
        if response.content_length().is_some_and(|len| len > MAX_BYTES) {
            return Err(FaviconError::TooLarge);
        }
        let bytes = response.bytes()?;
        if bytes.len() as u64 > MAX_BYTES {
            return Err(FaviconError::TooLarge);
        }
        Ok(Image {
            bytes: bytes.to_vec(),
            extension,
        })
    }
}

/// Fetches what the quicklinks point at, off the search path. Sweeps on start,
/// whenever the set of hosts changes, and every few hours for icons that went
/// stale on their own.
pub struct FaviconService {
    records: Arc<Records>,
    cache: Arc<FaviconCache>,
    source: Arc<dyn FaviconSource>,
    enabled: FaviconsEnabled,
    running: Arc<AtomicBool>,
}

impl FaviconService {
    pub fn new(
        records: Arc<Records>,
        cache: Arc<FaviconCache>,
        source: Arc<dyn FaviconSource>,
        enabled: FaviconsEnabled,
    ) -> Self {
        Self {
            records,
            cache,
            source,
            enabled,
            running: Arc::new(AtomicBool::new(false)),
        }
    }

    fn sweep(&self) {
        let Ok(records) = self.records.all() else {
            return;
        };
        let mut seen = Vec::new();
        for record in &records {
            // Both are re-read per record rather than once, so stopping the
            // extension or turning the preference off ends a sweep already
            // under way instead of letting it work through what is left.
            if !self.running.load(Ordering::SeqCst) || !(self.enabled)() {
                return;
            }
            let Some((host, site)) = site_of(record) else {
                continue;
            };
            if seen.contains(&host) || !self.cache.needs_fetch(&host) {
                continue;
            }
            seen.push(host.clone());
            match self.source.fetch(&site) {
                Ok(image) => {
                    if let Err(error) = self.cache.store(&host, &image) {
                        crate::log::append(&format!(
                            "favicon for {host} could not be saved: {error}"
                        ));
                    }
                }
                Err(error) => {
                    self.cache.store_miss(&host);
                    crate::log::append(&format!("no favicon for {host}: {error}"));
                }
            }
        }
    }

    /// When the records last changed, so a sweep can follow an edit rather than
    /// a timer alone. Taken from the file rather than from the records
    /// themselves, because every change reaches the file and rendering 500
    /// templates several times a second to notice one would not be free.
    fn fingerprint(&self) -> Option<SystemTime> {
        std::fs::metadata(self.records.path()).ok()?.modified().ok()
    }

    fn wait_for_refresh(&self) {
        let baseline = self.fingerprint();
        // Turning the preference back on has to resume fetching, and it lives in
        // the config file rather than in the records, so the wait watches it too
        // instead of sleeping until the next edit or the next interval.
        let was_wanted = (self.enabled)();
        let started = std::time::Instant::now();
        while self.running.load(Ordering::SeqCst) && started.elapsed() < REFRESH_INTERVAL {
            std::thread::sleep(TICK);
            if self.fingerprint() != baseline || (!was_wanted && (self.enabled)()) {
                return;
            }
        }
    }
}

impl Service for FaviconService {
    fn start(&self) -> ActivationResult {
        self.running.store(true, Ordering::SeqCst);
        let records = self.records.clone();
        let cache = self.cache.clone();
        let source = self.source.clone();
        let enabled = self.enabled.clone();
        let running = self.running.clone();
        std::thread::spawn(move || {
            let service = FaviconService {
                records,
                cache,
                source,
                enabled,
                running: running.clone(),
            };
            while running.load(Ordering::SeqCst) {
                service.sweep();
                service.wait_for_refresh();
            }
        });
        Ok(())
    }

    fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    use crate::extensions::snippets::store::Kind;

    fn png() -> Image {
        Image {
            bytes: vec![1, 2, 3],
            extension: "png",
        }
    }

    fn record(body: &str) -> Record {
        Record {
            id: "1".into(),
            name: "Example".into(),
            body: body.into(),
            keyword: None,
        }
    }

    #[test]
    fn an_unknown_host_has_no_path() {
        let cache = FaviconCache::in_temp();
        assert!(cache.path("example.com").is_none());
    }

    #[test]
    fn a_written_icon_is_found_by_path() {
        let cache = FaviconCache::in_temp();
        cache.store("example.com", &png()).unwrap();
        assert!(cache
            .path("example.com")
            .unwrap()
            .ends_with("example.com.png"));
    }

    #[test]
    fn a_fresh_icon_is_not_fetched_again() {
        let cache = FaviconCache::in_temp();
        cache.store("example.com", &png()).unwrap();
        assert!(!cache.needs_fetch("example.com"));
    }

    #[test]
    fn a_miss_keeps_the_path_empty_and_stops_the_retry() {
        let cache = FaviconCache::in_temp();
        cache.store_miss("example.com");
        assert!(cache.path("example.com").is_none());
        assert!(!cache.needs_fetch("example.com"));
    }

    #[test]
    fn an_aged_icon_and_an_aged_miss_are_fetched_again() {
        let cache = FaviconCache::in_temp();
        cache.store("icon.example", &png()).unwrap();
        cache.store_miss("miss.example");
        age_file(
            &cache.dir.join("icon.example.png"),
            ICON_MAX_AGE + Duration::from_secs(60),
        );
        age_file(
            &cache.miss_file("miss.example"),
            MISS_MAX_AGE + Duration::from_secs(60),
        );
        assert!(cache.needs_fetch("icon.example"));
        assert!(cache.needs_fetch("miss.example"));
    }

    #[test]
    fn an_icon_survives_a_restart_without_a_refetch() {
        let cache = FaviconCache::in_temp();
        cache.store("example.com", &png()).unwrap();
        let reopened = FaviconCache::new(cache.dir.clone());
        assert!(reopened.path("example.com").is_some());
        assert!(!reopened.needs_fetch("example.com"));
    }

    #[test]
    fn a_miss_survives_a_restart() {
        let cache = FaviconCache::in_temp();
        cache.store_miss("example.com");
        let reopened = FaviconCache::new(cache.dir.clone());
        assert!(reopened.path("example.com").is_none());
        assert!(!reopened.needs_fetch("example.com"));
    }

    #[test]
    fn a_new_type_replaces_the_old_file() {
        let cache = FaviconCache::in_temp();
        cache.store("example.com", &png()).unwrap();
        cache
            .store(
                "example.com",
                &Image {
                    bytes: vec![4],
                    extension: "svg",
                },
            )
            .unwrap();
        assert!(!cache.dir.join("example.com.png").exists());
        assert!(cache
            .path("example.com")
            .unwrap()
            .ends_with("example.com.svg"));
    }

    #[test]
    fn every_type_the_webview_draws_maps_to_an_extension() {
        for (content_type, extension) in [
            ("image/png", "png"),
            ("image/x-icon", "ico"),
            ("image/vnd.microsoft.icon", "ico"),
            ("image/svg+xml", "svg"),
            ("image/jpeg", "jpg"),
            ("image/webp", "webp"),
            ("image/gif", "gif"),
            ("IMAGE/PNG; charset=binary", "png"),
        ] {
            assert_eq!(
                extension_for(content_type),
                Some(extension),
                "{content_type}"
            );
        }
    }

    #[test]
    fn a_type_that_is_not_a_drawable_image_is_refused() {
        for content_type in ["text/html", "application/json", "image/heic", ""] {
            assert_eq!(extension_for(content_type), None, "{content_type}");
        }
    }

    #[test]
    fn a_host_comes_from_the_template_rendered_empty() {
        assert_eq!(
            site_of(&record("https://github.com/search?q={{query}}")),
            Some((
                "github.com".to_string(),
                Url::parse("https://github.com/").unwrap()
            ))
        );
        assert_eq!(
            site_of(&record("https://example.com/wiki/{{query}}")),
            Some((
                "example.com".to_string(),
                Url::parse("https://example.com/").unwrap()
            ))
        );
        assert_eq!(
            site_of(&record("https://example.com")),
            Some((
                "example.com".to_string(),
                Url::parse("https://example.com/").unwrap()
            ))
        );
    }

    #[test]
    fn a_url_that_is_not_http_has_no_site() {
        assert!(site_of(&record("mailto:{{query}}@example.com")).is_none());
        assert!(site_of(&record("file:///tmp/notes.txt")).is_none());
    }

    type Hook = Box<dyn Fn() + Send + Sync>;

    #[derive(Default)]
    struct FakeSource {
        asked: Mutex<Vec<String>>,
        answer: Option<&'static str>,
        /// Runs after the first fetch, so a test can change the world mid-sweep.
        first: Mutex<Option<Hook>>,
    }

    impl FakeSource {
        fn serving(extension: &'static str) -> Arc<Self> {
            Arc::new(Self {
                answer: Some(extension),
                ..Default::default()
            })
        }

        fn failing() -> Arc<Self> {
            Arc::new(Self::default())
        }

        fn asked(&self) -> Vec<String> {
            self.asked.lock().unwrap().clone()
        }

        fn on_first_fetch(&self, hook: impl Fn() + Send + Sync + 'static) {
            *self.first.lock().unwrap() = Some(Box::new(hook));
        }
    }

    impl FaviconSource for FakeSource {
        fn fetch(&self, site: &Url) -> Result<Image, FaviconError> {
            self.asked.lock().unwrap().push(site.to_string());
            if let Some(hook) = self.first.lock().unwrap().take() {
                hook();
            }
            match self.answer {
                Some(extension) => Ok(Image {
                    bytes: vec![7],
                    extension,
                }),
                None => Err(FaviconError::NoIcon),
            }
        }
    }

    fn service(source: Arc<dyn FaviconSource>, enabled: bool) -> (FaviconService, Arc<Records>) {
        let (service, records, _) = switchable_service(source, enabled);
        (service, records)
    }

    /// The service plus the switch behind its preference, for the tests that
    /// turn favicons off and on while it runs.
    fn switchable_service(
        source: Arc<dyn FaviconSource>,
        enabled: bool,
    ) -> (FaviconService, Arc<Records>, Arc<AtomicBool>) {
        let dir = std::env::temp_dir().join(format!("dango-quicklinks-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let (records, error) = Records::open(&dir, Kind::Quicklink);
        assert!(error.is_none());
        let switch = Arc::new(AtomicBool::new(enabled));
        let wanted = switch.clone();
        let service = FaviconService::new(
            records.clone(),
            FaviconCache::in_temp(),
            source,
            Arc::new(move || wanted.load(Ordering::SeqCst)),
        );
        // The sweep stops when the service is not running, which is what a
        // started service would have set.
        service.running.store(true, Ordering::SeqCst);
        (service, records, switch)
    }

    #[test]
    fn a_sweep_fetches_each_host_once_and_skips_a_fresh_one() {
        let source = FakeSource::serving("png");
        let (service, records) = service(source.clone(), true);
        records
            .create("Docs", "https://example.com/docs", None)
            .unwrap();
        records
            .create("Search", "https://example.com/search?q={{query}}", None)
            .unwrap();
        records
            .create("Other", "https://other.example/", None)
            .unwrap();

        service.sweep();
        assert_eq!(
            source.asked(),
            ["https://example.com/", "https://other.example/"],
            "one request per host, not per quicklink"
        );

        service.sweep();
        assert_eq!(source.asked().len(), 2, "a fresh icon is not fetched again");
        assert!(service.cache.path("example.com").is_some());
    }

    #[test]
    fn a_stale_icon_is_fetched_again() {
        let source = FakeSource::serving("png");
        let (service, records) = service(source.clone(), true);
        records
            .create("Docs", "https://example.com/", None)
            .unwrap();
        service.sweep();
        age_file(
            &service.cache.dir.join("example.com.png"),
            ICON_MAX_AGE + Duration::from_secs(60),
        );
        service.sweep();
        assert_eq!(source.asked().len(), 2);
    }

    #[test]
    fn a_host_that_has_no_icon_keeps_the_quicklink_usable() {
        let source = FakeSource::failing();
        let (service, records) = service(source.clone(), true);
        records
            .create("Docs", "https://example.com/", None)
            .unwrap();
        service.sweep();
        assert!(service.cache.path("example.com").is_none());
        service.sweep();
        assert_eq!(source.asked().len(), 1, "a miss is not retried at once");
    }

    #[test]
    fn a_sweep_with_the_preference_off_asks_for_nothing() {
        let source = FakeSource::serving("png");
        let (service, records) = service(source.clone(), false);
        records
            .create("Docs", "https://example.com/", None)
            .unwrap();
        service.sweep();
        assert!(source.asked().is_empty());
    }

    #[test]
    fn a_host_no_longer_used_is_not_fetched_again() {
        let source = FakeSource::serving("png");
        let (service, records) = service(source.clone(), true);
        let id = records
            .create("Docs", "https://example.com/", None)
            .unwrap();
        service.sweep();
        records.remove(&id).unwrap();
        age_file(
            &service.cache.dir.join("example.com.png"),
            ICON_MAX_AGE + Duration::from_secs(60),
        );
        service.sweep();
        assert_eq!(source.asked().len(), 1);
    }

    #[test]
    fn a_new_quicklink_is_picked_up_without_a_restart() {
        let source = FakeSource::serving("png");
        let (service, records) = service(source.clone(), true);
        records
            .create("Docs", "https://example.com/", None)
            .unwrap();
        service.start().unwrap();
        let added = wait_until(|| !source.asked().is_empty());
        assert!(added, "the first sweep should have run");

        let before = service.fingerprint();
        records
            .create("Other", "https://other.example/", None)
            .unwrap();
        assert_ne!(service.fingerprint(), before, "the file changed");
        let followed = wait_until(|| source.asked().len() == 2);
        service.stop();
        assert!(
            followed,
            "a new quicklink should be fetched without a restart"
        );
    }

    /// Polls rather than sleeping a fixed time, so the test is neither slow nor
    /// flaky on a loaded machine.
    fn wait_until(done: impl Fn() -> bool) -> bool {
        wait_ticks(25, done)
    }

    fn wait_ticks(ticks: usize, done: impl Fn() -> bool) -> bool {
        for _ in 0..ticks {
            if done() {
                return true;
            }
            std::thread::sleep(TICK);
        }
        false
    }

    #[test]
    fn turning_the_preference_back_on_resumes_fetching_without_a_restart() {
        let source = FakeSource::serving("png");
        let (service, records, switch) = switchable_service(source.clone(), false);
        records
            .create("Docs", "https://example.com/", None)
            .unwrap();
        service.start().unwrap();
        assert!(
            !wait_ticks(5, || !source.asked().is_empty()),
            "nothing should be fetched while the preference is off"
        );

        switch.store(true, Ordering::SeqCst);
        let resumed = wait_until(|| !source.asked().is_empty());
        service.stop();
        assert!(resumed, "turning it on should not wait for the next edit");
    }

    #[test]
    fn turning_the_preference_off_ends_a_sweep_already_running() {
        let source = FakeSource::serving("png");
        let (service, records, switch) = switchable_service(source.clone(), true);
        for i in 0..20 {
            records
                .create(
                    &format!("Link {i}"),
                    &format!("https://host{i}.example/"),
                    None,
                )
                .unwrap();
        }
        // Off after the first host, so the rest of the sweep must be abandoned.
        source.on_first_fetch(move || switch.store(false, Ordering::SeqCst));
        service.sweep();
        assert_eq!(
            source.asked().len(),
            1,
            "the sweep should stop where it was"
        );
    }

    #[test]
    fn stopping_ends_the_loop() {
        let source = FakeSource::serving("png");
        let (service, records) = service(source.clone(), true);
        records
            .create("Docs", "https://example.com/", None)
            .unwrap();
        service.start().unwrap();
        service.stop();
        std::thread::sleep(TICK * 3);
        assert!(!service.running.load(Ordering::SeqCst));
    }

    /// Fetches from the real internet, so it runs only when asked for by name.
    /// The check that matters is that a live site answers with a type the
    /// webview can draw, which no fake can tell us.
    #[test]
    #[ignore]
    fn real_sites_answer_with_something_drawable() {
        let source = HttpFavicons::new().unwrap();
        for site in [
            "https://github.com/",
            "https://crates.io/",
            "https://en.wikipedia.org/",
            "https://news.ycombinator.com/",
        ] {
            let url = Url::parse(site).unwrap();
            let image = source
                .fetch(&url)
                .unwrap_or_else(|error| panic!("{site}: {error}"));
            println!(
                "{site} -> .{} ({} bytes)",
                image.extension,
                image.bytes.len()
            );
            assert!(DRAWABLE.contains(&image.extension));
            assert!(!image.bytes.is_empty());
        }
    }

    /// Backdates a file so the staleness rules can be exercised without waiting.
    fn age_file(path: &Path, by: Duration) {
        let when = SystemTime::now() - by;
        let file = std::fs::File::options().write(true).open(path).unwrap();
        file.set_modified(when).unwrap();
    }
}
