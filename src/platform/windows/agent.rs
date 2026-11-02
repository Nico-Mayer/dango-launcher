use std::path::PathBuf;

/// Where agent CLIs land that Explorer's PATH can lack: `mise activate` adds
/// its folders only inside a shell, and npm, WinGet, scoop and the Claude
/// installer each have their own.
pub fn agent_known_dirs() -> Vec<PathBuf> {
    let home = dirs::home_dir();
    let local = dirs::data_local_dir();
    let mise = std::env::var_os("MISE_DATA_DIR")
        .map(PathBuf::from)
        .or_else(|| local.as_ref().map(|local| local.join("mise")));
    [
        mise.map(|mise| mise.join("shims")),
        home.as_ref().map(|home| home.join(".local").join("bin")),
        dirs::data_dir().map(|roaming| roaming.join("npm")),
        local.map(|local| local.join("Microsoft").join("WinGet").join("Links")),
        home.map(|home| home.join("scoop").join("shims")),
    ]
    .into_iter()
    .flatten()
    .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_known_dirs_cover_mise_npm_and_winget() {
        let dirs = agent_known_dirs();
        for tail in [r"shims", r"\npm", r"WinGet\Links", r".local\bin"] {
            assert!(
                dirs.iter().any(|dir| dir.to_string_lossy().ends_with(tail)),
                "no folder ending in {tail}: {dirs:?}"
            );
        }
    }
}
