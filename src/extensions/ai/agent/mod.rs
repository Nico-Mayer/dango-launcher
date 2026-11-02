//! Agent programs as providers: the CLI a vendor ships, run once per request
//! with the sign-in the user already gave it.
//!
//! `stream` returns at once and a worker thread does the rest, so finding the
//! program, which can mean waiting on a login shell, never keeps the launcher
//! from seeing Escape or the time limit. Dropping the `Chunks` stops the
//! program and everything it started, at any point of the run.

// Nothing calls these until the agent client lands on top of them.
#[allow(dead_code)]
mod args;
#[allow(dead_code)]
mod lookup;

use std::io::{Read, Write};
use std::path::PathBuf;
use std::process::{ExitStatus, Stdio};
use std::sync::{mpsc, Arc, Mutex};
use std::time::Duration;

use process_wrap::std::ChildWrapper;

use super::provider::{AiError, Chunks, Completions, Request};

/// How often the worker looks for the program's exit once its output closed.
const EXIT_POLL: Duration = Duration::from_millis(20);

type Log = Arc<dyn Fn(&str) + Send + Sync>;

pub struct AgentClient {
    workspace: PathBuf,
    log: Log,
}

impl AgentClient {
    /// `workspace` is the empty folder every run starts in, so a program that
    /// still has a way to touch files finds none of the user's.
    pub fn new(workspace: PathBuf) -> Self {
        Self::with_log(workspace, Arc::new(crate::log::append))
    }

    fn with_log(workspace: PathBuf, log: Log) -> Self {
        Self { workspace, log }
    }
}

/// The child, kept where abandoning can reach it. Nobody blocks while holding
/// the lock: the worker only polls, so a kill never waits behind a `wait` on a
/// process that outlived the program.
#[derive(Default)]
struct Running {
    child: Option<Box<dyn ChildWrapper>>,
    abandoned: bool,
}

type Shared = Arc<Mutex<Running>>;

fn lock(shared: &Shared) -> std::sync::MutexGuard<'_, Running> {
    shared
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

impl Completions for AgentClient {
    fn stream(&self, request: Request) -> Chunks {
        let (sender, receiver) = mpsc::channel();
        let shared = Shared::default();
        let run = Run {
            request,
            workspace: self.workspace.clone(),
            log: self.log.clone(),
            shared: shared.clone(),
            sender,
        };
        std::thread::spawn(move || run.go());
        Chunks::with_cleanup(receiver, move || {
            let mut running = lock(&shared);
            running.abandoned = true;
            if let Some(child) = running.child.as_mut() {
                let _ = child.start_kill();
            }
        })
    }
}

struct Run {
    request: Request,
    workspace: PathBuf,
    log: Log,
    shared: Shared,
    sender: mpsc::Sender<Result<String, AiError>>,
}

impl Run {
    fn go(self) {
        if let Err(error) = self.attempt() {
            let _ = self.sender.send(Err(error));
        }
    }

    fn attempt(&self) -> Result<(), AiError> {
        let request = &self.request;
        let program = args::program(request.kind).ok_or_else(|| self.failed())?;
        if let Err(error) = std::fs::create_dir_all(&self.workspace) {
            self.note(&format!(
                "can't create {}: {error}",
                self.workspace.display()
            ));
            return Err(self.failed());
        }
        let located = lookup::find(program, request.path.as_deref(), &self.workspace)?;

        let mut command = crate::platform::contained_command(&located.program);
        command
            .command_mut()
            .args(args::arguments(
                request.kind,
                request.model.as_deref(),
                request.thinking,
            ))
            .current_dir(&self.workspace)
            .env("PATH", &located.path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let mut child = command.spawn().map_err(|error| {
            self.note(&format!(
                "can't start {}: {error}",
                located.program.display()
            ));
            self.failed()
        })?;

        let stdin = child.stdin().take();
        let stdout = child.stdout().take();
        let stderr = child.stderr().take();
        {
            let mut running = lock(&self.shared);
            running.child = Some(child);
            if running.abandoned {
                if let Some(child) = running.child.as_mut() {
                    let _ = child.start_kill();
                }
                return Ok(());
            }
        }

        // Written from its own thread: a program that prints before it has read
        // all of its input would otherwise fill the pipe and wait on us forever.
        if let Some(mut stdin) = stdin {
            let prompt = request.prompt.clone();
            std::thread::spawn(move || {
                let _ = stdin.write_all(prompt.as_bytes());
            });
        }
        let chatter = stderr.map(|mut stderr| {
            std::thread::spawn(move || {
                let mut text = String::new();
                let _ = stderr.read_to_string(&mut text);
                text
            })
        });

        let Some(printed) = stdout.and_then(|stdout| self.pass_on(stdout)) else {
            return Ok(());
        };
        let Some(status) = self.exit_status() else {
            return Ok(());
        };
        let chatter = chatter
            .and_then(|thread| thread.join().ok())
            .unwrap_or_default();

        if status.success() {
            if !chatter.trim().is_empty() {
                self.note(&format!("printed: {}", chatter.trim()));
            }
            return Ok(());
        }
        self.note(&format!(
            "exited with {status}. Output: {} Errors: {}",
            printed.trim(),
            chatter.trim()
        ));
        Err(self.failed())
    }

    /// Sends standard output on as it arrives and returns all of it, or `None`
    /// once nobody is reading any more.
    fn pass_on(&self, mut stdout: impl Read) -> Option<String> {
        let mut printed = String::new();
        let mut pending: Vec<u8> = Vec::new();
        let mut buffer = [0u8; 4096];
        loop {
            let read = match stdout.read(&mut buffer) {
                Ok(0) | Err(_) => break,
                Ok(read) => read,
            };
            pending.extend_from_slice(&buffer[..read]);
            // A character split across two reads waits for its other half.
            let complete = match std::str::from_utf8(&pending) {
                Ok(_) => pending.len(),
                Err(error) if error.error_len().is_none() => error.valid_up_to(),
                Err(_) => pending.len(),
            };
            let text = String::from_utf8_lossy(&pending[..complete]).into_owned();
            pending.drain(..complete);
            if text.is_empty() {
                continue;
            }
            printed.push_str(&text);
            if self.sender.send(Ok(text)).is_err() {
                return None;
            }
        }
        if !pending.is_empty() {
            let text = String::from_utf8_lossy(&pending).into_owned();
            printed.push_str(&text);
            if self.sender.send(Ok(text)).is_err() {
                return None;
            }
        }
        Some(printed)
    }

    /// The program's exit, or `None` once the request was abandoned.
    fn exit_status(&self) -> Option<ExitStatus> {
        loop {
            {
                let mut running = lock(&self.shared);
                if running.abandoned {
                    return None;
                }
                match running.child.as_mut().map(|child| child.try_wait()) {
                    Some(Ok(Some(status))) => return Some(status),
                    Some(Ok(None)) => {}
                    Some(Err(_)) | None => return None,
                }
            }
            std::thread::sleep(EXIT_POLL);
        }
    }

    fn note(&self, message: &str) {
        (self.log)(&format!("ai: {}: {message}", self.request.provider));
    }

    fn failed(&self) -> AiError {
        AiError::ProgramFailed {
            provider: self.request.provider.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{ProviderKind, Thinking};
    use crate::extensions::ai::provider::Next;
    use std::path::Path;
    use std::time::Instant;

    struct Setup {
        dir: tempfile::TempDir,
        logged: Arc<Mutex<Vec<String>>>,
        client: AgentClient,
    }

    fn setup() -> Setup {
        let dir = tempfile::tempdir().unwrap();
        let logged = Arc::new(Mutex::new(Vec::new()));
        let sink = logged.clone();
        let client = AgentClient::with_log(
            dir.path().join("workspace"),
            Arc::new(move |line: &str| sink.lock().unwrap().push(line.to_string())),
        );
        Setup {
            dir,
            logged,
            client,
        }
    }

    impl Setup {
        /// A stand-in for the CLI: a batch file on Windows, a shell script
        /// elsewhere.
        fn script(&self, windows: &str, unix: &str) -> PathBuf {
            let (name, body) = match cfg!(windows) {
                true => (
                    "program.cmd",
                    format!("@echo off\r\n{}\r\n", windows.replace('\n', "\r\n")),
                ),
                false => ("program.sh", format!("#!/bin/sh\n{unix}\n")),
            };
            let path = self.dir.path().join(name);
            std::fs::write(&path, body).unwrap();
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
            }
            path
        }

        fn log(&self) -> String {
            self.logged.lock().unwrap().join("\n")
        }

        /// The id the script's long-running child wrote down.
        fn child_pid(&self) -> u32 {
            let file = self.dir.path().join("child.pid");
            let deadline = Instant::now() + Duration::from_secs(30);
            loop {
                if let Some(pid) = std::fs::read_to_string(&file)
                    .ok()
                    .and_then(|text| text.trim().parse().ok())
                {
                    return pid;
                }
                assert!(Instant::now() < deadline, "the child never started");
                std::thread::sleep(Duration::from_millis(50));
            }
        }
    }

    fn request(path: PathBuf) -> Request {
        Request {
            provider: "claude".into(),
            kind: ProviderKind::Claude,
            base_url: None,
            model: None,
            path: Some(path),
            thinking: Thinking::Off,
            prompt: "hello there".into(),
            key: None,
        }
    }

    fn read_all(chunks: &Chunks) -> Result<String, AiError> {
        let deadline = Instant::now() + Duration::from_secs(30);
        let mut answer = String::new();
        loop {
            match chunks.next(Duration::from_millis(100)) {
                Next::Text(text) => answer.push_str(&text),
                Next::Done => return Ok(answer),
                Next::Failed(error) => return Err(error),
                Next::Idle => assert!(Instant::now() < deadline, "no end in sight"),
            }
        }
    }

    fn assert_stops(pid: u32) {
        let deadline = Instant::now() + Duration::from_secs(5);
        while crate::platform::process_alive(pid) {
            assert!(Instant::now() < deadline, "process {pid} was left running");
            std::thread::sleep(Duration::from_millis(50));
        }
    }

    #[test]
    fn the_prompt_goes_in_on_stdin_and_the_output_is_the_answer() {
        let setup = setup();
        let program = setup.script(r#"findstr "^""#, "cat");
        let answer = read_all(&setup.client.stream(request(program))).unwrap();
        assert_eq!(answer.trim(), "hello there");
    }

    #[test]
    fn stream_returns_before_the_program_has_done_anything() {
        let setup = setup();
        let program = setup.script("ping -n 3 127.0.0.1 >nul", "sleep 2");
        let started = Instant::now();
        let chunks = setup.client.stream(request(program));
        assert!(started.elapsed() < Duration::from_millis(200));
        drop(chunks);
    }

    #[test]
    fn the_error_output_goes_to_the_log_and_not_into_the_answer() {
        let setup = setup();
        let program = setup.script(
            "echo warming up 1>&2\necho The answer",
            "echo warming up >&2\necho The answer",
        );
        let answer = read_all(&setup.client.stream(request(program))).unwrap();
        assert_eq!(answer.trim(), "The answer");
        assert!(setup.log().contains("warming up"), "{}", setup.log());
    }

    #[test]
    fn a_failing_program_is_reported_and_both_outputs_are_logged() {
        let setup = setup();
        let program = setup.script(
            "echo not signed in\necho token expired 1>&2\nexit /b 3",
            "echo not signed in\necho token expired >&2\nexit 3",
        );
        let error = read_all(&setup.client.stream(request(program))).unwrap_err();
        assert_eq!(
            error.to_string(),
            "claude stopped with an error. See dango.log for the reason."
        );
        let log = setup.log();
        assert!(log.contains("not signed in"), "{log}");
        assert!(log.contains("token expired"), "{log}");
    }

    #[test]
    fn a_missing_configured_path_starts_nothing() {
        let setup = setup();
        let missing = setup.dir.path().join("not-there.exe");
        let error = read_all(&setup.client.stream(request(missing.clone()))).unwrap_err();
        assert_eq!(
            error,
            AiError::PathMissing {
                path: missing.display().to_string()
            }
        );
        assert!(setup.log().is_empty(), "{}", setup.log());
    }

    #[test]
    fn abandoning_stops_a_child_that_keeps_the_output_open() {
        let setup = setup();
        let program = setup.script(
            r#"powershell -NoProfile -Command "$PID | Out-File -Encoding ascii '%~dp0child.pid'; Start-Sleep 60""#,
            r#"sleep 60 &
echo $! > "$(dirname "$0")/child.pid"
wait"#,
        );
        let chunks = setup.client.stream(request(program));
        let pid = setup.child_pid();

        let started = Instant::now();
        drop(chunks);
        assert!(started.elapsed() < Duration::from_millis(500));
        assert_stops(pid);
    }

    #[test]
    fn abandoning_stops_a_child_the_program_left_behind() {
        let setup = setup();
        let program = setup.script(
            r#"start "" /b powershell -NoProfile -Command "$PID | Out-File -Encoding ascii '%~dp0child.pid'; Start-Sleep 60" >nul 2>&1"#,
            r#"sleep 60 >/dev/null 2>&1 &
echo $! > "$(dirname "$0")/child.pid""#,
        );
        let chunks = setup.client.stream(request(program));
        let pid = setup.child_pid();
        std::thread::sleep(Duration::from_millis(300));

        let started = Instant::now();
        drop(chunks);
        assert!(started.elapsed() < Duration::from_millis(500));
        assert_stops(pid);
    }
}
