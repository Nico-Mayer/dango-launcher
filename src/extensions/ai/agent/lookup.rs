//! Finding the program a request runs.
//!
//! A tray app does not get the PATH a shell builds: `mise activate`, a shell
//! profile, or an installer that only edits the profile all add folders the
//! launcher never sees. So the search is the process PATH plus the folders the
//! platform knows programs land in, and the same list becomes the child's PATH,
//! so a wrapper script finds `node` and a mise shim finds `mise`.

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use crate::extensions::ai::provider::AiError;

/// Where a run starts: the program, and the PATH to give it.
#[derive(Debug, PartialEq, Eq)]
pub struct Located {
    pub program: PathBuf,
    pub path: OsString,
}

/// The process PATH first, so the search only adds folders, then `extra`, with
/// repeats and folders that cannot sit in a PATH dropped.
pub fn search_dirs(process_path: Option<&std::ffi::OsStr>, extra: Vec<PathBuf>) -> Vec<PathBuf> {
    let mut dirs: Vec<PathBuf> = Vec::new();
    let from_process = process_path
        .map(|path| std::env::split_paths(path).collect::<Vec<_>>())
        .unwrap_or_default();
    for dir in from_process.into_iter().chain(extra) {
        let joinable = std::env::join_paths([&dir]).is_ok();
        if joinable && !dir.as_os_str().is_empty() && !dirs.contains(&dir) {
            dirs.push(dir);
        }
    }
    dirs
}

pub fn locate(
    program: &str,
    configured: Option<&Path>,
    dirs: &[PathBuf],
    cwd: &Path,
) -> Result<Located, AiError> {
    let path = std::env::join_paths(dirs).unwrap_or_default();
    if let Some(configured) = configured {
        return match configured.is_absolute() && configured.is_file() {
            true => Ok(Located {
                program: configured.to_path_buf(),
                path,
            }),
            false => Err(AiError::PathMissing {
                path: configured.display().to_string(),
            }),
        };
    }
    match which::which_in(program, Some(&path), cwd) {
        Ok(found) => Ok(Located {
            program: found,
            path,
        }),
        Err(_) => Err(AiError::ProgramMissing {
            program: program.to_string(),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn program_file(dir: &Path, name: &str) -> PathBuf {
        let file = dir.join(if cfg!(windows) {
            format!("{name}.exe")
        } else {
            name.to_string()
        });
        std::fs::write(&file, b"").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
        file
    }

    #[test]
    fn a_configured_path_wins_over_the_search() {
        let dir = tempfile::tempdir().unwrap();
        let elsewhere = tempfile::tempdir().unwrap();
        program_file(dir.path(), "claude");
        let configured = program_file(elsewhere.path(), "my-claude");

        let found = locate(
            "claude",
            Some(&configured),
            &[dir.path().to_path_buf()],
            dir.path(),
        )
        .unwrap();
        assert_eq!(found.program, configured);
    }

    #[test]
    fn a_relative_or_missing_configured_path_is_reported() {
        let dir = tempfile::tempdir().unwrap();
        for configured in [PathBuf::from("claude"), dir.path().join("not-there.exe")] {
            let error = locate("claude", Some(&configured), &[], dir.path()).unwrap_err();
            assert_eq!(
                error.to_string(),
                format!(
                    "Couldn't find {}. Check the path in your config file.",
                    configured.display()
                )
            );
        }
    }

    #[test]
    fn the_first_folder_holding_the_program_wins() {
        let first = tempfile::tempdir().unwrap();
        let second = tempfile::tempdir().unwrap();
        program_file(second.path(), "pi");
        let expected = program_file(first.path(), "pi");

        let dirs = [first.path().to_path_buf(), second.path().to_path_buf()];
        let found = locate("pi", None, &dirs, first.path()).unwrap();
        assert_eq!(found.program, expected);
        assert_eq!(std::env::split_paths(&found.path).collect::<Vec<_>>(), dirs);
    }

    #[test]
    fn a_program_installed_after_a_miss_is_found_next_time() {
        let dir = tempfile::tempdir().unwrap();
        let dirs = [dir.path().to_path_buf()];
        let error = locate("claude", None, &dirs, dir.path()).unwrap_err();
        assert_eq!(
            error.to_string(),
            "Couldn't find claude. Install it or set its path in your config file."
        );

        program_file(dir.path(), "claude");
        assert!(locate("claude", None, &dirs, dir.path()).is_ok());
    }

    #[test]
    fn the_search_keeps_the_process_path_first_and_drops_repeats() {
        let a = PathBuf::from(if cfg!(windows) { r"C:\a" } else { "/a" });
        let b = PathBuf::from(if cfg!(windows) { r"C:\b" } else { "/b" });
        let c = PathBuf::from(if cfg!(windows) { r"C:\c" } else { "/c" });
        let process = std::env::join_paths([&a, &b]).unwrap();

        let dirs = search_dirs(Some(&process), vec![b.clone(), c.clone(), a.clone()]);
        assert_eq!(dirs, [a, b, c]);
        assert_eq!(search_dirs(None, Vec::new()), Vec::<PathBuf>::new());
    }
}
