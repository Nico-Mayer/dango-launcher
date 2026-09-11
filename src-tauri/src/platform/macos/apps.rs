//! The macOS application indexer: walks the applications directories for
//! bundles, reads their declared display names, renders icons through the
//! workspace, and launches through it so an already-running instance is
//! activated rather than duplicated.

use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::{Path, PathBuf};

use objc2_app_kit::{NSWorkspace, NSWorkspaceOpenConfiguration};
use objc2_foundation::{NSArray, NSFileManager, NSString, NSURL};

use crate::extensions::applications::{AppIndexer, IconRgba, IndexedApp, LaunchError};

pub struct MacAppIndexer;

impl AppIndexer for MacAppIndexer {
    fn index(&self) -> Vec<IndexedApp> {
        let mut apps = Vec::new();
        let mut seen = std::collections::HashSet::new();
        for root in roots() {
            collect(&root, &mut apps, &mut seen);
        }
        apps
    }

    /// Only the applications directories themselves are fingerprinted, because
    /// that is where an install or a drag lands. A bundle appearing in a
    /// category subfolder is picked up by the periodic sweep instead.
    fn fingerprint(&self) -> Option<u64> {
        let mut hasher = DefaultHasher::new();
        for root in roots() {
            if let Ok(modified) = std::fs::metadata(&root).and_then(|m| m.modified()) {
                root.hash(&mut hasher);
                modified.hash(&mut hasher);
            }
        }
        Some(hasher.finish())
    }

    fn launch(&self, app: &IndexedApp) -> Result<(), LaunchError> {
        let path = bundle_path(app);
        if !Path::new(path).exists() {
            return Err(LaunchError::NotFound);
        }
        let configuration = NSWorkspaceOpenConfiguration::new();
        configuration.setActivates(true);
        // The workspace reuses a running instance unless asked otherwise, which
        // is what brings an already-open application to the foreground.
        configuration.setCreatesNewApplicationInstance(false);
        NSWorkspace::sharedWorkspace().openApplicationAtURL_configuration_completionHandler(
            &file_url(path),
            &configuration,
            None,
        );
        Ok(())
    }

    fn reveal(&self, app: &IndexedApp) -> std::io::Result<()> {
        let path = bundle_path(app);
        if !Path::new(path).exists() {
            return Err(std::io::Error::other("this application no longer exists"));
        }
        let urls = NSArray::from_retained_slice(&[file_url(path)]);
        NSWorkspace::sharedWorkspace().activateFileViewerSelectingURLs(&urls);
        Ok(())
    }

    fn icon(&self, app: &IndexedApp, size: u32) -> Option<IconRgba> {
        super::icon_for(bundle_path(app), size)
    }
}

/// The applications directories named by the spec. Their immediate
/// subdirectories are searched too, which is what makes Utilities appear.
fn roots() -> Vec<PathBuf> {
    let mut roots = vec![
        PathBuf::from("/Applications"),
        PathBuf::from("/System/Applications"),
    ];
    if let Some(home) = std::env::var_os("HOME") {
        roots.push(PathBuf::from(home).join("Applications"));
    }
    roots
}

fn collect(dir: &Path, apps: &mut Vec<IndexedApp>, seen: &mut std::collections::HashSet<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if is_bundle(&path) {
            if seen.insert(path.clone()) {
                apps.push(indexed(&path));
            }
        } else if entry.file_type().is_ok_and(|t| t.is_dir()) {
            collect_bundles(&path, apps, seen);
        }
    }
}

/// One level deeper, where only bundles count. Descending further would walk
/// into the bundles themselves, which are directories.
fn collect_bundles(
    dir: &Path,
    apps: &mut Vec<IndexedApp>,
    seen: &mut std::collections::HashSet<PathBuf>,
) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if is_bundle(&path) && seen.insert(path.clone()) {
            apps.push(indexed(&path));
        }
    }
}

fn is_bundle(path: &Path) -> bool {
    path.extension().is_some_and(|e| e == "app") && path.is_dir()
}

fn indexed(path: &Path) -> IndexedApp {
    let location = path.to_string_lossy().into_owned();
    IndexedApp {
        name: display_name(path),
        id: location.clone(),
        target: Some(location),
    }
}

/// The name the system itself shows. Reading `CFBundleDisplayName` directly
/// would be wrong: macOS honours it only as a localisation of the bundle
/// filename, so Find My is named from its declaration while Visual Studio Code,
/// which declares the unrelated "Code", keeps its filename. Asking the file
/// manager gets both cases right for free.
fn display_name(path: &Path) -> String {
    let stem = path
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    let shown = NSFileManager::defaultManager()
        .displayNameAtPath(&NSString::from_str(&path.to_string_lossy()))
        .to_string();
    // The extension is included when the user asks Finder to show all of them.
    let shown = shown.strip_suffix(".app").unwrap_or(&shown);
    if shown.is_empty() {
        stem
    } else {
        shown.to_owned()
    }
}

fn bundle_path(app: &IndexedApp) -> &str {
    app.target.as_deref().unwrap_or(&app.id)
}

fn file_url(path: &str) -> objc2::rc::Retained<NSURL> {
    NSURL::fileURLWithPath(&NSString::from_str(path))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A tree shaped like an applications directory: a bundle at the top, one
    /// in a category folder, one buried too deep to count, and a file that only
    /// looks like a bundle.
    fn tree() -> PathBuf {
        let root = std::env::temp_dir().join(format!("dango-apps-{}", uuid::Uuid::new_v4()));
        for dir in [
            "Alpha.app/Contents",
            "Utilities/Beta.app/Contents",
            "Utilities/Deeper/Gamma.app/Contents",
        ] {
            std::fs::create_dir_all(root.join(dir)).unwrap();
        }
        std::fs::write(root.join("Notes.app"), b"not a bundle").unwrap();
        root
    }

    fn names_in(root: &Path) -> Vec<String> {
        let mut apps = Vec::new();
        collect(root, &mut apps, &mut std::collections::HashSet::new());
        let mut names: Vec<_> = apps.into_iter().map(|a| a.name).collect();
        names.sort();
        names
    }

    #[test]
    fn the_spec_directories_are_searched() {
        let roots = roots();
        assert!(roots.contains(&PathBuf::from("/Applications")));
        assert!(roots.contains(&PathBuf::from("/System/Applications")));
        assert!(roots
            .iter()
            .any(|r| r.ends_with("Applications") && r.starts_with(std::env::var("HOME").unwrap())));
    }

    #[test]
    fn bundles_at_the_top_and_one_level_down_are_indexed() {
        let root = tree();
        assert_eq!(names_in(&root), ["Alpha", "Beta"]);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn a_bundle_buried_deeper_than_one_level_is_not() {
        let root = tree();
        assert!(!names_in(&root).contains(&"Gamma".to_string()));
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn a_file_named_like_a_bundle_is_not_one() {
        let root = tree();
        assert!(!is_bundle(&root.join("Notes.app")));
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn an_application_is_indexed_by_its_own_path() {
        let root = tree();
        let bundle = root.join("Alpha.app");
        let app = indexed(&bundle);
        assert_eq!(app.id, bundle.to_string_lossy());
        assert_eq!(app.target.as_deref(), Some(&*app.id));
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn a_bundle_without_a_declared_name_keeps_its_filename() {
        let root = tree();
        assert_eq!(display_name(&root.join("Alpha.app")), "Alpha");
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn launching_a_missing_bundle_reports_it_as_gone() {
        let app = indexed(Path::new("/Applications/Dango Does Not Exist.app"));
        assert!(matches!(
            MacAppIndexer.launch(&app),
            Err(LaunchError::NotFound)
        ));
    }

    #[test]
    fn the_fingerprint_is_stable_while_nothing_changes() {
        assert!(MacAppIndexer.fingerprint().is_some());
        assert_eq!(MacAppIndexer.fingerprint(), MacAppIndexer.fingerprint());
    }

    /// The installed applications and the window server are not CI's to give,
    /// so these run on a real desktop: `cargo test -- --ignored`.
    mod live {
        use super::*;

        #[test]
        #[ignore]
        fn a_localised_display_name_wins_over_the_filename() {
            assert_eq!(
                display_name(Path::new("/System/Applications/FindMy.app")),
                "Find My"
            );
        }

        #[test]
        #[ignore]
        fn a_declaration_that_is_not_a_localisation_is_ignored() {
            // Visual Studio Code declares CFBundleDisplayName "Code", which
            // macOS itself disregards because it is not a translation of the
            // filename.
            assert_eq!(
                display_name(Path::new("/Applications/Visual Studio Code.app")),
                "Visual Studio Code"
            );
        }

        #[test]
        #[ignore]
        fn the_installed_applications_are_found() {
            let apps = MacAppIndexer.index();
            assert!(apps
                .iter()
                .any(|a| a.id == "/System/Applications/Utilities/Terminal.app"));
        }

        /// Calculator is the subject because it holds no documents, so a
        /// stray instance costs nothing. A machine already running it is left
        /// alone rather than having its process counted or killed.
        #[test]
        #[ignore]
        fn launching_twice_does_not_start_a_second_instance() {
            fn instances() -> usize {
                let output = std::process::Command::new("pgrep")
                    .args(["-x", "Calculator"])
                    .output()
                    .unwrap();
                String::from_utf8_lossy(&output.stdout).lines().count()
            }
            // A cold launch through LaunchServices can take several seconds.
            fn settle() -> usize {
                for _ in 0..40 {
                    std::thread::sleep(std::time::Duration::from_millis(250));
                    if instances() > 0 {
                        break;
                    }
                }
                std::thread::sleep(std::time::Duration::from_secs(2));
                instances()
            }
            assert_eq!(instances(), 0, "Calculator must not be running already");

            let app = indexed(Path::new("/System/Applications/Calculator.app"));
            MacAppIndexer.launch(&app).unwrap();
            assert_eq!(settle(), 1);

            MacAppIndexer.launch(&app).unwrap();
            let after = settle();
            let _ = std::process::Command::new("pkill")
                .args(["-x", "Calculator"])
                .status();
            assert_eq!(after, 1, "the running instance must be reused");
        }

        #[test]
        #[ignore]
        fn every_installed_application_has_an_icon() {
            for app in MacAppIndexer.index() {
                let icon = MacAppIndexer
                    .icon(&app, 64)
                    .unwrap_or_else(|| panic!("no icon for {}", app.name));
                assert_eq!((icon.width, icon.height), (64, 64));
                assert!(
                    icon.rgba.iter().any(|&byte| byte != 0),
                    "blank icon for {}",
                    app.name
                );
            }
        }
    }
}
