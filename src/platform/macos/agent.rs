use std::io::Read;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use process_wrap::std::{CommandWrap, ProcessGroup};

/// A profile that waits on a passphrase or a slow network call must not hold
/// the request up for longer than this.
const LOGIN_SHELL_LIMIT: Duration = Duration::from_secs(10);

pub fn contained_command(program: &Path) -> CommandWrap {
    let mut command = CommandWrap::with_new(program, |_| {});
    command.wrap(ProcessGroup::leader());
    command
}

/// Where agent CLIs land besides the login shell's PATH.
pub fn agent_known_dirs() -> Vec<PathBuf> {
    let home = dirs::home_dir();
    let mise = std::env::var_os("MISE_DATA_DIR")
        .map(PathBuf::from)
        .or_else(|| {
            home.as_ref()
                .map(|home| home.join(".local").join("share").join("mise"))
        });
    [
        mise.map(|mise| mise.join("shims")),
        home.map(|home| home.join(".local").join("bin")),
        Some(PathBuf::from("/opt/homebrew/bin")),
        Some(PathBuf::from("/usr/local/bin")),
    ]
    .into_iter()
    .flatten()
    .collect()
}

/// What `env` prints in the user's login shell. An app opened from Finder or
/// at login gets launchd's PATH, without anything the shell profile adds.
///
/// Not interactive, so the profile cannot stop at a prompt or print a banner,
/// and in its own process group, so a slow profile is killed with everything it
/// started.
pub fn login_shell_env() -> Option<String> {
    let shell = std::env::var_os("SHELL")
        .filter(|shell| !shell.is_empty())
        .unwrap_or_else(|| "/bin/zsh".into());
    let mut child = Command::new(shell)
        .args(["-l", "-c", "env"])
        .current_dir(std::env::temp_dir())
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .process_group(0)
        .spawn()
        .ok()?;
    let group = child.id() as libc::pid_t;
    let mut stdout = child.stdout.take()?;
    let reader = std::thread::spawn(move || {
        let mut output = String::new();
        let _ = stdout.read_to_string(&mut output);
        output
    });

    let deadline = Instant::now() + LOGIN_SHELL_LIMIT;
    let finished = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status.success(),
            Ok(None) if Instant::now() < deadline => std::thread::sleep(Duration::from_millis(50)),
            _ => break false,
        }
    };
    // Also after a clean exit: a process the profile left running would keep
    // stdout open and the reader waiting.
    unsafe {
        libc::killpg(group, libc::SIGKILL);
    }
    let _ = child.wait();
    let output = reader.join().ok()?;
    finished.then_some(output)
}
