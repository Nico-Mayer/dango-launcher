//! Provider keys, read from `auth.toml` in the config directory.
//!
//! Plain text, deliberately: one user with two machines, and a file that can be
//! copied. It is its own file so `config.toml` stays committable, and Dango only
//! ever reads it.
//!
//! Two rules hold everything here together. The file is read per request rather
//! than cached, so a key added while Dango runs works immediately and none sits
//! in memory between requests. And nothing derived from the file's contents ever
//! reaches an error: only the position of a syntax error is kept, because the
//! `toml` error text quotes the line it choked on.

use std::path::{Path, PathBuf};

use toml::{Table, Value};

use crate::config::line_of;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum AuthError {
    #[error("auth.toml: line {line} isn't valid TOML")]
    Parse { line: usize },
    #[error("auth.toml could not be read")]
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

/// The table extension secrets are filed under, never a provider's key.
const EXTENSIONS: &str = "extensions";

impl AuthFile {
    /// A secret of a JavaScript extension, from its own `[extensions.<id>]`
    /// table. Read per call, like a provider key.
    pub fn extension_secret(
        &self,
        extension_id: &str,
        name: &str,
    ) -> Result<Option<String>, AuthError> {
        let Some(table) = self.table()? else {
            return Ok(None);
        };
        Ok(table
            .get(EXTENSIONS)
            .and_then(|extensions| extensions.get(extension_id))
            .and_then(|secrets| secrets.get(name))
            .and_then(Value::as_str)
            .filter(|secret| !secret.trim().is_empty())
            .map(str::to_string))
    }

    fn table(&self) -> Result<Option<Table>, AuthError> {
        let Some(text) = read(&self.path)? else {
            return Ok(None);
        };
        restrict_to_owner(&self.path);
        text.parse()
            .map(Some)
            .map_err(|error: toml::de::Error| AuthError::Parse {
                line: line_of(&text, error.span()),
            })
    }
}

impl Keys for AuthFile {
    fn key(&self, provider: &str) -> Result<Option<String>, AuthError> {
        if provider == EXTENSIONS {
            return Ok(None);
        }
        let Some(table) = self.table()? else {
            return Ok(None);
        };
        Ok(table
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
        let path = dir.join("auth.toml");
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
            "version = 1\nanthropic = \"sk-ant-test\"\nopenrouter = \"sk-or\"\n",
        );
        let auth = AuthFile::new(path);
        assert_eq!(
            auth.key("anthropic").unwrap().as_deref(),
            Some("sk-ant-test")
        );
        assert_eq!(auth.key("openrouter").unwrap().as_deref(), Some("sk-or"));
        assert_eq!(auth.key("ollama").unwrap(), None);
        assert_eq!(auth.key("version").unwrap(), None);
    }

    #[test]
    fn an_extension_reads_only_its_own_secrets() {
        let dir = temp_dir("extension-secrets");
        let path = write(
            &dir,
            "anthropic = \"sk-ant-secret\"
[extensions.gitlab]
token = \"glpat-123\"
",
        );
        let auth = AuthFile::new(path);

        assert_eq!(
            auth.extension_secret("gitlab", "token").unwrap().as_deref(),
            Some("glpat-123")
        );
        assert_eq!(auth.extension_secret("jira", "token").unwrap(), None);
        assert_eq!(auth.extension_secret("gitlab", "anthropic").unwrap(), None);
        assert_eq!(auth.extension_secret("anthropic", "x").unwrap(), None);
    }

    #[test]
    fn the_extensions_table_is_no_provider_s_key() {
        let dir = temp_dir("extensions-provider");
        let path = write(
            &dir,
            "[extensions.gitlab]
token = \"glpat-123\"
",
        );

        assert_eq!(AuthFile::new(path).key("extensions").unwrap(), None);
    }

    #[test]
    fn a_broken_file_gives_an_extension_only_the_line() {
        let dir = temp_dir("extension-broken");
        let path = write(
            &dir,
            "[extensions.gitlab]
token = \"glpat-secret
",
        );

        let error = AuthFile::new(path)
            .extension_secret("gitlab", "token")
            .unwrap_err();

        assert_eq!(error, AuthError::Parse { line: 2 });
        assert!(!error.to_string().contains("glpat-secret"));
    }

    #[test]
    fn a_missing_file_is_no_keys_rather_than_an_error() {
        let dir = temp_dir("missing");
        let auth = AuthFile::new(dir.join("auth.toml"));
        assert_eq!(auth.key("anthropic").unwrap(), None);
    }

    #[test]
    fn an_empty_key_counts_as_absent() {
        let dir = temp_dir("empty-value");
        let path = write(&dir, r#"anthropic = "  ""#);
        assert_eq!(AuthFile::new(path).key("anthropic").unwrap(), None);
    }

    #[test]
    fn a_malformed_file_reports_where_not_what() {
        let dir = temp_dir("malformed");
        let path = write(&dir, "version = 1\nanthropic = \"sk-ant-secret\n");
        let error = AuthFile::new(path).key("anthropic").unwrap_err();
        assert_eq!(error, AuthError::Parse { line: 2 });
        let message = error.to_string();
        assert!(
            !message.contains("sk-ant-secret"),
            "the key reached the error: {message}"
        );
        assert!(
            message.contains("auth.toml"),
            "the file is not named: {message}"
        );
    }

    #[test]
    fn a_key_added_while_running_is_picked_up() {
        let dir = temp_dir("live");
        let path = write(&dir, "");
        let auth = AuthFile::new(path.clone());
        assert_eq!(auth.key("anthropic").unwrap(), None);
        std::fs::write(&path, r#"anthropic = "sk-ant-new""#).unwrap();
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
        let path = write(&dir, r#"anthropic = "sk-ant-test""#);
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
