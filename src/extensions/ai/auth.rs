//! Provider keys, read from `auth.json` in the config directory.
//!
//! Plain text, deliberately: one user with two machines, and a file that can be
//! copied. It is its own file so `config.json` stays committable, and Dango only
//! ever reads it.
//!
//! Two rules hold everything here together. The file is read per request rather
//! than cached, so a key added while Dango runs works immediately and none sits
//! in memory between requests. And nothing derived from the file's contents ever
//! reaches an error: the JSON is parsed to a value first and only position
//! information from a syntax error is kept, because a serde error over a typed
//! struct can quote the value it choked on.

use std::path::{Path, PathBuf};

use serde_json::Value;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum AuthError {
    #[error("auth.json is not valid JSON, at line {line}")]
    Parse { line: usize },
    #[error("auth.json could not be read")]
    Read,
}

/// Where keys come from. A trait so a command can be tested without a file, and
/// so the keychain can replace the file later without touching a caller.
pub trait Keys: Send + Sync {
    fn key(&self, provider: &str) -> Result<Option<String>, AuthError>;
}

pub struct AuthFile {
    path: PathBuf,
}

impl AuthFile {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }
}

impl Keys for AuthFile {
    fn key(&self, provider: &str) -> Result<Option<String>, AuthError> {
        let Some(text) = read(&self.path)? else {
            return Ok(None);
        };
        restrict_to_owner(&self.path);
        let value: Value =
            serde_json::from_str(&text).map_err(|error| AuthError::Parse { line: error.line() })?;
        Ok(value
            .get(provider)
            .and_then(Value::as_str)
            .filter(|key| !key.trim().is_empty())
            .map(str::to_string))
    }
}

fn read(path: &Path) -> Result<Option<String>, AuthError> {
    match std::fs::read_to_string(path) {
        Ok(text) => Ok(Some(text)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(_) => Err(AuthError::Read),
    }
}

/// Takes the file back to owner-only where the platform expresses permissions.
/// Windows has no cheap equivalent through `std::fs`, so the profile directory
/// is the boundary there.
#[cfg(unix)]
fn restrict_to_owner(path: &Path) {
    use std::os::unix::fs::PermissionsExt;

    let Ok(metadata) = std::fs::metadata(path) else {
        return;
    };
    let mode = metadata.permissions().mode();
    if mode & 0o077 != 0 {
        let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode & 0o700));
    }
}

#[cfg(not(unix))]
fn restrict_to_owner(_path: &Path) {}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(dir: &Path, text: &str) -> PathBuf {
        let path = dir.join("auth.json");
        std::fs::write(&path, text).unwrap();
        path
    }

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("dango-auth-{name}"));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn a_key_is_read_for_its_provider() {
        let dir = temp_dir("read");
        let path = write(
            &dir,
            r#"{ "anthropic": "sk-ant-test", "openrouter": "sk-or" }"#,
        );
        let auth = AuthFile::new(path);
        assert_eq!(
            auth.key("anthropic").unwrap().as_deref(),
            Some("sk-ant-test")
        );
        assert_eq!(auth.key("openrouter").unwrap().as_deref(), Some("sk-or"));
        assert_eq!(auth.key("ollama").unwrap(), None);
    }

    #[test]
    fn a_missing_file_is_no_keys_rather_than_an_error() {
        let dir = temp_dir("missing");
        let auth = AuthFile::new(dir.join("auth.json"));
        assert_eq!(auth.key("anthropic").unwrap(), None);
    }

    #[test]
    fn an_empty_key_counts_as_absent() {
        let dir = temp_dir("empty-value");
        let path = write(&dir, r#"{ "anthropic": "  " }"#);
        assert_eq!(AuthFile::new(path).key("anthropic").unwrap(), None);
    }

    #[test]
    fn a_malformed_file_reports_where_not_what() {
        let dir = temp_dir("malformed");
        let path = write(&dir, "{ \"anthropic\": \"sk-ant-secret\" ");
        let error = AuthFile::new(path).key("anthropic").unwrap_err();
        let message = error.to_string();
        assert!(
            !message.contains("sk-ant-secret"),
            "the key reached the error: {message}"
        );
        assert!(
            message.contains("auth.json"),
            "the file is not named: {message}"
        );
    }

    #[test]
    fn a_key_added_while_running_is_picked_up() {
        let dir = temp_dir("live");
        let path = write(&dir, r#"{}"#);
        let auth = AuthFile::new(path.clone());
        assert_eq!(auth.key("anthropic").unwrap(), None);
        std::fs::write(&path, r#"{ "anthropic": "sk-ant-new" }"#).unwrap();
        assert_eq!(
            auth.key("anthropic").unwrap().as_deref(),
            Some("sk-ant-new")
        );
    }

    #[cfg(unix)]
    #[test]
    fn a_world_readable_file_is_restricted_to_its_owner() {
        use std::os::unix::fs::PermissionsExt;

        let dir = temp_dir("permissions");
        let path = write(&dir, r#"{ "anthropic": "sk-ant-test" }"#);
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).unwrap();

        let auth = AuthFile::new(path.clone());
        assert_eq!(
            auth.key("anthropic").unwrap().as_deref(),
            Some("sk-ant-test")
        );

        let mode = std::fs::metadata(&path).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o600, "file left readable by others");
    }
}
