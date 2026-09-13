//! Dango's configuration file: one `config.json` the user keeps in their
//! dotfiles and shares over git.
//!
//! The file is the source of truth for the launcher hotkey, per-extension
//! enable/disable, preferences, and aliases. Data stays in SQLite. Everything
//! here is about reading that file safely: it is optional, a bad file never
//! takes the app down, and keys the running version does not understand are
//! preserved rather than dropped, so a newer Dango or a plugin can share the
//! file and a future settings UI can write it back without losing anything.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

pub mod hotkey;
mod location;
mod store;

pub use hotkey::{Hotkey, HotkeyError, ParsedHotkey};
pub use location::{config_dir, config_path};
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
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Config {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<u32>,
    #[serde(default, skip_serializing_if = "Launcher::is_empty")]
    pub launcher: Launcher,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub extensions: BTreeMap<String, Extension>,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Launcher {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hotkey: Option<Hotkey>,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

impl Launcher {
    fn is_empty(&self) -> bool {
        self.hotkey.is_none() && self.extra.is_empty()
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Extension {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub preferences: BTreeMap<String, Value>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub commands: BTreeMap<String, CommandSettings>,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CommandSettings {
    /// Reserved for `add-command-hotkeys`. Parsed and preserved here, not bound.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hotkey: Option<Hotkey>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub alias: Option<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub preferences: BTreeMap<String, Value>,
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
          "hyperkey": { "key": "capslock" },
          "extensions": {
            "dango.future": { "enabled": true, "somethingNew": 42 }
          }
        }"#;
        let config = Config::parse(text).unwrap();
        let json = config.to_json();
        assert!(
            json.contains("hyperkey"),
            "top-level unknown key dropped: {json}"
        );
        assert!(json.contains("capslock"));
        assert!(
            json.contains("somethingNew"),
            "per-extension unknown key dropped: {json}"
        );
        assert!(json.contains("42"));
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
}
