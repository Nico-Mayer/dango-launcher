//! `dango.exec`: running a program the way the AI agent runs its CLI, with no
//! window, the PATH a shell would give, and a kill when the caller lets go.

use std::collections::HashMap;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use process_wrap::std::ChildWrapper;
use serde::{Deserialize, Serialize};

use crate::extensions::ai::agent::lookup;

const POLL: Duration = Duration::from_millis(20);

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecRequest {
    pub program: String,
    #[serde(default)]
    pub args: Vec<String>,
    pub cwd: Option<String>,
    #[serde(default)]
    pub env: HashMap<String, String>,
    pub input: Option<String>,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct ExecResult {
    pub code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
}

type Slot = Arc<Mutex<Option<Box<dyn ChildWrapper + Send>>>>;

/// Kills the program when the call is dropped before it finished, as when
/// the session it runs for is stopped.
struct KillOnDrop(Slot);

impl Drop for KillOnDrop {
    fn drop(&mut self) {
        if let Some(child) = self.0.lock().unwrap().as_mut() {
            let _ = child.start_kill();
        }
    }
}

/// Runs `request` from `folder` unless it names a working directory. The
/// PATH is the one Dango finds agent CLIs on, so tray-started Dango finds
/// what a shell would.
pub async fn exec(request: ExecRequest, folder: PathBuf) -> Result<ExecResult, String> {
    exec_watched(request, folder, |_| {}).await
}

async fn exec_watched(
    request: ExecRequest,
    folder: PathBuf,
    on_spawn: impl FnOnce(u32) + Send + 'static,
) -> Result<ExecResult, String> {
    let slot: Slot = Arc::default();
    let guard = KillOnDrop(slot.clone());
    let result = tokio::task::spawn_blocking(move || run(request, &folder, &slot, on_spawn))
        .await
        .map_err(|error| error.to_string())?;
    std::mem::forget(guard);
    result
}

fn run(
    request: ExecRequest,
    folder: &Path,
    slot: &Slot,
    on_spawn: impl FnOnce(u32),
) -> Result<ExecResult, String> {
    let cwd = request
        .cwd
        .map(PathBuf::from)
        .unwrap_or_else(|| folder.to_path_buf());
    let located = lookup::find(&request.program, None, &cwd)
        .map_err(|_| format!("{} wasn't found", request.program))?;
    let mut command = crate::platform::contained_command(&located.program);
    command
        .command_mut()
        .args(&request.args)
        .current_dir(&cwd)
        .env("PATH", &located.path)
        .envs(&request.env)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = command
        .spawn()
        .map_err(|error| format!("{} couldn't start: {error}", request.program))?;
    on_spawn(child.id());
    let stdin = child.stdin().take();
    let stdout = child.stdout().take();
    let stderr = child.stderr().take();
    *slot.lock().unwrap() = Some(child);

    if let Some(mut stdin) = stdin {
        let input = request.input.unwrap_or_default();
        std::thread::spawn(move || {
            let _ = stdin.write_all(input.as_bytes());
        });
    }
    let stdout = read_all(stdout);
    let stderr = read_all(stderr);
    // Polled rather than waited on, so a kill from the guard never queues
    // behind a blocking wait for the lock.
    let status = loop {
        let mut guard = slot.lock().unwrap();
        let Some(child) = guard.as_mut() else {
            return Err("the program was stopped".into());
        };
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) => {}
            Err(error) => return Err(error.to_string()),
        }
        drop(guard);
        std::thread::sleep(POLL);
    };
    *slot.lock().unwrap() = None;
    Ok(ExecResult {
        code: status.code(),
        stdout: stdout.join().unwrap_or_default(),
        stderr: stderr.join().unwrap_or_default(),
    })
}

fn read_all(pipe: Option<impl Read + Send + 'static>) -> std::thread::JoinHandle<String> {
    std::thread::spawn(move || {
        let mut bytes = Vec::new();
        if let Some(mut pipe) = pipe {
            let _ = pipe.read_to_end(&mut bytes);
        }
        String::from_utf8_lossy(&bytes).into_owned()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(program: &str, args: &[&str]) -> ExecRequest {
        ExecRequest {
            program: program.into(),
            args: args.iter().map(|arg| arg.to_string()).collect(),
            cwd: None,
            env: HashMap::new(),
            input: None,
        }
    }

    #[cfg(target_os = "windows")]
    fn echo() -> ExecRequest {
        request("cmd", &["/c", "echo hi"])
    }

    #[cfg(target_os = "macos")]
    fn echo() -> ExecRequest {
        request("sh", &["-c", "echo hi"])
    }

    #[cfg(target_os = "windows")]
    fn long() -> ExecRequest {
        request("ping", &["-n", "30", "127.0.0.1"])
    }

    #[cfg(target_os = "macos")]
    fn long() -> ExecRequest {
        request("sleep", &["30"])
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn a_program_returns_its_code_and_output() {
        let result = exec(echo(), std::env::temp_dir()).await.unwrap();

        assert_eq!(result.code, Some(0));
        assert_eq!(result.stdout.trim(), "hi");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn a_missing_program_is_named() {
        let error = exec(request("no-such-program-dango", &[]), std::env::temp_dir())
            .await
            .unwrap_err();

        assert_eq!(error, "no-such-program-dango wasn't found");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn dropping_the_call_stops_the_program() {
        let (sender, spawned) = tokio::sync::oneshot::channel();
        let call = tokio::spawn(exec_watched(long(), std::env::temp_dir(), move |pid| {
            let _ = sender.send(pid);
        }));
        let pid = spawned.await.unwrap();
        assert!(crate::platform::process_alive(pid));

        call.abort();
        let _ = call.await;

        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        while crate::platform::process_alive(pid) {
            assert!(
                std::time::Instant::now() < deadline,
                "the program kept running"
            );
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    }
}
