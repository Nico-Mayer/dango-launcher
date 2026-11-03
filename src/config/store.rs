//! The config file behind the `PreferenceStore` and `EnabledStore` traits.
//! A value the file does not set falls through to the declared default.

use std::path::PathBuf;
use std::sync::{Arc, Mutex, RwLock};

use super::files::{write_user_file, UserFile};
use super::Config;
use crate::extension::EnabledStore;

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

    /// The file's path, creating the file first when there is none. A file
    /// that exists is left alone, whatever it contains.
    pub fn ensure_file(&self) -> Result<PathBuf, String> {
        if !self.path.exists() {
            write_user_file(&self.path, &UserFile::Config.header(), &self.own_write)
                .map_err(|error| format!("{}: {error}", self.path.display()))?;
        }
        Ok(self.path.clone())
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

    fn is_command_enabled(&self, extension_id: &str, command_id: &str) -> bool {
        self.config
            .read()
            .unwrap()
            .command_enabled(extension_id, command_id)
            .unwrap_or(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_path(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("dango-cfg-test-{name}-{}.toml", std::process::id()))
    }

    #[test]
    fn enabled_defaults_to_true_when_absent() {
        let store = FileConfig::new(temp_path("en"), Config::default());
        assert!(store.is_enabled("anything"));
    }

    #[test]
    fn disabled_in_the_file_reads_disabled() {
        let text = r#"extensions = { x = { enabled = false } }"#;
        let store = FileConfig::new(temp_path("dis"), Config::parse(text).unwrap());
        assert!(!store.is_enabled("x"));
    }

    #[test]
    fn a_command_is_enabled_unless_the_file_says_false() {
        let text = r#"
            [extensions.x.commands]
            on = { enabled = true }
            off = { enabled = false }
            plain = { alias = "p" }
        "#;
        let store = FileConfig::new(temp_path("cmd"), Config::parse(text).unwrap());
        assert!(store.is_command_enabled("x", "on"));
        assert!(!store.is_command_enabled("x", "off"));
        assert!(store.is_command_enabled("x", "plain"));
        assert!(store.is_command_enabled("x", "missing"));
        assert!(store.is_command_enabled("y", "missing"));
    }

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "dango-cfg-test-{name}-{}-{}",
            std::process::id(),
            uuid::Uuid::new_v4()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn ensure_file_creates_a_missing_file_pointing_at_the_schema() {
        let dir = temp_dir("ensure-missing");
        let path = dir.join("nested").join("config.toml");
        let store = FileConfig::new(path.clone(), Config::default());
        assert_eq!(store.ensure_file().unwrap(), path);
        let on_disk = std::fs::read_to_string(&path).unwrap();
        assert_eq!(on_disk, "#:schema ./.schemas/config.schema.json\n");
        assert!(Config::parse(&on_disk).is_ok());
        assert_eq!(
            store.own_write.lock().unwrap().as_deref(),
            Some(on_disk.as_str())
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn ensure_file_leaves_an_existing_file_alone() {
        let dir = temp_dir("ensure-existing");
        let path = dir.join("config.toml");
        std::fs::write(&path, "not json at all").unwrap();
        let store = FileConfig::new(path.clone(), Config::default());
        assert_eq!(store.ensure_file().unwrap(), path);
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "not json at all");
        assert!(store.own_write.lock().unwrap().is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn ensure_file_names_the_path_when_it_cannot_write() {
        let dir = temp_dir("ensure-unwritable");
        // A regular file where the directory should be: nothing can be
        // created under it on any platform.
        let blocker = dir.join("blocker");
        std::fs::write(&blocker, "").unwrap();
        let path = blocker.join("config.toml");
        let store = FileConfig::new(path.clone(), Config::default());
        let error = store.ensure_file().unwrap_err();
        assert!(error.contains(&path.display().to_string()), "{error}");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
