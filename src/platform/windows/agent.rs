use std::path::{Path, PathBuf};

use process_wrap::std::{CommandWrap, CreationFlags, JobObject};
use windows::Win32::System::Threading::CREATE_NO_WINDOW;

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

/// A GUI-subsystem Dango has no console to share, so without
/// `CREATE_NO_WINDOW` the child gets a visible one, which takes focus and
/// hides the launcher. It has to go through the `CreationFlags` wrapper:
/// `JobObject` overwrites flags set on the inner command.
pub fn contained_command(program: &Path) -> CommandWrap {
    let mut command = CommandWrap::with_new(program, |_| {});
    command
        .wrap(JobObject)
        .wrap(CreationFlags(CREATE_NO_WINDOW));
    command
}

#[cfg(test)]
pub fn process_alive(pid: u32) -> bool {
    use windows::Win32::Foundation::{CloseHandle, STILL_ACTIVE};
    use windows::Win32::System::Threading::{
        GetExitCodeProcess, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION,
    };
    unsafe {
        let Ok(process) = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) else {
            return false;
        };
        let mut code = 0u32;
        let alive = GetExitCodeProcess(process, &mut code).is_ok() && code == STILL_ACTIVE.0 as u32;
        let _ = CloseHandle(process);
        alive
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_contained_command_has_no_console_and_a_job() {
        let command = contained_command(Path::new("claude"));
        assert!(command.has_wrap::<JobObject>());
        let flags = command.get_wrap::<CreationFlags>().expect("creation flags");
        assert!(flags.0.contains(CREATE_NO_WINDOW));
    }

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
