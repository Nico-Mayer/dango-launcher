use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};

use windows::Win32::UI::Shell::{FOLDERID_Screenshots, SHGetKnownFolderPath, KF_FLAG_DEFAULT};

use super::apps::take_pwstr;

/// The Screenshots known folder, wherever the user moved or OneDrive
/// redirected it.
pub fn screenshot_folder() -> Option<PathBuf> {
    let path =
        unsafe { SHGetKnownFolderPath(&FOLDERID_Screenshots, KF_FLAG_DEFAULT, None) }.ok()?;
    Some(PathBuf::from(take_pwstr(path))).filter(|path| !path.as_os_str().is_empty())
}

/// The path is quoted on its own. Quoting the whole `/select,` argument, as
/// `arg` does for anything with a space, makes Explorer ignore it and open
/// the default folder instead.
pub fn reveal(path: &Path) -> std::io::Result<()> {
    std::process::Command::new("explorer.exe")
        .raw_arg(format!("/select,\"{}\"", path.display()))
        .spawn()
        .map(|_| ())
}
