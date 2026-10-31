//! Where the config file lives.
//!
//! A fixed `~/.config/dango/` on both platforms, chosen over the OS-native
//! config dir because the point is a predictable, git-managed path a person can
//! symlink and commit. `$DANGO_CONFIG_DIR` overrides it for anyone who keeps
//! their dotfiles elsewhere.

use std::path::PathBuf;

pub const CONFIG_DIR_ENV: &str = "DANGO_CONFIG_DIR";

/// The directory the config file lives in, honouring `$DANGO_CONFIG_DIR`.
pub fn config_dir() -> PathBuf {
    resolve(std::env::var_os(CONFIG_DIR_ENV).map(PathBuf::from), home())
}

/// The config file itself.
pub fn config_path() -> PathBuf {
    config_dir().join("config.json")
}

/// How the config file refers to the schema Dango writes beside it, relative
/// so the pair can move together.
pub const SCHEMA_REF: &str = "./config.schema.json";

/// The provider keys, beside the config file but deliberately not in it: the
/// config is meant to be committed and this file is not.
pub fn auth_path() -> PathBuf {
    config_dir().join("auth.json")
}

fn resolve(override_dir: Option<PathBuf>, home: PathBuf) -> PathBuf {
    override_dir.unwrap_or_else(|| home.join(".config").join("dango"))
}

fn home() -> PathBuf {
    dirs::home_dir().unwrap_or_else(|| PathBuf::from("."))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_dot_config_dango() {
        let dir = resolve(None, PathBuf::from("/home/nico"));
        assert_eq!(dir, PathBuf::from("/home/nico/.config/dango"));
    }

    #[test]
    fn the_override_wins() {
        let dir = resolve(
            Some(PathBuf::from("/somewhere/else")),
            PathBuf::from("/home/nico"),
        );
        assert_eq!(dir, PathBuf::from("/somewhere/else"));
    }
}
