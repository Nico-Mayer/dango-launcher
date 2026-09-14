//! A provider that is a program rather than a service.
//!
//! A subscription to ChatGPT or Claude buys inference no API key can spend, and
//! the vendor's own CLI is already signed in to it. Running that program is the
//! whole idea, and it generalises: anything that takes a prompt and prints an
//! answer can be a provider.
//!
//! Nothing here knows about any particular program. The command and its
//! arguments come from the configuration, `{prompt}` and `{model}` are replaced
//! where they appear, and standard output is the answer. What the program writes
//! on standard error is its own chatter and goes to the log.
//!
//! The child is spawned inside a job object on Windows and a process group on
//! Unix, through `process-wrap`, because the programs worth driving here are
//! wrapper scripts: killing the one we spawned would leave the agent it started
//! running, which is what abandoning an answer must not do.

use std::io::{Read, Write};
use std::process::Stdio;
use std::sync::{mpsc, Arc, Mutex};

use process_wrap::std::{ChildWrapper, CommandWrap};

use super::provider::{AiError, Chunks, CliSpec, Completions, Request};

/// The placeholders the arguments may carry.
const PROMPT: &str = "{prompt}";
const MODEL: &str = "{model}";

pub struct CliClient;

impl Completions for CliClient {
    fn stream(&self, request: Request) -> Chunks {
        let (sender, receiver) = mpsc::channel();
        let Some(spec) = request.cli.clone() else {
            let _ = sender.send(Err(AiError::Unreachable {
                provider: request.provider.clone(),
            }));
            return Chunks::new(receiver);
        };

        let args = arguments(&spec, &request);
        let mut command = CommandWrap::with_new(&spec.command, |command| {
            command
                .args(&args)
                .current_dir(workspace())
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped());
            // A launcher must not flash a console window on its way to an answer.
            #[cfg(windows)]
            {
                use std::os::windows::process::CommandExt;
                const CREATE_NO_WINDOW: u32 = 0x0800_0000;
                command.creation_flags(CREATE_NO_WINDOW);
            }
        });
        #[cfg(windows)]
        command.wrap(process_wrap::std::JobObject);
        #[cfg(unix)]
        command.wrap(process_wrap::std::ProcessGroup::leader());

        let mut child = match command.spawn() {
            Ok(child) => child,
            Err(_) => {
                let _ = sender.send(Err(AiError::CommandMissing {
                    command: spec.command.clone(),
                }));
                return Chunks::new(receiver);
            }
        };

        // The prompt goes in on standard input unless an argument asked for it.
        if !wants_prompt_argument(&spec) {
            if let Some(mut stdin) = child.stdin().take() {
                let _ = stdin.write_all(request.prompt.as_bytes());
            }
        }
        // Dropped either way: a program waiting on end-of-input never answers.
        drop(child.stdin().take());

        let stdout = child.stdout().take();
        let stderr = child.stderr().take();
        let provider = request.provider.clone();
        let running = Arc::new(Mutex::new(Some(child)));

        {
            let running = running.clone();
            std::thread::spawn(move || pump(stdout, stderr, provider, running, sender));
        }

        // Abandoning has to stop an agent, not orphan it.
        Chunks::with_cleanup(receiver, move || {
            if let Ok(mut held) = running.lock() {
                if let Some(child) = held.as_mut() {
                    let _ = child.kill();
                }
            }
        })
    }
}

type Running = Arc<Mutex<Option<Box<dyn ChildWrapper>>>>;

fn pump(
    stdout: Option<std::process::ChildStdout>,
    stderr: Option<std::process::ChildStderr>,
    provider: String,
    running: Running,
    sender: mpsc::Sender<Result<String, AiError>>,
) {
    let noise = stderr.map(|mut stderr| {
        std::thread::spawn(move || {
            let mut text = String::new();
            let _ = stderr.read_to_string(&mut text);
            text
        })
    });

    let mut answered = false;
    if let Some(mut stdout) = stdout {
        // Decoding as it arrives, so a multi-byte character split across two
        // reads is held back rather than mangled.
        let mut pending: Vec<u8> = Vec::new();
        let mut buffer = [0u8; 4096];
        loop {
            match stdout.read(&mut buffer) {
                Ok(0) | Err(_) => break,
                Ok(read) => {
                    pending.extend_from_slice(&buffer[..read]);
                    let text = match std::str::from_utf8(&pending) {
                        Ok(text) => {
                            let text = text.to_string();
                            pending.clear();
                            text
                        }
                        Err(error) => {
                            let good = error.valid_up_to();
                            let text = String::from_utf8_lossy(&pending[..good]).to_string();
                            pending.drain(..good);
                            text
                        }
                    };
                    if text.is_empty() {
                        continue;
                    }
                    answered = true;
                    if sender.send(Ok(text)).is_err() {
                        return;
                    }
                }
            }
        }
    }

    let status = running
        .lock()
        .ok()
        .and_then(|mut held| held.as_mut().map(|child| child.wait()));
    let failed = matches!(status, Some(Ok(status)) if !status.success());

    if let Some(noise) = noise {
        if let Ok(text) = noise.join() {
            if !text.trim().is_empty() {
                crate::log::append(&format!("ai: {provider} printed: {}", text.trim()));
            }
        }
    }

    if failed && !answered {
        let _ = sender.send(Err(AiError::CommandFailed { provider }));
    }
}

/// A program that can touch files is given nothing of the user's to touch. The
/// flags that constrain it further belong in the user's own arguments, because
/// only they know how each program spells them.
fn workspace() -> std::path::PathBuf {
    let dir = std::env::temp_dir().join("dango-cli");
    let _ = std::fs::create_dir_all(&dir);
    dir
}

fn wants_prompt_argument(spec: &CliSpec) -> bool {
    spec.args.iter().any(|arg| arg.contains(PROMPT))
}

fn arguments(spec: &CliSpec, request: &Request) -> Vec<String> {
    spec.args
        .iter()
        .map(|arg| {
            arg.replace(PROMPT, &request.prompt)
                .replace(MODEL, &request.model)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ProviderKind;
    use crate::extensions::ai::provider::{Next, Thinking};
    use std::time::Duration;

    fn request(command: &str, args: &[&str]) -> Request {
        Request {
            provider: "helper".into(),
            kind: ProviderKind::Cli,
            base_url: None,
            model: "some-model".into(),
            thinking: Thinking::Off,
            prompt: "say hello".into(),
            key: None,
            cli: Some(CliSpec {
                command: command.into(),
                args: args.iter().map(|arg| arg.to_string()).collect(),
            }),
        }
    }

    /// Every test drives a real child process, because the point of this client
    /// is what happens around one. The interpreter running the test suite is the
    /// one program guaranteed to be installed.
    fn shell() -> &'static str {
        if cfg!(windows) {
            "cmd"
        } else {
            "sh"
        }
    }

    fn echo_args(text: &str) -> Vec<String> {
        if cfg!(windows) {
            vec!["/c".into(), format!("echo {text}")]
        } else {
            vec!["-c".into(), format!("echo {text}")]
        }
    }

    fn collect(chunks: &Chunks) -> Result<String, AiError> {
        let mut answer = String::new();
        let deadline = std::time::Instant::now() + Duration::from_secs(20);
        loop {
            match chunks.next(Duration::from_millis(100)) {
                Next::Text(text) => answer.push_str(&text),
                Next::Done => return Ok(answer),
                Next::Failed(error) => return Err(error),
                Next::Idle if std::time::Instant::now() > deadline => {
                    panic!("the child never finished")
                }
                Next::Idle => {}
            }
        }
    }

    #[test]
    fn the_prompt_goes_in_on_standard_input_by_default() {
        // `sort` and `more` both read stdin and print it; `more` is the one both
        // platforms have.
        let args: Vec<&str> = if cfg!(windows) { vec!["/c", "more"] } else { vec!["-c", "cat"] };
        let chunks = CliClient.stream(request(shell(), &args));
        assert_eq!(collect(&chunks).unwrap().trim(), "say hello");
    }

    #[test]
    fn an_argument_placeholder_carries_the_prompt_instead() {
        let args = echo_args("[{prompt}]");
        let borrowed: Vec<&str> = args.iter().map(String::as_str).collect();
        let chunks = CliClient.stream(request(shell(), &borrowed));
        assert!(
            collect(&chunks).unwrap().contains("say hello"),
            "the prompt did not reach the arguments"
        );
    }

    #[test]
    fn the_model_placeholder_is_replaced_too() {
        let args = echo_args("model={model}");
        let borrowed: Vec<&str> = args.iter().map(String::as_str).collect();
        let chunks = CliClient.stream(request(shell(), &borrowed));
        assert!(collect(&chunks).unwrap().contains("model=some-model"));
    }

    #[test]
    fn a_command_that_is_not_installed_names_itself() {
        let chunks = CliClient.stream(request("dango-nothing-here", &[]));
        let error = collect(&chunks).unwrap_err();
        assert_eq!(
            error.to_string(),
            "Dango couldn't run dango-nothing-here. Check that it's installed and on your PATH."
        );
    }

    #[test]
    fn a_command_that_answers_nothing_and_fails_says_so() {
        let args: Vec<&str> = if cfg!(windows) {
            vec!["/c", "exit 3"]
        } else {
            vec!["-c", "exit 3"]
        };
        let chunks = CliClient.stream(request(shell(), &args));
        let error = collect(&chunks).unwrap_err();
        assert_eq!(
            error.to_string(),
            "helper gave no answer. See dango.log for what it printed."
        );
    }

    #[test]
    fn the_child_runs_somewhere_with_none_of_the_users_files() {
        let args: Vec<&str> = if cfg!(windows) { vec!["/c", "cd"] } else { vec!["-c", "pwd"] };
        let chunks = CliClient.stream(request(shell(), &args));
        let answer = collect(&chunks).unwrap();
        assert!(
            answer.to_lowercase().contains("dango-cli"),
            "ran somewhere else: {answer}"
        );
    }

    /// Against a CLI this machine has signed in, so never part of a normal run:
    /// `cargo test a_signed_in_cli_answers -- --ignored --nocapture`.
    /// `DANGO_TEST_CLI` names the program; it defaults to the Claude CLI.
    #[test]
    #[ignore]
    fn a_signed_in_cli_answers() {
        use crate::extensions::ai::commands::{AiCommand, Output, Providers};

        let program = std::env::var("DANGO_TEST_CLI").unwrap_or("claude".to_string());
        let config = crate::config::Config::parse(&format!(
            r#"{{ "extensions": {{ "dango.ai": {{ "providers": {{
                "subscription": {{ "kind": "cli", "command": "{program}", "args": ["-p"] }}
            }} }} }} }}"#
        ))
        .unwrap();

        let mut request = Providers::from_config(&config)
            .resolve(&AiCommand {
                id: "t".into(),
                title: "T".into(),
                prompt: "{{ selection }}".into(),
                provider: None,
                model: None,
                thinking: None,
                output: Output::View,
            })
            .expect("the provider resolves");
        request.prompt = "Reply with exactly one word: pong".into();

        let answer = collect(&CliClient.stream(request)).unwrap();
        println!("--- answer ---
{answer}
--- end ---");
        assert!(!answer.trim().is_empty(), "the program answered with nothing");
    }

    #[test]
    fn abandoning_the_answer_stops_the_program() {
        let args: Vec<&str> = if cfg!(windows) {
            vec!["/c", "ping -n 30 127.0.0.1 > nul"]
        } else {
            vec!["-c", "sleep 30"]
        };
        let chunks = CliClient.stream(request(shell(), &args));
        // Nothing to read: the program is busy.
        assert_eq!(chunks.next(Duration::from_millis(200)), Next::Idle);
        let started = std::time::Instant::now();
        drop(chunks);
        assert!(
            started.elapsed() < Duration::from_secs(5),
            "dropping the answer waited for the program"
        );
    }

    /// The programs worth driving are wrapper scripts, so the one spawned is
    /// rarely the one doing the work. Killing only that one leaves the real
    /// process running, which this catches: the shell is the child, and the
    /// sleeping program it starts is the grandchild.
    #[test]
    fn abandoning_stops_what_the_program_itself_started() {
        fn sleepers() -> usize {
            let listing = if cfg!(windows) {
                std::process::Command::new("tasklist")
                    .args(["/fi", "imagename eq PING.EXE", "/nh"])
                    .output()
            } else {
                std::process::Command::new("pgrep").args(["-x", "sleep"]).output()
            };
            let Ok(listing) = listing else { return 0 };
            let text = String::from_utf8_lossy(&listing.stdout).to_lowercase();
            if cfg!(windows) {
                text.matches("ping.exe").count()
            } else {
                text.lines().filter(|line| !line.trim().is_empty()).count()
            }
        }

        let before = sleepers();
        let args: Vec<&str> = if cfg!(windows) {
            vec!["/c", "ping -n 30 127.0.0.1 > nul"]
        } else {
            vec!["-c", "sleep 30"]
        };
        let chunks = CliClient.stream(request(shell(), &args));
        assert_eq!(chunks.next(Duration::from_millis(1500)), Next::Idle);
        assert!(sleepers() > before, "the grandchild never started");

        drop(chunks);
        std::thread::sleep(Duration::from_millis(1500));
        assert_eq!(
            sleepers(),
            before,
            "the program's own child outlived the answer"
        );
    }
}
