//! The files the user reads or edits in the config directory, and how Dango
//! writes one. Each file's path, schema path, and `#:schema` header come from
//! here, so every file follows the same layout.

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use super::location::config_dir;

const SCHEMA_DIR: &str = ".schemas";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UserFile {
    Config,
}

impl UserFile {
    pub const ALL: [UserFile; 1] = [UserFile::Config];

    /// The name without extension, shared by the file and its schema.
    pub fn name(self) -> &'static str {
        match self {
            UserFile::Config => "config",
        }
    }

    pub fn file_name(self) -> &'static str {
        match self {
            UserFile::Config => "config.toml",
        }
    }

    pub fn path(self) -> PathBuf {
        self.path_in(&config_dir())
    }

    pub fn path_in(self, dir: &Path) -> PathBuf {
        dir.join(self.file_name())
    }

    pub fn schema_path_in(self, dir: &Path) -> PathBuf {
        dir.join(SCHEMA_DIR).join(self.schema_file_name())
    }

    pub fn schema_file_name(self) -> String {
        format!("{}.schema.json", self.name())
    }

    /// The first line of a file Dango creates, relative so the config
    /// directory can move as a whole.
    pub fn header(self) -> String {
        format!("#:schema ./{SCHEMA_DIR}/{}\n", self.schema_file_name())
    }
}

/// Writes `text` to `path` through a temp file and a rename, so a reader never
/// sees half a file, and records it in `own_write` so the watcher can tell
/// Dango's write from the user's.
pub fn write_user_file(
    path: &Path,
    text: &str,
    own_write: &Mutex<Option<String>>,
) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut temp = path.as_os_str().to_owned();
    temp.push(".tmp");
    let temp = PathBuf::from(temp);
    std::fs::write(&temp, text)?;
    *own_write.lock().unwrap() = Some(text.to_string());
    std::fs::rename(&temp, path).inspect_err(|_| {
        *own_write.lock().unwrap() = None;
        let _ = std::fs::remove_file(&temp);
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_file_and_its_schema_share_a_name() {
        let dir = Path::new("/c");
        assert_eq!(UserFile::Config.path_in(dir), dir.join("config.toml"));
        assert_eq!(
            UserFile::Config.schema_path_in(dir),
            dir.join(".schemas").join("config.schema.json")
        );
        assert_eq!(
            UserFile::Config.header(),
            "#:schema ./.schemas/config.schema.json\n"
        );
    }

    #[test]
    fn a_write_replaces_the_file_and_leaves_no_temp_file() {
        let dir = std::env::temp_dir().join(format!("dango-files-{}", uuid::Uuid::new_v4()));
        let path = dir.join("nested").join("x.toml");
        let own = Mutex::new(None);
        write_user_file(&path, "a = 1\n", &own).unwrap();
        write_user_file(&path, "a = 2\n", &own).unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "a = 2\n");
        assert_eq!(own.lock().unwrap().as_deref(), Some("a = 2\n"));
        assert!(!dir.join("nested").join("x.toml.tmp").exists());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
