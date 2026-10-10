//! Dango's configuration file: one `config.json` the user keeps in their
//! dotfiles and shares over git.
//!
//! The file is the source of truth for the launcher hotkey, per-extension
//! enable/disable, preferences, aliases, command and application hotkeys, and
//! the commands themselves for an extension that builds them from configuration.
//! Data stays in SQLite. Everything here is about reading that file safely:
//! it is optional, a bad file never takes the app down, and keys the running
//! version does not understand are preserved rather than dropped, so a newer
//! Dango or a plugin can share the file and a future settings UI can write it
//! back without losing anything.

use std::collections::BTreeMap;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use tauri_plugin_global_shortcut::Modifiers;

pub mod hotkey;
mod location;
pub mod schema;
mod store;
pub mod watch;

pub use hotkey::{Hotkey, HotkeyError, ParsedHotkey, PerPlatform};
pub use location::{auth_path, config_dir, config_path, SCHEMA_REF};
pub use store::FileConfig;

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("the configuration file is not valid JSON: {0}")]
    Parse(String),
    #[error("the configuration file could not be read: {0}")]
    Read(String),
}

/// The whole configuration. Every level keeps an `extra` catch-all so a key the
/// running version does not understand survives a load and a write.
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
pub struct Config {
    /// Where your editor finds this schema. Dango writes it beside the config
    /// file, so `./config.schema.json` is the value to use.
    #[serde(rename = "$schema", default, skip_serializing_if = "Option::is_none")]
    pub schema: Option<String>,
    /// The config schema version. Leave it at 1.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<u32>,
    /// Settings of the launcher itself.
    #[serde(default, skip_serializing_if = "Launcher::is_empty")]
    pub launcher: Launcher,
    /// Per-extension settings, keyed by extension id.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub extensions: BTreeMap<String, Extension>,
    /// A physical key remapped to act as the hyper modifier system-wide.
    /// Present means on; leave it out to turn it off.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hyperkey: Option<Hyperkey>,
    /// How Dango reads the text you have selected.
    #[serde(default, skip_serializing_if = "Selection::is_empty")]
    pub selection: Selection,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

/// One physical key remapped to act as the hyper modifier system-wide. While
/// it is held, Ctrl+Alt+Shift+Super are in effect, so a chord like
/// `hyper+left` fires from this one key.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Hyperkey {
    /// The physical key to remap. Defaults to capslock.
    #[serde(default = "default_hyperkey_key")]
    pub key: String,
    /// Whether Shift is part of the combination. Default true; false emits
    /// Ctrl+Alt+Super, which leaves typed characters unshifted.
    #[serde(default = "default_true", skip_serializing_if = "is_true")]
    pub shift: bool,
    /// A key to send on a quick, solitary tap of the hyperkey, such as
    /// `escape`. A hold, or a press with another key, still acts as the
    /// modifier. Leave it out for no tap action.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tap: Option<String>,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

impl Default for Hyperkey {
    fn default() -> Self {
        Self {
            key: default_hyperkey_key(),
            shift: true,
            tap: None,
            extra: Map::new(),
        }
    }
}

fn default_hyperkey_key() -> String {
    "capslock".to_string()
}

fn default_true() -> bool {
    true
}

fn is_true(value: &bool) -> bool {
    *value
}

/// How Dango reads what the user has selected. Beside `hyperkey` rather than
/// under an extension, because the text plumbing belongs to the application.
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub struct Selection {
    /// Applications where Dango must never send the copy chord to read a
    /// selection, comma separated and matched ignoring case. For editors that
    /// bind Ctrl+C to something of their own. Dango says it cannot read the
    /// selection there instead.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub excluded_applications: Option<String>,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

impl Selection {
    fn is_empty(&self) -> bool {
        self.excluded_applications.is_none() && self.extra.is_empty()
    }

    /// The names as written, split and trimmed. Empty by default: an exclusion
    /// shipped as a default would quietly disable a working feature for someone
    /// whose editor does not have the problem.
    pub fn excluded(&self) -> Vec<String> {
        match &self.excluded_applications {
            Some(value) => value
                .split(',')
                .map(str::trim)
                .filter(|name| !name.is_empty())
                .map(str::to_string)
                .collect(),
            None => Vec::new(),
        }
    }

    /// Whether the keystroke fallback is refused for this application.
    pub fn refuses(&self, application: &str) -> bool {
        self.excluded()
            .iter()
            .any(|name| name.eq_ignore_ascii_case(application))
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
pub struct Launcher {
    /// The chord that summons the launcher. Default alt+space, which is
    /// Option+Space on macOS.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hotkey: Option<Hotkey>,
    /// Start Dango when you log in, on the machine that reads this file.
    /// Default false. A development build ignores it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub autostart: Option<bool>,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

impl Launcher {
    fn is_empty(&self) -> bool {
        self.hotkey.is_none() && self.autostart.is_none() && self.extra.is_empty()
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
pub struct Extension {
    /// Whether the extension runs. Absent means enabled.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    /// The extension's preference values, overriding its defaults.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub preferences: BTreeMap<String, Value>,
    /// Per-command settings, keyed by command id.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub commands: BTreeMap<String, CommandSettings>,
    /// Applications bound to global hotkeys, keyed by the name Dango shows for
    /// the application in root search.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub apps: BTreeMap<String, AppSettings>,
    /// Model services, keyed by the name commands refer to them by. That name
    /// is also the key its API key is filed under in auth.json.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub providers: BTreeMap<String, ProviderSettings>,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

/// One model service. The key it is stored under is the name a command uses to
/// refer to it, and the name its key in `auth.json` is filed under.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ProviderSettings {
    pub kind: ProviderKind,
    /// Where the service lives. Anthropic needs none; an OpenAI-compatible
    /// endpoint always does; Ollama defaults to the local one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base_url: Option<String>,
    /// The model used by a command that names this provider but no model.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// The program to run, for a provider of the `cli` kind.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    /// What to pass it. `{prompt}` and `{model}` are replaced where they appear;
    /// with no `{prompt}`, the prompt goes in on standard input.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub args: Vec<String>,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

/// The protocol a provider speaks. `openai` covers any OpenAI-compatible
/// endpoint, such as OpenRouter, LM Studio, or vLLM. `cli` runs a program on
/// this machine that takes a prompt and prints an answer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum ProviderKind {
    Anthropic,
    Openai,
    Ollama,
    Cli,
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

impl Thinking {
    /// The levels as they are written in the config file, lowest first.
    pub const NAMES: [&'static str; 4] = ["off", "low", "medium", "high"];

    pub fn parse(text: &str) -> Option<Self> {
        match text {
            "off" => Some(Thinking::Off),
            "low" => Some(Thinking::Low),
            "medium" => Some(Thinking::Medium),
            "high" => Some(Thinking::High),
            _ => None,
        }
    }
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
    #[serde(flatten)]
    pub extra: Map<String, Value>,
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
    /// Preference values scoped to this command.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub preferences: BTreeMap<String, Value>,
    /// Whether the command is offered. Absent means yes; false removes a
    /// command the extension ships.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    /// The title root search shows. Needed for a command the file defines
    /// from nothing.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// What the model is asked, as a template. Placeholders: {{ selection }},
    /// {{ clipboard }}, and any other name, which the launcher asks for.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,
    /// The provider this command uses, by its name under providers. Absent
    /// falls back to the extension's default provider.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
    /// The model this command uses. Absent falls back to the provider's model.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// How hard the model thinks before answering. Absent falls back to the
    /// extension's default level.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thinking: Option<Thinking>,
    /// What becomes of the answer: shown in the launcher, pasted into the
    /// previous app, or copied. Default view.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output: Option<Output>,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

impl Config {
    /// Parses configuration from JSON text. Unknown keys are kept.
    pub fn parse(text: &str) -> Result<Config, ConfigError> {
        // An empty or whitespace-only file is a valid empty configuration
        // rather than a JSON error, so a `touch`ed file behaves like no file.
        if text.trim().is_empty() {
            return Ok(Config::default());
        }
        serde_json::from_str(text).map_err(|error| ConfigError::Parse(error.to_string()))
    }

    /// Serialises back to pretty JSON, preserving every preserved unknown key.
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_else(|_| "{}".to_string())
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

    /// The enabled state the file declares for an extension, if any.
    pub fn enabled(&self, extension_id: &str) -> Option<bool> {
        self.extensions.get(extension_id).and_then(|e| e.enabled)
    }

    /// A preference value the file sets, as the string the reader expects.
    /// Typed JSON is rendered to the string form: a number to its digits, a
    /// boolean to `true`/`false`, a string to itself. `command` scopes it to a
    /// command's own preferences; `None` is the extension-level value.
    pub fn preference(
        &self,
        extension_id: &str,
        command: Option<&str>,
        key: &str,
    ) -> Option<String> {
        let extension = self.extensions.get(extension_id)?;
        let value = match command {
            Some(command_id) => extension.commands.get(command_id)?.preferences.get(key)?,
            None => extension.preferences.get(key)?,
        };
        Some(match value {
            Value::String(text) => text.clone(),
            other => other.to_string(),
        })
    }

    /// The modifier set the hyperkey emits, and therefore what a `hyper` chord
    /// must expand to for the two to match. Always Ctrl+Alt+Super, plus Shift
    /// unless the hyperkey turns it off. With no hyperkey configured this is the
    /// full four, so a `hyper` chord stays registrable.
    pub fn hyper_modifiers(&self) -> Modifiers {
        let shift = self.hyperkey.as_ref().map(|h| h.shift).unwrap_or(true);
        let mut mods = Modifiers::CONTROL | Modifiers::ALT | Modifiers::SUPER;
        if shift {
            mods |= Modifiers::SHIFT;
        }
        mods
    }

    /// A command alias the file sets.
    pub fn alias(&self, extension_id: &str, command_id: &str) -> Option<String> {
        self.extensions
            .get(extension_id)?
            .commands
            .get(command_id)?
            .alias
            .clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_full_file_round_trips() {
        let text = r#"{
          "version": 1,
          "launcher": { "hotkey": "alt+space" },
          "extensions": {
            "dango.clipboard": { "enabled": false, "preferences": { "max_entries": 200 } },
            "dango.window-management": { "commands": { "left-half": { "alias": "lh" } } }
          }
        }"#;
        let config = Config::parse(text).unwrap();
        assert_eq!(config.version, Some(1));
        assert_eq!(config.enabled("dango.clipboard"), Some(false));
        assert_eq!(
            config.preference("dango.clipboard", None, "max_entries"),
            Some("200".to_string())
        );
        assert_eq!(
            config.alias("dango.window-management", "left-half"),
            Some("lh".to_string())
        );
        // Re-parsing the serialised form yields the same view.
        let again = Config::parse(&config.to_json()).unwrap();
        assert_eq!(again.enabled("dango.clipboard"), Some(false));
        assert_eq!(again.version, Some(1));
    }

    #[test]
    fn unknown_keys_are_preserved_through_a_round_trip() {
        let text = r#"{
          "version": 1,
          "somethingFuturistic": { "on": true },
          "extensions": {
            "dango.future": { "enabled": true, "somethingNew": 42 }
          }
        }"#;
        let config = Config::parse(text).unwrap();
        let json = config.to_json();
        assert!(
            json.contains("somethingFuturistic"),
            "top-level unknown key dropped: {json}"
        );
        assert!(
            json.contains("somethingNew"),
            "per-extension unknown key dropped: {json}"
        );
        assert!(json.contains("42"));
    }

    #[test]
    fn a_hyperkey_block_parses_with_defaults() {
        let config = Config::parse(r#"{ "hyperkey": { "key": "capslock" } }"#).unwrap();
        let hyperkey = config.hyperkey.as_ref().expect("hyperkey present");
        assert_eq!(hyperkey.key, "capslock");
        assert!(hyperkey.shift, "shift defaults on");
        assert_eq!(
            config.hyper_modifiers(),
            Modifiers::CONTROL | Modifiers::ALT | Modifiers::SHIFT | Modifiers::SUPER
        );
    }

    #[test]
    fn a_hyperkey_tap_key_parses_and_round_trips() {
        let config =
            Config::parse(r#"{ "hyperkey": { "key": "capslock", "tap": "escape" } }"#).unwrap();
        assert_eq!(
            config.hyperkey.as_ref().unwrap().tap.as_deref(),
            Some("escape")
        );
        let again = Config::parse(&config.to_json()).unwrap();
        assert_eq!(again.hyperkey.unwrap().tap.as_deref(), Some("escape"));
        // Absent means no tap.
        let plain = Config::parse(r#"{ "hyperkey": { "key": "capslock" } }"#).unwrap();
        assert_eq!(plain.hyperkey.unwrap().tap, None);
    }

    #[test]
    fn a_hyperkey_can_exclude_shift() {
        let config =
            Config::parse(r#"{ "hyperkey": { "key": "capslock", "shift": false } }"#).unwrap();
        assert!(!config.hyperkey.as_ref().unwrap().shift);
        assert_eq!(
            config.hyper_modifiers(),
            Modifiers::CONTROL | Modifiers::ALT | Modifiers::SUPER
        );
        // The excluded Shift also survives a round trip.
        let again = Config::parse(&config.to_json()).unwrap();
        assert!(!again.hyperkey.unwrap().shift);
    }

    #[test]
    fn no_hyperkey_still_means_full_hyper() {
        let config = Config::parse("{}").unwrap();
        assert!(config.hyperkey.is_none());
        assert_eq!(
            config.hyper_modifiers(),
            Modifiers::CONTROL | Modifiers::ALT | Modifiers::SHIFT | Modifiers::SUPER
        );
    }

    #[test]
    fn a_missing_section_falls_back() {
        let config = Config::parse("{}").unwrap();
        assert_eq!(config.version, None);
        assert_eq!(config.enabled("anything"), None);
        assert_eq!(config.preference("anything", None, "key"), None);
        assert!(config.launcher.hotkey.is_none());
    }

    #[test]
    fn an_empty_file_is_the_default() {
        assert!(Config::parse("").is_ok());
        assert!(Config::parse("   \n  ").is_ok());
    }

    #[test]
    fn invalid_json_is_reported_not_panicked() {
        let error = Config::parse("{ not json ").unwrap_err();
        assert!(matches!(error, ConfigError::Parse(_)));
    }

    #[test]
    fn a_wrong_typed_value_is_reported() {
        // `enabled` must be a boolean; a string is a typed error, not a panic.
        let error =
            Config::parse(r#"{ "extensions": { "x": { "enabled": "yes" } } }"#).unwrap_err();
        assert!(matches!(error, ConfigError::Parse(_)));
    }

    #[test]
    fn load_of_a_missing_file_is_the_default() {
        let path = std::env::temp_dir().join("dango-does-not-exist-xyz.json");
        let _ = std::fs::remove_file(&path);
        let config = Config::load(&path).unwrap();
        assert_eq!(config.version, None);
    }

    #[test]
    fn a_load_modify_write_cycle_keeps_unrelated_keys() {
        let text = r#"{ "version": 1, "keepMe": "yes", "extensions": { "x": { "enabled": true, "keepThis": 7 } } }"#;
        let mut config = Config::parse(text).unwrap();
        // Change one thing.
        config.extensions.get_mut("x").unwrap().enabled = Some(false);
        let json = config.to_json();
        assert!(json.contains("keepMe"));
        assert!(json.contains("keepThis"));
        assert!(json.contains("\"enabled\": false"));
    }

    #[test]
    fn application_bindings_parse_in_both_shapes_and_round_trip() {
        let text = r#"{
          "extensions": { "dango.applications": { "apps": {
            "Safari": { "hotkey": "hyper+b" },
            "terminal": { "name": { "macos": "Terminal", "windows": "Windows Terminal" }, "hotkey": "hyper+t" },
            "elsewhere": { "name": { "windows": "Only There" }, "hotkey": "hyper+e" },
            "renamed": { "name": "Visual Studio Code", "hotkey": "hyper+c" }
          } } }
        }"#;
        let config = Config::parse(text).unwrap();
        let apps = &config.extensions["dango.applications"].apps;
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

        let again = Config::parse(&config.to_json()).unwrap();
        let apps = &again.extensions["dango.applications"].apps;
        assert_eq!(apps.len(), 4);
        assert!(apps["terminal"].hotkey.is_some());
        assert!(matches!(
            apps["terminal"].name,
            Some(AppName::PerPlatform(_))
        ));
    }

    #[test]
    fn a_launcher_autostart_flag_parses_and_round_trips() {
        assert_eq!(Config::parse("{}").unwrap().launcher.autostart, None);
        let config = Config::parse(r#"{ "launcher": { "autostart": true } }"#).unwrap();
        assert_eq!(config.launcher.autostart, Some(true));

        let json = config.to_json();
        assert!(json.contains("autostart"), "key dropped: {json}");
        assert_eq!(Config::parse(&json).unwrap().launcher.autostart, Some(true));
    }

    #[test]
    fn a_selection_block_parses_and_round_trips() {
        let config =
            Config::parse(r#"{ "selection": { "excluded-applications": "Zed, Neovim" } }"#)
                .unwrap();
        assert_eq!(config.selection.excluded(), ["Zed", "Neovim"]);
        assert!(config.selection.refuses("zed"), "matched ignoring case");
        assert!(!config.selection.refuses("Notepad"));

        let json = config.to_json();
        assert!(
            json.contains("excluded-applications"),
            "key renamed: {json}"
        );
        assert!(Config::parse(&json).unwrap().selection.refuses("Zed"));
    }

    #[test]
    fn no_selection_block_excludes_nothing() {
        let config = Config::parse("{}").unwrap();
        assert!(config.selection.excluded().is_empty());
        assert!(!config.selection.refuses("Zed"));
        assert!(
            !config.to_json().contains("selection"),
            "an empty block was written back"
        );
    }

    #[test]
    fn a_provider_branch_parses_and_round_trips() {
        let text = r#"{
          "extensions": {
            "dango.ai": {
              "providers": {
                "anthropic": { "kind": "anthropic", "model": "claude-sonnet-5" },
                "local": { "kind": "ollama", "baseUrl": "http://localhost:11434", "keepAlive": "10m" }
              }
            }
          }
        }"#;
        let config = Config::parse(text).unwrap();
        let providers = &config.extensions["dango.ai"].providers;
        assert_eq!(providers["anthropic"].kind, ProviderKind::Anthropic);
        assert_eq!(
            providers["anthropic"].model.as_deref(),
            Some("claude-sonnet-5")
        );
        assert_eq!(providers["local"].kind, ProviderKind::Ollama);
        assert_eq!(
            providers["local"].base_url.as_deref(),
            Some("http://localhost:11434")
        );

        let json = config.to_json();
        assert!(json.contains("baseUrl"), "camelCase key lost: {json}");
        assert!(
            json.contains("keepAlive"),
            "unknown provider key dropped: {json}"
        );
        let again = Config::parse(&json).unwrap();
        assert_eq!(
            again.extensions["dango.ai"].providers["local"].kind,
            ProviderKind::Ollama
        );
    }

    #[test]
    fn a_command_provider_carries_its_command_and_arguments() {
        let config = Config::parse(
            r#"{ "extensions": { "dango.ai": { "providers": {
                "claude": { "kind": "cli", "command": "claude", "args": ["-p", "{prompt}"] }
            } } } }"#,
        )
        .unwrap();
        let entry = &config.extensions["dango.ai"].providers["claude"];
        assert_eq!(entry.kind, ProviderKind::Cli);
        assert_eq!(entry.command.as_deref(), Some("claude"));
        assert_eq!(entry.args, ["-p", "{prompt}"]);

        let again = Config::parse(&config.to_json()).unwrap();
        assert_eq!(
            again.extensions["dango.ai"].providers["claude"].args,
            ["-p", "{prompt}"]
        );
    }

    #[test]
    fn an_unrecognised_provider_kind_is_a_config_error() {
        let error = Config::parse(
            r#"{ "extensions": { "dango.ai": { "providers": { "future": { "kind": "mistral" } } } } }"#,
        )
        .unwrap_err();
        assert!(error.to_string().contains("mistral"), "{error}");
    }

    #[test]
    fn the_schema_key_round_trips_first() {
        let config =
            Config::parse(r#"{ "version": 1, "$schema": "./config.schema.json" }"#).unwrap();
        assert_eq!(config.schema.as_deref(), Some("./config.schema.json"));
        assert_eq!(
            config.to_json(),
            "{\n  \"$schema\": \"./config.schema.json\",\n  \"version\": 1\n}"
        );
    }

    #[test]
    fn ai_command_fields_parse_beside_the_hotkey_and_alias() {
        let text = r#"{
          "extensions": {
            "dango.ai": {
              "commands": {
                "improve-writing": { "hotkey": "hyper+i", "alias": "iw", "provider": "local", "thinking": "low" },
                "make-longer": { "enabled": false },
                "review-rust": {
                  "title": "Review Rust",
                  "prompt": "Review this.\n\n{{ selection }}",
                  "model": "claude-opus-5",
                  "output": "view"
                }
              }
            }
          }
        }"#;
        let config = Config::parse(text).unwrap();
        let commands = &config.extensions["dango.ai"].commands;
        assert_eq!(
            commands["improve-writing"].provider.as_deref(),
            Some("local")
        );
        assert_eq!(commands["improve-writing"].thinking, Some(Thinking::Low));
        assert_eq!(commands["make-longer"].enabled, Some(false));
        assert_eq!(
            commands["review-rust"].title.as_deref(),
            Some("Review Rust")
        );
        assert_eq!(commands["review-rust"].output, Some(Output::View));
        assert_eq!(
            config.alias("dango.ai", "improve-writing"),
            Some("iw".to_string())
        );

        let again = Config::parse(&config.to_json()).unwrap();
        let commands = &again.extensions["dango.ai"].commands;
        assert!(commands["improve-writing"].hotkey.is_some(), "hotkey lost");
        assert_eq!(
            commands["review-rust"].prompt.as_deref(),
            Some("Review this.\n\n{{ selection }}")
        );
    }
}
