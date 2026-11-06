//! `manifest.json`: the file form of an extension's `Manifest`.

use std::path::Path;
use std::time::Duration;

use serde::Deserialize;

use crate::extension::manifest::{
    CommandDecl, InvocationMode, Manifest, ManifestError, NAMED_ICON,
};

/// A manifest read from a folder, with what only JavaScript extensions have.
#[derive(Debug)]
pub struct JsManifest {
    pub manifest: Manifest,
    /// How often root items are built again while the extension is enabled.
    pub refresh: Option<Duration>,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ManifestFileError {
    #[error("manifest.json isn't valid: {0}")]
    Json(String),
    #[error("manifest.json names the extension '{id}', but its folder is '{folder}'")]
    IdMismatch { id: String, folder: String },
    #[error("manifest.json declares services, which only built-in extensions can have")]
    Services,
    #[error(transparent)]
    Invalid(#[from] ManifestError),
}

#[derive(Deserialize)]
#[cfg_attr(test, derive(ts_rs::TS), ts(rename = "Manifest", optional_fields))]
#[serde(rename_all = "camelCase")]
pub(crate) struct ManifestFile {
    manifest_version: u32,
    id: String,
    name: String,
    icon: Option<String>,
    #[serde(default)]
    #[cfg_attr(test, ts(optional, as = "Option<_>"))]
    commands: Vec<CommandFile>,
    root_items: Option<RootItemsFile>,
    #[cfg_attr(test, ts(skip))]
    services: Option<serde_json::Value>,
}

#[derive(Deserialize)]
#[cfg_attr(
    test,
    derive(ts_rs::TS),
    ts(rename = "ManifestCommand", optional_fields)
)]
#[serde(rename_all = "camelCase")]
struct CommandFile {
    id: String,
    title: String,
    mode: ModeFile,
    icon: Option<String>,
    #[serde(default)]
    #[cfg_attr(test, ts(optional, as = "Option<_>"))]
    keywords: Vec<String>,
}

#[derive(Deserialize)]
#[cfg_attr(test, derive(ts_rs::TS), ts(rename = "CommandMode"))]
#[serde(rename_all = "kebab-case")]
enum ModeFile {
    View,
    NoView,
}

#[derive(Deserialize)]
#[cfg_attr(
    test,
    derive(ts_rs::TS),
    ts(rename = "ManifestRootItems", optional_fields)
)]
#[serde(rename_all = "camelCase")]
struct RootItemsFile {
    refresh_minutes: Option<u64>,
}

#[cfg(test)]
pub(crate) fn type_decls(cfg: &ts_rs::Config) -> Vec<String> {
    use ts_rs::TS;
    vec![
        ManifestFile::decl(cfg),
        CommandFile::decl(cfg),
        ModeFile::decl(cfg),
        RootItemsFile::decl(cfg),
    ]
}

/// Reads the manifest of the extension in `folder`, whose name is its id.
pub fn parse(text: &str, folder: &Path) -> Result<JsManifest, ManifestFileError> {
    let file: ManifestFile =
        serde_json::from_str(text).map_err(|error| ManifestFileError::Json(error.to_string()))?;
    let folder_name = folder
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    if file.id != folder_name {
        return Err(ManifestFileError::IdMismatch {
            id: file.id,
            folder: folder_name,
        });
    }
    if file.services.is_some() {
        return Err(ManifestFileError::Services);
    }
    let icon = |icon: Option<String>| icon.map(|icon| resolve_icon(icon, folder));
    let refresh = file
        .root_items
        .as_ref()
        .and_then(|root| root.refresh_minutes)
        .map(|minutes| Duration::from_secs(minutes * 60));
    let manifest = Manifest {
        manifest_version: file.manifest_version,
        id: file.id,
        name: file.name,
        icon: icon(file.icon),
        commands: file
            .commands
            .into_iter()
            .map(|command| CommandDecl {
                id: command.id,
                title: command.title,
                mode: match command.mode {
                    ModeFile::View => InvocationMode::View,
                    ModeFile::NoView => InvocationMode::NoView,
                },
                icon: icon(command.icon),
                keywords: command.keywords,
                alias: None,
                repeats: false,
            })
            .collect(),
        root_items: file.root_items.is_some(),
        services: false,
    };
    manifest.validate()?;
    Ok(JsManifest { manifest, refresh })
}

/// A named icon stays as it is; a file path is taken from the extension's
/// folder unless it is absolute.
pub fn resolve_icon(icon: String, folder: &Path) -> String {
    if icon.starts_with(NAMED_ICON) || Path::new(&icon).is_absolute() {
        return icon;
    }
    folder.join(icon).to_string_lossy().into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn folder() -> &'static Path {
        Path::new("/ext/gh-prs")
    }

    const VALID: &str = r#"{
        "manifestVersion": 1,
        "id": "gh-prs",
        "name": "GitHub PRs",
        "commands": [{ "id": "my-prs", "title": "My Pull Requests", "mode": "view" }]
    }"#;

    #[test]
    fn a_valid_manifest_reads() {
        let read = parse(VALID, folder()).unwrap();

        assert_eq!(read.manifest.name, "GitHub PRs");
        assert_eq!(read.manifest.commands[0].id, "my-prs");
        assert_eq!(read.manifest.commands[0].mode, InvocationMode::View);
        assert!(!read.manifest.root_items);
        assert_eq!(read.refresh, None);
    }

    #[test]
    fn bad_json_is_reported() {
        assert!(matches!(
            parse("{ nope", folder()),
            Err(ManifestFileError::Json(_))
        ));
    }

    #[test]
    fn the_id_must_match_the_folder() {
        let text = VALID.replace(r#""id": "gh-prs""#, r#""id": "github""#);

        assert_eq!(
            parse(&text, folder()).unwrap_err(),
            ManifestFileError::IdMismatch {
                id: "github".into(),
                folder: "gh-prs".into()
            }
        );
    }

    #[test]
    fn services_are_refused() {
        let text = VALID.replace(r#""name""#, r#""services": ["poll"], "name""#);

        assert_eq!(
            parse(&text, folder()).unwrap_err(),
            ManifestFileError::Services
        );
    }

    #[test]
    fn a_relative_icon_is_read_from_the_folder() {
        let text = VALID.replace(r#""name""#, r#""icon": "icon.png", "name""#);

        let read = parse(&text, folder()).unwrap();

        assert_eq!(
            read.manifest.icon.map(std::path::PathBuf::from),
            Some(folder().join("icon.png"))
        );
    }

    #[test]
    fn a_named_icon_stays_named() {
        let text = VALID.replace(r#""name""#, r#""icon": "icon:github", "name""#);

        assert_eq!(
            parse(&text, folder()).unwrap().manifest.icon.as_deref(),
            Some("icon:github")
        );
    }

    #[test]
    fn root_items_carry_their_refresh() {
        let text = VALID.replace(
            r#""name""#,
            r#""rootItems": { "refreshMinutes": 30 }, "name""#,
        );

        let read = parse(&text, folder()).unwrap();

        assert!(read.manifest.root_items);
        assert_eq!(read.refresh, Some(Duration::from_secs(1800)));
    }

    #[test]
    fn a_newer_manifest_version_is_refused() {
        let text = VALID.replace(r#""manifestVersion": 1"#, r#""manifestVersion": 2"#);

        assert_eq!(
            parse(&text, folder()).unwrap_err(),
            ManifestFileError::Invalid(ManifestError::UnsupportedVersion { found: 2 })
        );
    }
}
