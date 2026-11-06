//! Finding extensions in the extensions folder.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::extension::{ExtensionHost, LoadReport};

use super::extension::JsExtension;
use super::manifest;

pub const MANIFEST_FILE: &str = "manifest.json";
pub const MAIN_FILE: &str = "main.js";

/// The extensions folder inside the config directory.
pub fn extensions_dir(config_dir: &Path) -> PathBuf {
    config_dir.join("extensions")
}

#[derive(Debug, PartialEq, Eq)]
pub struct LoadFailure {
    pub id: String,
    pub reason: String,
}

/// Reads every extension folder in `dir`, in name order. A missing `dir` is no
/// extensions; a folder with neither file isn't an extension and is skipped.
pub fn scan(dir: &Path) -> (Vec<JsExtension>, Vec<LoadFailure>) {
    let mut folders: Vec<PathBuf> = match std::fs::read_dir(dir) {
        Ok(entries) => entries
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| path.is_dir())
            .collect(),
        Err(_) => Vec::new(),
    };
    folders.sort();
    let mut loaded = Vec::new();
    let mut failed = Vec::new();
    for folder in folders {
        match load(&folder) {
            Some(Ok(extension)) => loaded.push(extension),
            Some(Err(reason)) => failed.push(LoadFailure {
                id: folder_id(&folder),
                reason,
            }),
            None => {}
        }
    }
    (loaded, failed)
}

/// Registers every extension in `dir`. A failure is logged with its reason and
/// lands in the report, so the tray can name the extension.
pub fn register_all(host: &mut ExtensionHost, dir: &Path) -> LoadReport {
    let (loaded, failed) = scan(dir);
    let mut report = LoadReport::default();
    for failure in failed {
        fail(&mut report, failure.id, failure.reason);
    }
    for extension in loaded {
        let id = extension.manifest_id().to_owned();
        match host.register(Arc::new(extension)) {
            Ok(registered) => report.merge(registered),
            Err(error) => fail(&mut report, id, error.to_string()),
        }
    }
    report
}

fn fail(report: &mut LoadReport, id: String, reason: String) {
    crate::log::append(&format!("extension {id} didn't load: {reason}"));
    report.load_errors.push((id, reason));
}

/// `None` when the folder holds neither file and so isn't an extension.
pub fn load(folder: &Path) -> Option<Result<JsExtension, String>> {
    let has_manifest = folder.join(MANIFEST_FILE).is_file();
    let has_main = folder.join(MAIN_FILE).is_file();
    match (has_manifest, has_main) {
        (false, false) => None,
        (true, false) => Some(Err(format!("{MAIN_FILE} is missing"))),
        (false, true) => Some(Err(format!("{MANIFEST_FILE} is missing"))),
        (true, true) => Some(read(folder)),
    }
}

fn read(folder: &Path) -> Result<JsExtension, String> {
    let text = std::fs::read_to_string(folder.join(MANIFEST_FILE))
        .map_err(|error| format!("{MANIFEST_FILE} can't be read: {error}"))?;
    let read = manifest::parse(&text, folder).map_err(|error| error.to_string())?;
    Ok(JsExtension::new(read, folder.to_path_buf()))
}

pub fn folder_id(folder: &Path) -> String {
    folder
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default()
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::extension::Extension;

    pub fn write_extension(dir: &Path, id: &str, main: &str) -> PathBuf {
        let folder = dir.join(id);
        std::fs::create_dir_all(&folder).unwrap();
        std::fs::write(
            folder.join(MANIFEST_FILE),
            format!(
                r#"{{ "manifestVersion": 1, "id": "{id}", "name": "Test",
                     "commands": [{{ "id": "run", "title": "Run It", "mode": "view" }}] }}"#
            ),
        )
        .unwrap();
        std::fs::write(folder.join(MAIN_FILE), main).unwrap();
        folder
    }

    fn temp() -> PathBuf {
        let dir = std::env::temp_dir().join(format!("dango-js-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn a_missing_folder_is_no_extensions() {
        let (loaded, failed) = scan(&temp().join("nope"));

        assert!(loaded.is_empty());
        assert!(failed.is_empty());
    }

    #[test]
    fn a_complete_folder_loads() {
        let dir = temp();
        write_extension(&dir, "gh-prs", "export function run() {}");

        let (loaded, failed) = scan(&dir);

        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].manifest().id, "gh-prs");
        assert!(failed.is_empty());
    }

    #[test]
    fn a_folder_without_main_js_fails() {
        let dir = temp();
        let folder = write_extension(&dir, "gh-prs", "");
        std::fs::remove_file(folder.join(MAIN_FILE)).unwrap();

        let (loaded, failed) = scan(&dir);

        assert!(loaded.is_empty());
        assert_eq!(
            failed,
            vec![LoadFailure {
                id: "gh-prs".into(),
                reason: "main.js is missing".into()
            }]
        );
    }

    struct AllEnabled;

    impl crate::extension::EnabledStore for AllEnabled {
        fn is_enabled(&self, _extension_id: &str) -> bool {
            true
        }
    }

    #[test]
    fn registered_extensions_put_their_commands_in_the_registry() {
        let dir = temp();
        write_extension(&dir, "gh-prs", "export function run() {}");
        std::fs::create_dir_all(dir.join("broken")).unwrap();
        std::fs::write(dir.join("broken").join(MANIFEST_FILE), "{}").unwrap();
        let mut host = ExtensionHost::new(Arc::new(AllEnabled));

        let report = register_all(&mut host, &dir);

        assert!(host.registry().get("gh-prs.run").is_some());
        assert_eq!(report.load_errors.len(), 1);
        assert_eq!(report.load_errors[0].0, "broken");
    }

    #[test]
    fn a_folder_that_is_not_an_extension_is_ignored() {
        let dir = temp();
        std::fs::create_dir_all(dir.join(".git")).unwrap();

        let (loaded, failed) = scan(&dir);

        assert!(loaded.is_empty());
        assert!(failed.is_empty());
    }
}
