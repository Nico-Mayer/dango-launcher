//! Reading an extension's preferences.
//!
//! The stored value lives in the database and the default lives in the
//! manifest, so neither half can answer on its own. A reader binds the two, and
//! is bound to one extension: an extension cannot reach another's values
//! because it is never handed a reader that could.

use std::sync::Arc;

use super::manifest::{PreferenceDecl, PreferenceKind};

/// Where preference values are kept. Abstracted so the reader is testable
/// without a database, the same way the enabled state is.
pub trait PreferenceStore: Send + Sync {
    fn get(&self, extension_id: &str, command_id: Option<&str>, key: &str) -> Option<String>;
    fn set(&self, extension_id: &str, command_id: Option<&str>, key: &str, value: &str);
}

impl PreferenceStore for crate::store::Store {
    fn get(&self, extension_id: &str, command_id: Option<&str>, key: &str) -> Option<String> {
        self.preference(extension_id, command_id, key)
            .ok()
            .flatten()
    }

    fn set(&self, extension_id: &str, command_id: Option<&str>, key: &str, value: &str) {
        if let Err(error) = self.set_preference(extension_id, command_id, key, value) {
            eprintln!("[dango] could not persist preference {extension_id}.{key}: {error}");
        }
    }
}

/// One extension's view of its own preferences.
pub struct Preferences {
    extension_id: String,
    declarations: Vec<PreferenceDecl>,
    store: Arc<dyn PreferenceStore>,
}

impl Preferences {
    pub fn new(
        extension_id: &str,
        declarations: Vec<PreferenceDecl>,
        store: Arc<dyn PreferenceStore>,
    ) -> Self {
        Self {
            extension_id: extension_id.to_string(),
            declarations,
            store,
        }
    }

    pub fn string(&self, key: &str) -> Option<String> {
        self.read(None, key, PreferenceKind::String, |raw| {
            Some(raw.to_string())
        })
    }

    pub fn number(&self, key: &str) -> Option<f64> {
        self.read(None, key, PreferenceKind::Number, |raw| raw.parse().ok())
    }

    pub fn boolean(&self, key: &str) -> Option<bool> {
        self.read(None, key, PreferenceKind::Boolean, |raw| raw.parse().ok())
    }

    /// A number read as a count, which is what every bound in practice is.
    pub fn count(&self, key: &str) -> Option<u64> {
        self.number(key)
            .filter(|value| value.is_finite() && *value >= 0.0)
            .map(|value| value as u64)
    }

    pub fn set_string(&self, key: &str, value: &str) {
        self.store.set(&self.extension_id, None, key, value);
    }

    pub fn set_number(&self, key: &str, value: f64) {
        self.store
            .set(&self.extension_id, None, key, &value.to_string());
    }

    pub fn set_boolean(&self, key: &str, value: bool) {
        self.store
            .set(&self.extension_id, None, key, &value.to_string());
    }

    /// The stored value when it parses as the declared type, the declared
    /// default otherwise. A value that no longer fits its type reads as the
    /// default rather than stopping the extension: the user did not write the
    /// schema change that invalidated it.
    fn read<T>(
        &self,
        command_id: Option<&str>,
        key: &str,
        kind: PreferenceKind,
        parse: impl Fn(&str) -> Option<T>,
    ) -> Option<T> {
        let declaration = self.declaration(command_id, key)?;
        if declaration.kind != kind {
            return None;
        }
        let stored = self
            .store
            .get(&self.extension_id, command_id, key)
            .and_then(|raw| parse(&raw));
        stored.or_else(|| {
            declaration
                .default
                .as_ref()
                .and_then(|value| parse(&as_raw(value)))
        })
    }

    fn declaration(&self, command_id: Option<&str>, key: &str) -> Option<&PreferenceDecl> {
        self.declarations
            .iter()
            .find(|d| d.key == key && d.command_id.as_deref() == command_id)
    }
}

/// Defaults are JSON, so a string default arrives quoted and everything else
/// reads the way it was written.
fn as_raw(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::String(text) => text.clone(),
        other => other.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::sync::Mutex;

    #[derive(Default)]
    struct MemoryStore(Mutex<HashMap<String, String>>);

    impl MemoryStore {
        fn key(extension_id: &str, command_id: Option<&str>, key: &str) -> String {
            format!("{extension_id}/{}/{key}", command_id.unwrap_or_default())
        }
    }

    impl PreferenceStore for MemoryStore {
        fn get(&self, extension_id: &str, command_id: Option<&str>, key: &str) -> Option<String> {
            self.0
                .lock()
                .unwrap()
                .get(&Self::key(extension_id, command_id, key))
                .cloned()
        }
        fn set(&self, extension_id: &str, command_id: Option<&str>, key: &str, value: &str) {
            self.0
                .lock()
                .unwrap()
                .insert(Self::key(extension_id, command_id, key), value.to_string());
        }
    }

    fn decl(
        command_id: Option<&str>,
        key: &str,
        kind: PreferenceKind,
        default: Option<serde_json::Value>,
    ) -> PreferenceDecl {
        PreferenceDecl {
            command_id: command_id.map(str::to_string),
            key: key.into(),
            kind,
            default,
            required: false,
        }
    }

    fn preferences(store: Arc<dyn PreferenceStore>) -> Preferences {
        Preferences::new(
            "dango.clipboard",
            vec![
                decl(None, "entries", PreferenceKind::Number, Some(500.into())),
                decl(None, "excluded", PreferenceKind::String, Some("".into())),
                decl(None, "enabled", PreferenceKind::Boolean, Some(true.into())),
                decl(None, "no-default", PreferenceKind::Number, None),
            ],
            store,
        )
    }

    #[test]
    fn an_unset_preference_reads_its_declared_default() {
        let prefs = preferences(Arc::new(MemoryStore::default()));
        assert_eq!(prefs.count("entries"), Some(500));
        assert_eq!(prefs.boolean("enabled"), Some(true));
        assert_eq!(prefs.string("excluded").as_deref(), Some(""));
    }

    #[test]
    fn a_stored_value_wins_over_the_default() {
        let store = Arc::new(MemoryStore::default());
        let prefs = preferences(store.clone());
        prefs.set_number("entries", 20.0);
        assert_eq!(prefs.count("entries"), Some(20));
    }

    #[test]
    fn a_value_that_no_longer_parses_falls_back_to_the_default() {
        let store = Arc::new(MemoryStore::default());
        store.set("dango.clipboard", None, "entries", "not a number");
        let prefs = preferences(store);
        assert_eq!(
            prefs.count("entries"),
            Some(500),
            "a value that does not fit its type must not take the extension down"
        );
    }

    #[test]
    fn a_preference_with_no_default_and_no_value_reads_as_absent() {
        let prefs = preferences(Arc::new(MemoryStore::default()));
        assert_eq!(prefs.number("no-default"), None);
    }

    #[test]
    fn reading_a_key_as_the_wrong_type_yields_nothing() {
        let prefs = preferences(Arc::new(MemoryStore::default()));
        assert_eq!(prefs.number("excluded"), None);
        assert_eq!(prefs.boolean("entries"), None);
    }

    #[test]
    fn an_undeclared_key_reads_as_absent() {
        let prefs = preferences(Arc::new(MemoryStore::default()));
        assert_eq!(prefs.string("never-declared"), None);
    }

    #[test]
    fn extension_and_command_scopes_with_one_key_stay_apart() {
        let store = Arc::new(MemoryStore::default());
        let prefs = Preferences::new(
            "dango.clipboard",
            vec![
                decl(None, "limit", PreferenceKind::Number, Some(1.into())),
                decl(
                    Some("history"),
                    "limit",
                    PreferenceKind::Number,
                    Some(2.into()),
                ),
            ],
            store.clone(),
        );
        assert_eq!(prefs.count("limit"), Some(1));
        assert_eq!(
            prefs.read(Some("history"), "limit", PreferenceKind::Number, |raw| raw
                .parse::<f64>()
                .ok()),
            Some(2.0)
        );

        prefs.set_number("limit", 9.0);
        assert_eq!(prefs.count("limit"), Some(9));
        assert_eq!(
            prefs.read(Some("history"), "limit", PreferenceKind::Number, |raw| raw
                .parse::<f64>()
                .ok()),
            Some(2.0),
            "setting the extension-level value must not touch the command-level one"
        );
    }

    #[test]
    fn an_extension_only_reads_its_own_values() {
        let store = Arc::new(MemoryStore::default());
        store.set("dango.applications", None, "entries", "7");
        let prefs = preferences(store);
        assert_eq!(
            prefs.count("entries"),
            Some(500),
            "another extension's value is not visible, so the default stands"
        );
    }
}
