//! Building the core: the store, every built-in extension, search, hotkeys,
//! and the watchers. Runs once, on the main thread, before the first show.

use std::path::PathBuf;
use std::sync::{Arc, Mutex, Weak};

use tokio::runtime::Handle;

use super::{aliased_candidates, apply_hyperkey, AiParts, Core, HideLauncher, OnMainThread};
use super::{Services, ViewRunner, RESULT_LIMIT};
use crate::config::{self, FileConfig};
use crate::extension::{self, EnabledStore, ExtensionHost, HostResolver};
use crate::extensions::ai::{AuthFile, GenAiClient};
use crate::extensions::applications::{AppIndex, ApplicationsExtension, IconCache};
use crate::extensions::clipboard::{
    ClipboardExtension, ClipboardSource, History, PreferencePolicy, Watcher,
};
use crate::extensions::permissions::PermissionsExtension;
use crate::extensions::snippets::{self, SnippetsExtension};
use crate::extensions::system::SystemExtension;
use crate::extensions::updates::{self, UpdateSource, UpdatesExtension};
use crate::extensions::window_management::WindowManagementExtension;
use crate::extensions::{self};
use crate::hotkeys::{self, Grab, Registry, SystemGrab};
use crate::invocation::Invoker;
use crate::opener::SystemOpener;
use crate::platform;
use crate::ranking::{DangoRanker, FrecencyTable, InputHistory};
use crate::search::{SearchPipeline, StaticCommandSource};
use crate::shell::{Shell, TrayStatus};
use crate::store::{Opened, Store};
use crate::text;

const IDENTIFIER: &str = "com.nimayer.dango";

#[cfg(target_os = "macos")]
const SHORTCUT_LABEL: &str = "Option+Space";
#[cfg(not(target_os = "macos"))]
const SHORTCUT_LABEL: &str = "Alt+Space";

pub struct Startup {
    pub shell: Arc<dyn Shell>,
    pub runtime: Handle,
    pub updates: Arc<dyn UpdateSource>,
}

/// Where cached icons, favicons, and clipboard images live.
pub fn cache_dir() -> PathBuf {
    dirs::cache_dir()
        .map(|dir| dir.join(IDENTIFIER))
        .unwrap_or_else(std::env::temp_dir)
}

fn data_dir() -> Option<PathBuf> {
    dirs::data_dir().map(|dir| dir.join(IDENTIFIER))
}

/// Builds the core and starts everything that runs beside it. Main thread
/// only: the hotkeys and the clipboard's activation observer register there.
pub fn start(startup: Startup) -> Arc<Core> {
    let config_path = config::config_path();
    let (initial_config, config_error) = match config::Config::load(&config_path) {
        Ok(config) => (config, None),
        Err(error) => {
            eprintln!("[dango] config file error, using defaults: {error}");
            (config::Config::default(), Some(error.to_string()))
        }
    };
    let file_config = Arc::new(FileConfig::new(config_path, initial_config));

    let core = Arc::new_cyclic(|weak: &Weak<Core>| {
        let mut notices = Vec::new();
        let services = build_services(&startup, &file_config, weak, &mut notices);
        let tray = TrayStatus {
            notices,
            config_error: config_error.is_some(),
            missing: platform::permissions().missing(),
        };
        Core::new(
            startup.shell.clone(),
            startup.runtime.clone(),
            file_config.clone(),
            tray,
            services,
        )
    });

    {
        let core = core.clone();
        global_hotkey::GlobalHotKeyEvent::set_event_handler(Some(move |event| {
            core.on_hotkey(event)
        }));
    }
    core.shell.set_tray(&core.tray());
    core.watch_permissions();
    if config_error.is_some() {
        crate::log::append("config file did not parse at startup, using defaults");
    }
    watch_config(&core);
    core
}

/// Applies edits to the config file live, on the main thread.
fn watch_config(core: &Arc<Core>) {
    let weak = Arc::downgrade(core);
    let started = config::watch::watch(
        config::config_path(),
        core.config.own_write(),
        move |text| {
            let Some(core) = weak.upgrade() else {
                return;
            };
            let shell = core.shell.clone();
            shell.on_main_thread(Box::new(move || {
                core.apply_config_reload(config::Config::parse(&text));
            }));
        },
    );
    if !started {
        crate::log::append("config file watching is off; edits need a restart");
    }
}

fn open_store() -> Result<Opened, Box<dyn std::error::Error>> {
    let dir = data_dir().ok_or("no data directory")?;
    Ok(Store::open(&dir.join("dango.sqlite"))?)
}

fn register(host: &mut ExtensionHost, name: &str, extension: Arc<dyn extension::Extension>) {
    match host.register(extension) {
        Ok(report) if report.is_clean() => {}
        Ok(report) => eprintln!("[dango] {name} loaded with issues: {report:?}"),
        Err(error) => eprintln!("[dango] {name} failed to load: {error}"),
    }
}

/// The tray menu is the only surface for startup problems, so they are
/// collected into `notices`. None of them stops Dango from starting.
fn build_services(
    startup: &Startup,
    file_config: &Arc<FileConfig>,
    core: &Weak<Core>,
    notices: &mut Vec<String>,
) -> Services {
    let store: Option<Arc<Store>> = match open_store() {
        Ok(Opened {
            store,
            recovered_from,
        }) => {
            if let Some(aside) = recovered_from {
                eprintln!("[dango] database was corrupt, moved to {}", aside.display());
                notices.push("Database was reset, see dango.log".to_string());
            }
            eprintln!(
                "[dango] store ready at schema version {}",
                store.schema_version().unwrap_or(0)
            );
            Some(Arc::new(store))
        }
        Err(error) => {
            eprintln!("[dango] database unavailable: {error}");
            notices.push("Database unavailable, see dango.log".to_string());
            None
        }
    };

    // Enable/disable lives in the config file, so the host reads it from there
    // rather than the database.
    let enabled: Arc<dyn EnabledStore> = file_config.clone();
    let mut host = ExtensionHost::new(enabled);

    let index = AppIndex::new(platform::app_indexer(), store.clone());
    let icons = IconCache::new(cache_dir().join("icons"));
    let applications = Arc::new(ApplicationsExtension::new(index, icons.clone()));
    register(&mut host, "applications", applications.clone());

    // Registering for application activations needs the main thread, which
    // this is; the watcher itself runs on its own.
    let attribution = platform::attribution();
    let clipboard = extensions::clipboard::CrateClipboard::new();
    if clipboard.is_none() {
        eprintln!("[dango] no clipboard available, so history and pasting are off");
    }

    let history = match (&store, &clipboard) {
        (Some(store), Some(clipboard)) => {
            let history = Arc::new(History::new(store.clone(), cache_dir().join("clipboard")));
            let policy = Arc::new(PreferencePolicy(extension::Preferences::new(
                extensions::clipboard::EXTENSION_ID,
                extensions::clipboard::preference_declarations(),
                file_config.clone(),
            )));
            let watcher = Arc::new(Watcher::new(
                clipboard.clone(),
                attribution,
                history.clone(),
                policy,
            ));
            Some((history, watcher))
        }
        (None, _) => {
            eprintln!("[dango] no database, so clipboard history is off");
            None
        }
        (Some(_), None) => None,
    };

    let exchange = clipboard.as_ref().and_then(|clipboard| {
        // The watcher is what keeps a paste out of the history, so the exchange
        // is built against it whenever there is one.
        let own_writes: Arc<dyn text::OwnWrites> = match &history {
            Some((_, watcher)) => watcher.clone(),
            None => Arc::new(text::Unwatched),
        };
        platform::text_exchange(
            clipboard.clone(),
            own_writes,
            Arc::new(HideLauncher(core.clone())),
            Arc::new(OnMainThread(startup.shell.clone())),
            Some(refuses_fallback(file_config.clone())),
        )
        .map(Arc::new)
    });
    let exchange: Option<Arc<dyn text::TextTarget>> = match exchange {
        Some(exchange) => Some(exchange),
        None => {
            eprintln!("[dango] no key injection or clipboard, so pasting is off");
            notices.push("Pasting is off, see dango.log".to_string());
            None
        }
    };

    if let Some((history, watcher)) = history {
        let preferences = extension::Preferences::new(
            extensions::clipboard::EXTENSION_ID,
            extensions::clipboard::preference_declarations(),
            file_config.clone(),
        );
        let extension = Arc::new(ClipboardExtension::new(
            history,
            watcher,
            preferences,
            exchange.clone(),
        ));
        register(&mut host, "clipboard", extension);
    }

    register_snippets(&mut host, file_config, exchange.clone(), core, notices);

    let system = Arc::new(SystemExtension::new(
        platform::system_control(),
        icons.clone(),
        file_config.clone(),
        Arc::new(SystemOpener),
    ));
    register(&mut host, "system", system);

    let permissions = Arc::new(PermissionsExtension::new(
        platform::permissions(),
        Arc::new(SystemOpener),
    ));
    register(&mut host, "permissions", permissions);

    // Registered after permissions, so a permission warning leads the update item.
    let updates = Arc::new(UpdatesExtension::new(
        startup.updates.clone(),
        env!("CARGO_PKG_VERSION"),
        !cfg!(debug_assertions),
        startup.runtime.clone(),
        Some(Arc::new(ViewRunner {
            core: core.clone(),
            owner: updates::EXTENSION_ID,
        })),
    ));
    register(&mut host, "updates", updates.clone());
    updates.schedule();

    let window_management = Arc::new(WindowManagementExtension::new(platform::window_manager(
        Arc::new(OnMainThread(startup.shell.clone())),
    )));
    register(&mut host, "window-management", window_management);

    if let Err(error) =
        config::schema::write(&config::config_dir(), &extensions::builtin_manifests())
    {
        crate::log::append(&format!(
            "config: could not write {}: {error}",
            config::schema::FILE_NAME
        ));
    }

    let ai = AiParts {
        keys: Arc::new(AuthFile::new(config::auth_path())),
        client: Arc::new(GenAiClient::new(startup.runtime.clone())),
        text: exchange.clone(),
        clipboard: clipboard
            .as_ref()
            .map(|clipboard| clipboard.clone() as Arc<dyn ClipboardSource>),
        runner: Arc::new(ViewRunner {
            core: core.clone(),
            owner: extensions::ai::EXTENSION_ID,
        }),
    };
    {
        let (extension, problems) = {
            let config = file_config.shared();
            let config = config.read().unwrap();
            ai.build(&config)
        };
        for problem in &problems {
            crate::log::append(problem);
        }
        if !problems.is_empty() {
            notices.push("An AI command has an error, see dango.log".to_string());
        }
        register(&mut host, "ai", extension);
    }

    let frecency = Arc::new(match &store {
        Some(store) => FrecencyTable::load(Box::new(store.clone())),
        None => FrecencyTable::in_memory(),
    });
    let learned = Arc::new(match &store {
        Some(store) => InputHistory::load(Box::new(store.clone())),
        None => InputHistory::in_memory(),
    });
    let ranker = Arc::new(DangoRanker::new(frecency.clone(), learned.clone()));
    let commands = StaticCommandSource(aliased_candidates(&host, file_config));
    let pipeline = SearchPipeline::new(
        Arc::new(commands),
        host.root_providers(),
        ranker,
        RESULT_LIMIT,
    );
    let tints = host.tints();

    // Shared rather than owned by the invoker, because resolving a command has
    // to see the host as it is now: an extension disabled since startup must no
    // longer be invocable.
    let host = Arc::new(Mutex::new(host));
    let invoker = Arc::new(Invoker::new(Arc::new(HostResolver(host.clone()))));

    let hotkeys = register_hotkeys(file_config, notices);
    let hyperkey = Mutex::new(None);
    {
        let config = file_config.shared();
        let config = config.read().unwrap();
        if let Some(message) = apply_hyperkey(&config, &hyperkey) {
            crate::log::append(&message);
            notices.push("Hyperkey didn't start, see dango.log".to_string());
        }
        if let Err(message) = crate::autostart::apply(config.launcher.autostart.unwrap_or(false)) {
            crate::log::append(&message);
            notices.push("Autostart couldn't be applied, see dango.log".to_string());
        }
    }

    Services {
        host,
        invoker,
        pipeline,
        frecency,
        learned,
        tints,
        applications: Some(applications),
        ai: Some(ai),
        hotkeys,
        hyperkey,
    }
}

/// Snippets and quicklinks are file-backed, so they load from the config
/// directory and share one extension type, differing in their file and what
/// they do on confirm.
fn register_snippets(
    host: &mut ExtensionHost,
    file_config: &Arc<FileConfig>,
    exchange: Option<Arc<dyn text::TextTarget>>,
    core: &Weak<Core>,
    notices: &mut Vec<String>,
) {
    let opener: Arc<dyn snippets::OpenUrl> = Arc::new(SystemOpener);
    // Keyword expansion skips the applications the clipboard history is told to
    // skip: one list, read fresh each keystroke.
    let excluded: snippets::ExcludedApps = {
        use extensions::clipboard::Policy;
        let policy = PreferencePolicy(extension::Preferences::new(
            extensions::clipboard::EXTENSION_ID,
            extensions::clipboard::preference_declarations(),
            file_config.clone(),
        ));
        Arc::new(move || policy.excluded_applications())
    };
    let records_dir = config::config_dir();
    // Quicklinks wear the icon of the site they open, cached beside the
    // application icons.
    let favicons = snippets::FaviconCache::new(cache_dir().join("favicons"));
    let favicon_source = snippets::HttpFavicons::new();
    for kind in [snippets::Kind::Snippet, snippets::Kind::Quicklink] {
        let (records, load_error) = snippets::Records::open(&records_dir, kind);
        if let Some(error) = load_error {
            crate::log::append(&format!("{}: {error}", kind.file_name()));
            notices.push(format!("{} has an error, see dango.log", kind.file_name()));
        }
        // Apply hand-edits to the file live, keeping the last good records on a
        // parse failure.
        {
            let records = records.clone();
            let core = core.clone();
            let file = kind.file_name();
            config::watch::watch(
                records.path().to_path_buf(),
                records.own_write(),
                move |text| {
                    if let Err(error) = records.reload(&text) {
                        if let Some(core) = core.upgrade() {
                            core.report_problem(&format!("{file}: {error}"));
                        }
                    }
                },
            );
        }
        let mut extension = SnippetsExtension::new(
            records,
            exchange.clone(),
            Some(opener.clone()),
            excluded.clone(),
        );
        if let Some(source) = favicon_source.clone() {
            let preferences = extension::Preferences::new(
                snippets::extension_id(kind),
                snippets::preference_declarations(kind),
                file_config.clone(),
            );
            let wanted: snippets::FaviconsEnabled =
                Arc::new(move || preferences.boolean(snippets::PREF_FAVICONS).unwrap_or(true));
            extension = extension.with_favicons(favicons.clone(), source, wanted);
        }
        register(host, &format!("{kind:?}"), Arc::new(extension));
    }
}

fn register_hotkeys(file_config: &Arc<FileConfig>, notices: &mut Vec<String>) -> Registry {
    let config = file_config.shared();
    let config = config.read().unwrap();
    let launcher = hotkeys::launcher(&config);
    let grab: Box<dyn Grab> = match SystemGrab::new() {
        Ok(grab) => Box::new(grab),
        Err(error) => {
            eprintln!("[dango] global hotkeys are unavailable: {error}");
            Box::new(NoGrab)
        }
    };
    let registry = Registry::new(grab, launcher);
    if let Err(error) = registry.register_launcher() {
        eprintln!("[dango] could not register the launcher hotkey: {error}");
        notices.push(format!("{SHORTCUT_LABEL} is in use, set launcher.hotkey"));
    }
    let conflicts = registry.apply(&config);
    if !conflicts.is_empty() {
        for conflict in &conflicts {
            crate::log::append(conflict);
        }
        notices.push("Some hotkeys are already in use, see dango.log".to_string());
    }
    registry
}

/// Stands in when the OS refuses a hotkey manager, so the rest still starts.
struct NoGrab;

impl Grab for NoGrab {
    fn register(&self, _chord: global_hotkey::hotkey::HotKey) -> Result<(), String> {
        Err("global hotkeys are unavailable".into())
    }
    fn unregister(&self, _chord: global_hotkey::hotkey::HotKey) {}
}

/// Names the frontmost application when the user has refused the keystroke
/// fallback there. Read fresh per call, so an edit to the file applies without
/// a restart, and so the answer is about whatever is in front right now.
fn refuses_fallback(config: Arc<FileConfig>) -> text::RefusesFallback {
    Arc::new(move || {
        // The application behind the launcher, not the launcher itself.
        let application = platform::covered_app()?;
        let shared = config.shared();
        let refused = shared.read().ok()?.selection.refuses(&application);
        refused.then_some(application)
    })
}
