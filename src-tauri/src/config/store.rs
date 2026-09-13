//! The config file behind the `PreferenceStore` and `EnabledStore` traits.
//!
//! The readers never learn the store changed: they still ask for a preference
//! or an enabled state and get one. The value now comes from the file instead
//! of SQLite, and a missing value falls through to the declared default the
//! same way it did before.

use std::path::PathBuf;
use std::sync::{Arc, Mutex, RwLock};

use serde_json::Value;

use super::Config;
use crate::extension::{EnabledStore, PreferenceStore};

/// Holds the loaded configuration and answers the store traits from it. The
/// config is behind a lock so a live reload can swap it while the app reads.
pub struct FileConfig {
    path: PathBuf,
    config: Arc<RwLock<Config>>,
    /// The last text the app itself wrote, so a file watcher can tell the app's
    /// own write from a user's edit, the way the clipboard watcher does.
    own_write: Arc<Mutex<Option<String>>>,
}

impl FileConfig {
    pub fn new(path: PathBuf, config: Config) -> Self {
        Self {
            path,
            config: Arc::new(RwLock::new(config)),
            own_write: Arc::new(Mutex::new(None)),
        }
    }

    /// The shared configuration, so a reload handler can swap it and other
    /// readers can see the current values.
    pub fn shared(&self) -> Arc<RwLock<Config>> {
        self.config.clone()
    }

    /// The self-write signal, for the watcher to consult.
    pub fn own_write(&self) -> Arc<Mutex<Option<String>>> {
        self.own_write.clone()
    }

    /// Replaces the in-memory configuration on a reload. Does not write the
    /// file: the file is what was just read.
    pub fn replace(&self, config: Config) {
        *self.config.write().unwrap() = config;
    }

    fn persist(&self) {
        let text = self.config.read().unwrap().to_json();
        *self.own_write.lock().unwrap() = Some(text.clone());
        if let Some(parent) = self.path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Err(error) = std::fs::write(&self.path, text) {
            eprintln!("[dango] could not write the config file: {error}");
        }
    }
}

impl PreferenceStore for FileConfig {
    fn get(&self, extension_id: &str, command_id: Option<&str>, key: &str) -> Option<String> {
        self.config
            .read()
            .unwrap()
            .preference(extension_id, command_id, key)
    }

    fn set(&self, extension_id: &str, command_id: Option<&str>, key: &str, value: &str) {
        {
            let mut config = self.config.write().unwrap();
            let extension = config
                .extensions
                .entry(extension_id.to_string())
                .or_default();
            let preferences = match command_id {
                Some(command) => {
                    &mut extension
                        .commands
                        .entry(command.to_string())
                        .or_default()
                        .preferences
                }
                None => &mut extension.preferences,
            };
            preferences.insert(key.to_string(), Value::String(value.to_string()));
        }
        self.persist();
    }
}

impl EnabledStore for FileConfig {
    fn is_enabled(&self, extension_id: &str) -> bool {
        // Absent means enabled: a fresh install with no file runs everything.
        self.config
            .read()
            .unwrap()
            .enabled(extension_id)
            .unwrap_or(true)
    }

    fn set_enabled(&self, extension_id: &str, enabled: bool) {
        {
            let mut config = self.config.write().unwrap();
            config
                .extensions
                .entry(extension_id.to_string())
                .or_default()
                .enabled = Some(enabled);
        }
        self.persist();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_path(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("dango-cfg-test-{name}-{}.json", std::process::id()))
    }

    #[test]
    fn reads_a_preference_and_falls_back() {
        let text =
            r#"{ "extensions": { "dango.clipboard": { "preferences": { "max_entries": 200 } } } }"#;
        let store = FileConfig::new(temp_path("pref"), Config::parse(text).unwrap());
        assert_eq!(
            store.get("dango.clipboard", None, "max_entries"),
            Some("200".to_string())
        );
        assert_eq!(store.get("dango.clipboard", None, "unset"), None);
    }

    #[test]
    fn enabled_defaults_to_true_when_absent() {
        let store = FileConfig::new(temp_path("en"), Config::default());
        assert!(store.is_enabled("anything"));
    }

    #[test]
    fn disabled_in_the_file_reads_disabled() {
        let text = r#"{ "extensions": { "x": { "enabled": false } } }"#;
        let store = FileConfig::new(temp_path("dis"), Config::parse(text).unwrap());
        assert!(!store.is_enabled("x"));
    }

    #[test]
    fn a_set_persists_and_records_its_own_write() {
        let path = temp_path("set");
        let _ = std::fs::remove_file(&path);
        let store = FileConfig::new(path.clone(), Config::default());
        store.set_enabled("x", false);
        assert!(!store.is_enabled("x"));
        // The file was written and the write was recorded for the watcher.
        let on_disk = std::fs::read_to_string(&path).unwrap();
        assert!(on_disk.contains("\"enabled\": false"));
        assert_eq!(
            store.own_write.lock().unwrap().as_deref(),
            Some(on_disk.as_str())
        );
        let _ = std::fs::remove_file(&path);
    }
}
