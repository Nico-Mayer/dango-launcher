//! Dango's configuration file: one `config.toml` the user keeps in their
//! dotfiles and shares over git.
//!
//! The file is the source of truth for the launcher hotkey, per-extension
//! enable/disable, settings, aliases, command and application hotkeys, and
//! the commands themselves for an extension that builds them from configuration.
//! Data stays in SQLite. Everything here is about reading that file safely:
//! it is optional, a bad file never takes the app down, and keys the running
//! version does not understand are ignored rather than rejected.

use std::collections::BTreeMap;

use global_hotkey::hotkey::Modifiers;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

mod files;
pub mod hotkey;
mod location;
pub mod schema;
mod store;
mod tables;
pub mod watch;

pub use files::{line_of, write_user_file, UserFile};
pub use hotkey::{Hotkey, HotkeyError, ParsedHotkey, PerPlatform};
pub use location::config_dir;
pub use store::FileConfig;
pub use tables::{
    table_name, AiCommandSettings, AiTable, ApplicationsTable, ClipboardTable, CommandBinding,
    ExtensionTable, PrimaryAction, QuicklinksTable, Table,
};

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("config.toml: {0}")]
    Parse(String),
    #[error("config.toml could not be read: {0}")]
    Read(String),
}

/// The whole configuration.
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub struct Config {
    /// The config schema version. Leave it at 1.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<u32>,
    /// Settings of the launcher itself.
    #[serde(default, skip_serializing_if = "Launcher::is_empty")]
    pub launcher: Launcher,
    /// CapsLock, remapped to act as the hyper modifier system-wide. On macOS
    /// it uses F18, so a hotkey with `f18` does not work while it is on.
    #[serde(default)]
    pub hyperkey: Hyperkey,
    /// Settings for the AI extension.
    #[serde(default)]
    pub ai: AiTable,
    /// Settings for the Applications extension.
    #[serde(default)]
    pub applications: ApplicationsTable,
    /// Settings for the Calculator extension.
    #[serde(default)]
    pub calculator: Table,
    /// Settings for the Clipboard History extension.
    #[serde(default)]
    pub clipboard: ClipboardTable,
    /// Settings for the Display extension.
    #[serde(default)]
    pub display: Table,
    /// Settings for the Games extension.
    #[serde(default)]
    pub games: Table,
    /// Settings for the Permissions extension.
    #[serde(default)]
    pub permissions: Table,
    /// Settings for the Quicklinks extension.
    #[serde(default)]
    pub quicklinks: QuicklinksTable,
    /// Settings for the Snippets extension.
    #[serde(default)]
    pub snippets: Table,
    /// Settings for the System extension.
    #[serde(default)]
    pub system: Table,
    /// Settings for the Updates extension.
    #[serde(default)]
    pub updates: Table,
    /// Settings for the Window Management extension.
    #[serde(default)]
    pub window_management: Table,
    /// Extensions that are not built in, keyed by extension id. Holds only
    /// what Dango itself reads for them: enabled state, aliases, and hotkeys.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub extensions: BTreeMap<String, Table>,
}

/// Whether an extension id names a built-in, which is configured in its own
/// table even when the same id also appears under `extensions`.
fn is_built_in(extension_id: &str) -> bool {
    table_name(extension_id).is_some_and(|name| BUILT_IN_TABLES.contains(&name))
}

/// The table names of the built-in extensions, each also its id after `dango.`.
pub const BUILT_IN_TABLES: [&str; 12] = [
    "ai",
    "applications",
    "calculator",
    "clipboard",
    "display",
    "games",
    "permissions",
    "quicklinks",
    "snippets",
    "system",
    "updates",
    "window-management",
];

/// CapsLock, remapped to act as the hyper modifier system-wide. While it is
/// held, Ctrl+Alt+Shift+Super are in effect, so a chord like `hyper+left`
/// fires from this one key.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Hyperkey {
    /// Whether CapsLock acts as the hyper modifier. Default false.
    #[serde(default)]
    pub enabled: bool,
    /// Whether Shift is part of the combination. Default true; false emits
    /// Ctrl+Alt+Super, which leaves typed characters unshifted.
    #[serde(default = "default_true", skip_serializing_if = "is_true")]
    pub shift: bool,
    /// A key to send on a quick, solitary tap of the hyperkey, such as
    /// `escape`. A hold, or a press with another key, still acts as the
    /// modifier. Leave it out for no tap action.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tap: Option<String>,
}

impl Default for Hyperkey {
    fn default() -> Self {
        Self {
            enabled: false,
            shift: true,
            tap: None,
        }
    }
}

fn default_true() -> bool {
    true
}

fn is_true(value: &bool) -> bool {
    *value
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub struct Launcher {
    /// The chord that summons the launcher. Default alt+space, which is
    /// Option+Space on macOS.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hotkey: Option<Hotkey>,
    /// Start Dango when you log in, on the machine that reads this file.
    /// Default false. A development build ignores it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub autostart: Option<bool>,
    /// Seconds the launcher keeps what you were doing after it is hidden by
    /// the hotkey or by losing focus. Pressing the hotkey again within this
    /// time continues where you left off. Default 60. 0 resets to root search
    /// at once.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reset_after_seconds: Option<u32>,
    /// The display the launcher and its notice open on. "pointer" is the
    /// display under the mouse pointer, "primary" is your main display.
    /// Default "pointer".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display: Option<LauncherDisplay>,
}

impl Launcher {
    fn is_empty(&self) -> bool {
        self.hotkey.is_none()
            && self.autostart.is_none()
            && self.reset_after_seconds.is_none()
            && self.display.is_none()
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum LauncherDisplay {
    #[default]
    Pointer,
    Primary,
}

/// One model service. The key it is stored under is the name a command uses to
/// refer to it, and the name its key in `auth.toml` is filed under.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub struct ProviderSettings {
    pub kind: ProviderKind,
    /// Where the service lives. Anthropic needs none; an OpenAI-compatible
    /// endpoint always does; Ollama defaults to the local one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base_url: Option<String>,
    /// The model used by a command that names this provider but no model.
    /// Optional for `claude` and `pi`, which then use their own default.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// Absolute path to the program, for `claude` and `pi`. Without it Dango
    /// looks on PATH and in the usual install folders.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<std::path::PathBuf>,
}

/// The protocol a provider speaks. `openai` covers any OpenAI-compatible
/// endpoint, such as OpenRouter, LM Studio, or vLLM. `claude` and `pi` run that
/// CLI with the sign-in it already has.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum ProviderKind {
    Anthropic,
    Openai,
    Ollama,
    Claude,
    Pi,
}

impl ProviderKind {
    /// Whether the provider is a program Dango runs rather than a service it
    /// calls.
    pub fn is_agent(self) -> bool {
        matches!(self, ProviderKind::Claude | ProviderKind::Pi)
    }
}

/// How hard the model should think before answering. A model that cannot think
/// ignores it.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum Thinking {
    #[default]
    Off,
    Low,
    Medium,
    High,
}

/// What becomes of an AI command's answer once it is complete.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum Output {
    /// Left on screen with actions to paste or copy it.
    #[default]
    View,
    Paste,
    Copy,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
pub struct AppSettings {
    /// The application's shown name when it differs from the key, either one
    /// string or one per platform. A side with no value means the entry is
    /// skipped on that platform.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<AppName>,
    /// The chord that opens the application, or brings it to the front.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hotkey: Option<Hotkey>,
}

/// An application's shown name: one string for both platforms, or one per
/// platform, the same two shapes a hotkey may take.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum AppName {
    Portable(String),
    PerPlatform(PerPlatform),
}

impl AppSettings {
    /// The name to look for on this platform, or `None` when the entry only
    /// names the other platform's application and so is not for this machine.
    pub fn name_for_platform<'a>(&'a self, key: &'a str) -> Option<&'a str> {
        match &self.name {
            None => Some(key),
            Some(AppName::Portable(name)) => Some(name),
            Some(AppName::PerPlatform(per)) => per.for_platform(),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
pub struct CommandSettings {
    /// A global hotkey that runs this command from anywhere. A no-view command
    /// runs without showing the launcher; a view command shows its view.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hotkey: Option<Hotkey>,
    /// A short alias that matches this command in root search.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub alias: Option<String>,
    /// Whether the command is offered. Absent means yes; false removes a
    /// command the extension ships.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
}

impl Config {
    /// Parses configuration from TOML text. Unknown keys are ignored.
    pub fn parse(text: &str) -> Result<Config, ConfigError> {
        toml::from_str(text).map_err(|error| ConfigError::Parse(error.to_string()))
    }

    /// Loads from a path. A missing file is the default configuration, not an
    /// error, because the file is optional.
    pub fn load(path: &std::path::Path) -> Result<Config, ConfigError> {
        match std::fs::read_to_string(path) {
            Ok(text) => Config::parse(&text),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Config::default()),
            Err(error) => Err(ConfigError::Read(error.to_string())),
        }
    }

    /// The table an extension is configured under: its own top-level table
    /// for a built-in, its entry under `extensions` otherwise.
    pub fn table(&self, extension_id: &str) -> Option<&dyn ExtensionTable> {
        let built_in: Option<&dyn ExtensionTable> = match table_name(extension_id) {
            Some("ai") => Some(&self.ai),
            Some("applications") => Some(&self.applications),
            Some("calculator") => Some(&self.calculator),
            Some("clipboard") => Some(&self.clipboard),
            Some("display") => Some(&self.display),
            Some("games") => Some(&self.games),
            Some("permissions") => Some(&self.permissions),
            Some("quicklinks") => Some(&self.quicklinks),
            Some("snippets") => Some(&self.snippets),
            Some("system") => Some(&self.system),
            Some("updates") => Some(&self.updates),
            Some("window-management") => Some(&self.window_management),
            _ => None,
        };
        built_in.or_else(|| {
            self.extensions
                .get(extension_id)
                .map(|table| table as &dyn ExtensionTable)
        })
    }

    /// Every table with its extension id, in id order, so walking them is
    /// deterministic.
    pub fn tables(&self) -> Vec<(String, &dyn ExtensionTable)> {
        let mut tables: Vec<(String, &dyn ExtensionTable)> = BUILT_IN_TABLES
            .iter()
            .map(|name| format!("dango.{name}"))
            .chain(
                self.extensions
                    .keys()
                    .filter(|id| !is_built_in(id))
                    .cloned(),
            )
            .filter_map(|id| self.table(&id).map(|table| (id, table)))
            .collect();
        tables.sort_by(|a, b| a.0.cmp(&b.0));
        tables
    }

    /// The enabled state the file declares for an extension, if any.
    pub fn enabled(&self, extension_id: &str) -> Option<bool> {
        self.table(extension_id)?.enabled()
    }

    /// The enabled state the file declares for one command, if any.
    pub fn command_enabled(&self, extension_id: &str, command_id: &str) -> Option<bool> {
        self.table(extension_id)?.command(command_id)?.enabled
    }

    /// The hyperkey's settings when it is turned on.
    pub fn active_hyperkey(&self) -> Option<&Hyperkey> {
        Some(&self.hyperkey).filter(|hyperkey| hyperkey.enabled)
    }

    /// The modifier set the hyperkey emits, and therefore what a `hyper` chord
    /// must expand to for the two to match. Always Ctrl+Alt+Super, plus Shift
    /// unless the hyperkey turns it off. With no hyperkey configured this is the
    /// full four, so a `hyper` chord stays registrable.
    pub fn hyper_modifiers(&self) -> Modifiers {
        let shift = self.active_hyperkey().is_none_or(|hyperkey| hyperkey.shift);
        let mut mods = Modifiers::CONTROL | Modifiers::ALT | Modifiers::SUPER;
        if shift {
            mods |= Modifiers::SHIFT;
        }
        mods
    }

    /// A command alias the file sets.
    pub fn alias(&self, extension_id: &str, command_id: &str) -> Option<String> {
        self.table(extension_id)?
            .command(command_id)?
            .alias
            .map(str::to_string)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_full_file_parses() {
        let text = r#"
            version = 1
            launcher = { hotkey = "alt+space" }
            clipboard = { enabled = false, entries = 200 }
            window-management = { commands = { left-half = { alias = "lh" } } }
        "#;
        let config = Config::parse(text).unwrap();
        assert_eq!(config.version, Some(1));
        assert_eq!(config.enabled("dango.clipboard"), Some(false));
        assert_eq!(config.clipboard.entries, 200);
        assert_eq!(
            config.clipboard.total_megabytes, 256,
            "unset keeps its default"
        );
        assert_eq!(
            config.alias("dango.window-management", "left-half"),
            Some("lh".to_string())
        );
    }

    #[test]
    fn the_launcher_display_parses_and_defaults_to_absent() {
        let config = Config::parse(r#"launcher = { display = "primary" }"#).unwrap();
        assert_eq!(config.launcher.display, Some(LauncherDisplay::Primary));
        assert_eq!(Config::parse("").unwrap().launcher.display, None);
    }

    #[test]
    fn an_unknown_launcher_display_names_the_setting() {
        let error = Config::parse(r#"launcher = { display = "secondary" }"#).unwrap_err();
        assert!(error.to_string().contains("display"), "{error}");
    }

    #[test]
    fn unknown_keys_are_ignored() {
        let text = r#"
            version = 1
            somethingFuturistic = { on = true }
            extensions = { "dango.future" = { enabled = true, somethingNew = 42 } }

            [ai]
            providers = { local = { kind = "ollama", keepAlive = "10m" } }
            commands = { make-longer = { enabled = false, later = [1, 2] } }
        "#;
        let config = Config::parse(text).unwrap();
        assert_eq!(config.enabled("dango.future"), Some(true));
        assert_eq!(
            config.command_enabled("dango.ai", "make-longer"),
            Some(false)
        );
    }

    #[test]
    fn enabled_alone_turns_the_hyperkey_on_with_defaults() {
        let config = Config::parse("hyperkey = { enabled = true }").unwrap();
        let hyperkey = config.active_hyperkey().expect("hyperkey on");
        assert!(hyperkey.shift, "shift defaults on");
        assert!(hyperkey.tap.is_none());
        assert_eq!(
            config.hyper_modifiers(),
            Modifiers::CONTROL | Modifiers::ALT | Modifiers::SHIFT | Modifiers::SUPER
        );
    }

    #[test]
    fn a_hyperkey_can_exclude_shift() {
        let config = Config::parse("hyperkey = { enabled = true, shift = false }").unwrap();
        assert!(!config.active_hyperkey().unwrap().shift);
        assert_eq!(
            config.hyper_modifiers(),
            Modifiers::CONTROL | Modifiers::ALT | Modifiers::SUPER
        );
    }

    #[test]
    fn a_leftover_hyperkey_key_is_ignored() {
        let config = Config::parse(r#"hyperkey = { enabled = true, key = "f13" }"#).unwrap();
        assert!(config.active_hyperkey().is_some());
    }

    #[test]
    fn a_hyperkey_table_without_enabled_leaves_it_off() {
        let config = Config::parse(r#"hyperkey = { tap = "escape" }"#).unwrap();
        assert!(config.active_hyperkey().is_none());
        assert!(Config::parse("[hyperkey]")
            .unwrap()
            .active_hyperkey()
            .is_none());
    }

    #[test]
    fn no_hyperkey_still_means_full_hyper() {
        let config = Config::parse("").unwrap();
        assert!(config.active_hyperkey().is_none());
        assert_eq!(
            config.hyper_modifiers(),
            Modifiers::CONTROL | Modifiers::ALT | Modifiers::SHIFT | Modifiers::SUPER
        );
    }

    #[test]
    fn a_missing_section_falls_back() {
        let config = Config::parse("").unwrap();
        assert_eq!(config.version, None);
        assert_eq!(config.enabled("anything"), None);
        assert!(config.table("anything").is_none());
        assert!(config.launcher.hotkey.is_none());
    }

    #[test]
    fn a_built_in_is_read_from_its_own_table() {
        let config = Config::parse(
            r#"
            [clipboard]
            enabled = false
            commands.history = { alias = "ch", enabled = false }

            [extensions."author.x"]
            enabled = false
            commands.go = { alias = "g" }
        "#,
        )
        .unwrap();
        assert_eq!(config.enabled("dango.clipboard"), Some(false));
        assert_eq!(
            config.command_enabled("dango.clipboard", "history"),
            Some(false)
        );
        assert_eq!(
            config.alias("dango.clipboard", "history").as_deref(),
            Some("ch")
        );
        assert_eq!(config.enabled("author.x"), Some(false));
        assert_eq!(config.alias("author.x", "go").as_deref(), Some("g"));
        assert_eq!(config.enabled("dango.games"), None);
        let ids: Vec<String> = config.tables().into_iter().map(|(id, _)| id).collect();
        assert_eq!(ids.len(), BUILT_IN_TABLES.len() + 1);
        assert!(ids.windows(2).all(|pair| pair[0] < pair[1]), "{ids:?}");
    }

    #[test]
    fn favicons_are_on_unless_turned_off() {
        assert!(Config::default().quicklinks.favicons);
        let config = Config::parse("quicklinks = { favicons = false }").unwrap();
        assert!(!config.quicklinks.favicons);
    }

    #[test]
    fn a_built_in_under_extensions_is_not_its_table() {
        let config = Config::parse(
            r#"extensions = { "dango.clipboard" = { enabled = false, commands = { history = { hotkey = "ctrl+alt+h" } } } }"#,
        )
        .unwrap();
        assert_eq!(config.enabled("dango.clipboard"), None);
        let ids: Vec<String> = config.tables().into_iter().map(|(id, _)| id).collect();
        assert_eq!(ids.len(), BUILT_IN_TABLES.len(), "no id twice: {ids:?}");
    }

    #[test]
    fn an_empty_file_is_the_default() {
        assert!(Config::parse("").is_ok());
        assert!(Config::parse("   \n  ").is_ok());
    }

    #[test]
    fn invalid_toml_is_reported_with_its_line() {
        let error = Config::parse(
            "version = 1
launcher = { not toml",
        )
        .unwrap_err();
        assert!(matches!(error, ConfigError::Parse(_)));
        assert!(error.to_string().starts_with("config.toml: "), "{error}");
        assert!(error.to_string().contains("line 2"), "{error}");
    }

    #[test]
    fn a_wrong_typed_value_is_reported() {
        // `enabled` must be a boolean; a string is a typed error, not a panic.
        let error = Config::parse(r#"extensions = { x = { enabled = "yes" } }"#).unwrap_err();
        assert!(matches!(error, ConfigError::Parse(_)));
    }

    #[test]
    fn load_of_a_missing_file_is_the_default() {
        let path = std::env::temp_dir().join("dango-does-not-exist-xyz.toml");
        let _ = std::fs::remove_file(&path);
        let config = Config::load(&path).unwrap();
        assert_eq!(config.version, None);
    }

    #[test]
    fn application_bindings_parse_in_both_shapes() {
        let text = r#"
            [applications.hotkeys]
            Safari = { hotkey = "hyper+b" }
            elsewhere = { name = { windows = "Only There" }, hotkey = "hyper+e" }
            renamed = { name = "Visual Studio Code", hotkey = "hyper+c" }

            [applications.hotkeys.terminal]
            name = { macos = "Terminal", windows = "Windows Terminal" }
            hotkey = "hyper+t"
        "#;
        let config = Config::parse(text).unwrap();
        let apps = &config.applications.hotkeys;
        assert_eq!(apps["Safari"].name_for_platform("Safari"), Some("Safari"));
        assert_eq!(
            apps["renamed"].name_for_platform("renamed"),
            Some("Visual Studio Code")
        );
        #[cfg(target_os = "macos")]
        {
            assert_eq!(
                apps["terminal"].name_for_platform("terminal"),
                Some("Terminal")
            );
            assert_eq!(apps["elsewhere"].name_for_platform("elsewhere"), None);
        }
        #[cfg(not(target_os = "macos"))]
        {
            assert_eq!(
                apps["terminal"].name_for_platform("terminal"),
                Some("Windows Terminal")
            );
            assert_eq!(
                apps["elsewhere"].name_for_platform("elsewhere"),
                Some("Only There")
            );
        }
        assert_eq!(apps.len(), 4);
        assert!(apps["terminal"].hotkey.is_some());
        assert!(matches!(
            apps["terminal"].name,
            Some(AppName::PerPlatform(_))
        ));
    }

    #[test]
    fn a_leftover_selection_block_is_ignored() {
        assert!(Config::parse(r#"selection = { excluded-applications = ["Zed"] }"#).is_ok());
    }

    #[test]
    fn an_unrecognised_provider_kind_is_a_config_error() {
        let error =
            Config::parse(r#"ai = { providers = { future = { kind = "mistral" } } }"#).unwrap_err();
        assert!(error.to_string().contains("mistral"), "{error}");
    }

    #[test]
    fn a_program_is_not_a_provider_kind() {
        let error = Config::parse(
            r#"
                ai = { providers = { claude = { kind = "cli", command = "claude" } } }
            "#,
        )
        .unwrap_err();
        assert!(error.to_string().contains("cli"), "{error}");
    }

    #[test]
    fn codex_is_not_a_provider_kind() {
        let error =
            Config::parse(r#"ai = { providers = { codex = { kind = "codex" } } }"#).unwrap_err();
        assert!(error.to_string().contains("codex"), "{error}");
    }

    #[test]
    fn an_agent_provider_round_trips_with_its_path() {
        let text = r#"
            [ai.providers]
            claude = { kind = "claude", model = "sonnet", path = "C:/tools/claude.exe" }
            pi = { kind = "pi" }
        "#;
        let config = Config::parse(text).unwrap();
        let again = Config::parse(&toml::to_string(&config).unwrap()).unwrap();
        let providers = &again.ai.providers;

        let claude = &providers["claude"];
        assert_eq!(claude.kind, ProviderKind::Claude);
        assert_eq!(claude.model.as_deref(), Some("sonnet"));
        assert_eq!(
            claude.path.as_deref(),
            Some(std::path::Path::new("C:/tools/claude.exe"))
        );
        assert_eq!(providers["pi"].kind, ProviderKind::Pi);
        assert!(providers["pi"].path.is_none());
        assert!(ProviderKind::Claude.is_agent() && ProviderKind::Pi.is_agent());
        assert!(!ProviderKind::Ollama.is_agent());
    }
}
