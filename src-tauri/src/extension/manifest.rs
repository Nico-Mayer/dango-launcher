use serde::{Deserialize, Serialize};

/// The only manifest version this build understands. Bumped when the shape
/// changes in a way older code cannot read.
pub const SUPPORTED_MANIFEST_VERSION: u32 = 1;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Manifest {
    pub manifest_version: u32,
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub icon: Option<String>,
    #[serde(default)]
    pub commands: Vec<CommandDecl>,
    #[serde(default)]
    pub preferences: Vec<PreferenceDecl>,
    /// Whether the extension supplies the single per-keystroke root items
    /// provider. The provider itself is native code, not part of the manifest.
    #[serde(default)]
    pub root_items: bool,
    /// Whether the extension runs background services. Third-party extensions
    /// will never set this; only built-ins have it.
    #[serde(default)]
    pub services: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CommandDecl {
    pub id: String,
    pub title: String,
    pub mode: InvocationMode,
    #[serde(default)]
    pub subtitle: Option<String>,
    #[serde(default)]
    pub icon: Option<String>,
    #[serde(default)]
    pub keywords: Vec<String>,
    #[serde(default)]
    pub alias: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum InvocationMode {
    View,
    NoView,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PreferenceDecl {
    /// `None` scopes the preference to the whole extension; `Some(command_id)`
    /// scopes it to one command.
    #[serde(default)]
    pub command_id: Option<String>,
    pub key: String,
    #[serde(rename = "type")]
    pub kind: PreferenceKind,
    #[serde(default)]
    pub default: Option<serde_json::Value>,
    #[serde(default)]
    pub required: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PreferenceKind {
    String,
    Number,
    Boolean,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ManifestError {
    #[error("manifest version {found} is newer than the supported {SUPPORTED_MANIFEST_VERSION}")]
    UnsupportedVersion { found: u32 },
    #[error("extension '{id}' contributes nothing")]
    NoContributions { id: String },
    #[error("extension '{id}' declares two commands with id '{command_id}'")]
    DuplicateCommand { id: String, command_id: String },
}

impl Manifest {
    /// Rejects a manifest this build cannot honour. A newer version is refused
    /// rather than half-read, and an extension that contributes nothing is
    /// malformed. Duplicate command ids within one extension are caught here;
    /// collisions between extensions are the registry's job.
    pub fn validate(&self) -> Result<(), ManifestError> {
        if self.manifest_version > SUPPORTED_MANIFEST_VERSION {
            return Err(ManifestError::UnsupportedVersion {
                found: self.manifest_version,
            });
        }
        if !self.contributes_anything() {
            return Err(ManifestError::NoContributions {
                id: self.id.clone(),
            });
        }
        for (i, command) in self.commands.iter().enumerate() {
            if self.commands[..i].iter().any(|c| c.id == command.id) {
                return Err(ManifestError::DuplicateCommand {
                    id: self.id.clone(),
                    command_id: command.id.clone(),
                });
            }
        }
        Ok(())
    }

    fn contributes_anything(&self) -> bool {
        !self.commands.is_empty()
            || self.root_items
            || self.services
            || !self.preferences.is_empty()
    }
}

impl CommandDecl {
    /// Extension id joined to command id. Unique across the whole registry.
    pub fn qualified_id(&self, extension_id: &str) -> String {
        format!("{extension_id}.{}", self.id)
    }
}

impl PreferenceDecl {
    /// The stored value if the user set one, otherwise the declared default.
    pub fn effective<'a>(
        &'a self,
        stored: Option<&'a serde_json::Value>,
    ) -> Option<&'a serde_json::Value> {
        stored.or(self.default.as_ref())
    }

    /// A required preference with neither a stored value nor a default. A
    /// command with one of these must prompt instead of running.
    pub fn is_unsatisfied(&self, stored: Option<&serde_json::Value>) -> bool {
        self.required && self.effective(stored).is_none()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn command(id: &str) -> CommandDecl {
        CommandDecl {
            id: id.into(),
            title: id.into(),
            mode: InvocationMode::View,
            subtitle: None,
            icon: None,
            keywords: vec![],
            alias: None,
        }
    }

    fn manifest(version: u32, commands: Vec<CommandDecl>) -> Manifest {
        Manifest {
            manifest_version: version,
            id: "test".into(),
            name: "Test".into(),
            icon: None,
            commands,
            preferences: vec![],
            root_items: false,
            services: false,
        }
    }

    #[test]
    fn supported_version_with_a_command_is_valid() {
        assert!(manifest(1, vec![command("a")]).validate().is_ok());
    }

    #[test]
    fn newer_version_is_rejected() {
        assert_eq!(
            manifest(2, vec![command("a")]).validate(),
            Err(ManifestError::UnsupportedVersion { found: 2 })
        );
    }

    #[test]
    fn empty_contributions_are_rejected() {
        assert_eq!(
            manifest(1, vec![]).validate(),
            Err(ManifestError::NoContributions { id: "test".into() })
        );
    }

    #[test]
    fn a_root_items_provider_alone_is_enough() {
        let mut m = manifest(1, vec![]);
        m.root_items = true;
        assert!(m.validate().is_ok());
    }

    #[test]
    fn duplicate_command_ids_are_rejected() {
        assert_eq!(
            manifest(1, vec![command("a"), command("a")]).validate(),
            Err(ManifestError::DuplicateCommand {
                id: "test".into(),
                command_id: "a".into()
            })
        );
    }

    #[test]
    fn qualified_id_joins_extension_and_command() {
        assert_eq!(command("open").qualified_id("apps"), "apps.open");
    }

    fn preference(default: Option<serde_json::Value>, required: bool) -> PreferenceDecl {
        PreferenceDecl {
            command_id: None,
            key: "token".into(),
            kind: PreferenceKind::String,
            default,
            required,
        }
    }

    #[test]
    fn default_is_returned_when_no_value_is_stored() {
        let pref = preference(Some(serde_json::json!("fallback")), false);
        assert_eq!(pref.effective(None), Some(&serde_json::json!("fallback")));
    }

    #[test]
    fn stored_value_overrides_the_default() {
        let pref = preference(Some(serde_json::json!("fallback")), false);
        let stored = serde_json::json!("set");
        assert_eq!(pref.effective(Some(&stored)), Some(&stored));
    }

    #[test]
    fn required_without_value_or_default_is_unsatisfied() {
        let pref = preference(None, true);
        assert!(pref.is_unsatisfied(None));
        assert!(!pref.is_unsatisfied(Some(&serde_json::json!("set"))));
    }
}
