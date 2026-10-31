/// The only manifest version this build understands. Bumped when the shape
/// changes in a way older code cannot read.
pub const SUPPORTED_MANIFEST_VERSION: u32 = 1;

/// Prefix marking an icon as one of the launcher's own named icons rather than
/// a file an extension produced. `icon:lock` is drawn from the icon set;
/// anything without the prefix is a path to an image file.
pub const NAMED_ICON: &str = "icon:";

#[derive(Clone, Debug)]
pub struct Manifest {
    pub manifest_version: u32,
    pub id: String,
    pub name: String,
    /// Either `icon:<name>` for a named icon or a path to an image file.
    pub icon: Option<String>,
    pub commands: Vec<CommandDecl>,
    pub preferences: Vec<PreferenceDecl>,
    /// Whether the extension supplies the single per-keystroke root items
    /// provider. The provider itself is native code, not part of the manifest.
    pub root_items: bool,
    /// Whether the extension runs background services. Third-party extensions
    /// will never set this; only built-ins have it.
    pub services: bool,
}

#[derive(Clone, Debug)]
pub struct CommandDecl {
    pub id: String,
    pub title: String,
    pub mode: InvocationMode,
    pub subtitle: Option<String>,
    /// Either `icon:<name>` for a named icon or a path to an image file.
    pub icon: Option<String>,
    pub keywords: Vec<String>,
    pub alias: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InvocationMode {
    View,
    NoView,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PreferenceDecl {
    pub key: String,
    pub kind: PreferenceKind,
    pub default: Option<serde_json::Value>,
    /// One sentence for the user, shown wherever the preference is described.
    pub description: String,
    /// For a string preference that takes one of a fixed set, that set. Empty
    /// means any string.
    pub values: Vec<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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
    #[error("extension '{id}' declares preference '{key}' without a description")]
    UndescribedPreference { id: String, key: String },
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
        if let Some(preference) = self
            .preferences
            .iter()
            .find(|p| p.description.trim().is_empty())
        {
            return Err(ManifestError::UndescribedPreference {
                id: self.id.clone(),
                key: preference.key.clone(),
            });
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
    /// Whether a raw stored value is one this preference allows. Only a string
    /// preference with a declared set can refuse one.
    pub fn accepts(&self, raw: &str) -> bool {
        self.kind != PreferenceKind::String
            || self.values.is_empty()
            || self.values.iter().any(|value| value == raw)
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

    fn preference() -> PreferenceDecl {
        PreferenceDecl {
            key: "token".into(),
            kind: PreferenceKind::String,
            default: None,
            description: "A token.".into(),
            values: vec![],
        }
    }

    #[test]
    fn a_preference_without_a_description_is_rejected() {
        let mut m = manifest(1, vec![command("a")]);
        let mut pref = preference();
        pref.description = " ".into();
        m.preferences.push(pref);
        assert_eq!(
            m.validate(),
            Err(ManifestError::UndescribedPreference {
                id: "test".into(),
                key: "token".into()
            })
        );
    }

    #[test]
    fn a_declared_set_decides_what_is_accepted() {
        let mut pref = preference();
        assert!(pref.accepts("anything"));
        pref.values = vec!["paste".into(), "copy".into()];
        assert!(pref.accepts("copy"));
        assert!(!pref.accepts("speak"));
        pref.kind = PreferenceKind::Number;
        assert!(pref.accepts("speak"), "only strings have a set");
    }
}
