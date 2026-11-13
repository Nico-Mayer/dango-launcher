use std::path::{Path, PathBuf};
use std::time::SystemTime;

const EXTENSIONS: [&str; 4] = ["png", "jpg", "jpeg", "webp"];

/// The configured folders when there are any, the system's otherwise.
pub fn folders(configured: &[String], default: Option<PathBuf>) -> Vec<PathBuf> {
    if configured.is_empty() {
        return default.into_iter().collect();
    }
    configured
        .iter()
        .map(|folder| crate::platform::expand_home(folder))
        .collect()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Screenshot {
    pub path: PathBuf,
    pub modified: SystemTime,
}

impl Screenshot {
    pub fn name(&self) -> String {
        self.path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default()
    }
}

#[derive(Debug, Default)]
pub struct Scan {
    /// Newest first.
    pub screenshots: Vec<Screenshot>,
    pub any_folder: bool,
}

pub fn scan(folders: &[PathBuf]) -> Scan {
    let mut scan = Scan::default();
    for folder in folders {
        let Ok(entries) = std::fs::read_dir(folder) else {
            continue;
        };
        scan.any_folder = true;
        scan.screenshots.extend(
            entries
                .flatten()
                .map(|entry| entry.path())
                .filter(|path| is_screenshot(path))
                .filter_map(|path| {
                    let metadata = std::fs::metadata(&path).ok()?;
                    metadata.is_file().then_some(Screenshot {
                        modified: metadata.modified().ok()?,
                        path,
                    })
                }),
        );
    }
    scan.screenshots
        .sort_by_key(|screenshot| std::cmp::Reverse(screenshot.modified));
    scan
}

fn is_screenshot(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            EXTENSIONS
                .iter()
                .any(|known| extension.eq_ignore_ascii_case(known))
        })
}

#[cfg(test)]
pub(super) mod tests {
    use super::*;
    use std::time::Duration;

    pub fn file(folder: &Path, name: &str, age_seconds: u64) -> PathBuf {
        let path = folder.join(name);
        let file = std::fs::File::create(&path).unwrap();
        file.set_modified(SystemTime::now() - Duration::from_secs(age_seconds))
            .unwrap();
        path
    }

    fn names(scan: &Scan) -> Vec<String> {
        scan.screenshots.iter().map(Screenshot::name).collect()
    }

    #[test]
    fn configured_folders_replace_the_default_in_order() {
        let folders = folders(&["/a".into(), "/b".into()], Some(PathBuf::from("/default")));
        assert_eq!(folders, [PathBuf::from("/a"), PathBuf::from("/b")]);
    }

    #[test]
    fn an_empty_list_means_the_default() {
        assert_eq!(
            folders(&[], Some(PathBuf::from("/default"))),
            [PathBuf::from("/default")]
        );
        assert!(folders(&[], None).is_empty());
    }

    #[test]
    fn a_folder_under_home_is_expanded() {
        let home = dirs::home_dir().unwrap();
        assert_eq!(folders(&["~/Desktop".into()], None), [home.join("Desktop")]);
    }

    #[test]
    fn only_images_directly_inside_count() {
        let dir = tempfile::tempdir().unwrap();
        file(dir.path(), "shot.png", 0);
        file(dir.path(), "recording.mov", 0);
        file(dir.path(), "notes.pdf", 0);
        let nested = dir.path().join("older");
        std::fs::create_dir(&nested).unwrap();
        file(&nested, "nested.png", 0);
        assert_eq!(names(&scan(&[dir.path().into()])), ["shot.png"]);
    }

    #[test]
    fn upper_case_extensions_count() {
        let dir = tempfile::tempdir().unwrap();
        file(dir.path(), "Capture.PNG", 0);
        file(dir.path(), "photo.Jpeg", 1);
        file(dir.path(), "web.webp", 2);
        file(dir.path(), "old.jpg", 3);
        assert_eq!(
            names(&scan(&[dir.path().into()])),
            ["Capture.PNG", "photo.Jpeg", "web.webp", "old.jpg"]
        );
    }

    #[test]
    fn a_missing_folder_is_skipped() {
        let dir = tempfile::tempdir().unwrap();
        file(dir.path(), "shot.png", 0);
        let scan = scan(&[dir.path().join("gone"), dir.path().into()]);
        assert!(scan.any_folder);
        assert_eq!(names(&scan), ["shot.png"]);
    }

    #[test]
    fn no_folder_at_all_is_reported() {
        let dir = tempfile::tempdir().unwrap();
        let scan = scan(&[dir.path().join("gone")]);
        assert!(!scan.any_folder);
        assert!(scan.screenshots.is_empty());
    }

    #[test]
    fn two_folders_merge_newest_first() {
        let desktop = tempfile::tempdir().unwrap();
        let pictures = tempfile::tempdir().unwrap();
        file(desktop.path(), "oldest.png", 30);
        file(pictures.path(), "newest.png", 0);
        file(desktop.path(), "middle.png", 10);
        assert_eq!(
            names(&scan(&[desktop.path().into(), pictures.path().into()])),
            ["newest.png", "middle.png", "oldest.png"]
        );
    }
}
