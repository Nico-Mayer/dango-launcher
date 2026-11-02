//! Builds, starts, and stops this checkout's Dango debug build.

use std::fs::File;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;

use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, System, UpdateKind};
use tokio::process::{Child, Command};

pub const PORT: &str = "9315";

#[derive(Clone)]
pub struct Checkout {
    pub root: PathBuf,
    pub log: PathBuf,
}

impl Checkout {
    pub fn new() -> Self {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..");
        let root = std::fs::canonicalize(&root).unwrap_or(root);
        let log = root.join("target").join("dango-ui-mcp.log");
        Self { root, log }
    }

    fn exe(&self) -> PathBuf {
        self.root
            .join("target")
            .join("debug")
            .join(format!("dango{}", std::env::consts::EXE_SUFFIX))
    }

    fn command(&self, program: impl AsRef<std::ffi::OsStr>, log: &File) -> io::Result<Command> {
        let mut command = Command::new(program);
        command
            .current_dir(&self.root)
            .stdin(Stdio::null())
            .stdout(log.try_clone()?)
            .stderr(log.try_clone()?);
        #[cfg(windows)]
        command.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
        Ok(command)
    }

    /// Builds with the Slint MCP server compiled in. The build log replaces
    /// the previous one.
    pub async fn build(&self) -> Result<(), String> {
        let log = std::fs::create_dir_all(self.root.join("target"))
            .and_then(|()| File::create(&self.log))
            .map_err(|error| format!("Couldn't write {}: {error}", self.log.display()))?;
        let status = self
            .command("cargo", &log)
            .and_then(|mut command| {
                command.args(["build", "--features", "slint/mcp"]);
                command.spawn()
            })
            .map_err(|error| format!("Couldn't start `cargo build`: {error}"))?
            .wait()
            .await
            .map_err(|error| format!("`cargo build` failed: {error}"))?;
        if status.success() {
            Ok(())
        } else {
            Err(format!(
                "The build failed ({status}).\n\n{}",
                self.log_tail(30)
            ))
        }
    }

    pub fn spawn(&self) -> io::Result<Child> {
        let log = File::options().append(true).create(true).open(&self.log)?;
        let mut command = self.command(self.exe(), &log)?;
        command.env("SLINT_MCP_PORT", PORT);
        command.spawn()
    }

    /// Kills every process running this checkout's debug build, however it
    /// was started, and waits until they are gone. Never touches an installed
    /// Dango. Returns how many it stopped.
    pub fn stop(&self) -> usize {
        let exe = self.exe();
        let Ok(mut system) = System::new() else {
            return 0;
        };
        system.refresh_processes_specifics(
            ProcessesToUpdate::All,
            true,
            ProcessRefreshKind::nothing().with_exe(UpdateKind::Always),
        );
        let ours: Vec<_> = system
            .processes()
            .values()
            .filter(|process| {
                process
                    .exe()
                    .and_then(|path| std::fs::canonicalize(path).ok())
                    .is_some_and(|path| path == exe)
            })
            .collect();
        for process in &ours {
            if process.kill() {
                process.wait();
            }
        }
        ours.len()
    }

    pub fn log_tail(&self, lines: usize) -> String {
        let log = std::fs::read_to_string(&self.log).unwrap_or_default();
        let all: Vec<&str> = log.lines().collect();
        format!(
            "End of {}:\n{}",
            self.log.display(),
            all[all.len().saturating_sub(lines)..].join("\n")
        )
    }
}

/// The single instance socket answers until a killed Dango is fully gone.
pub async fn wait_until_gone(running: impl Fn() -> bool) {
    for _ in 0..20 {
        if !running() {
            return;
        }
        tokio::time::sleep(Duration::from_millis(250)).await;
    }
}
