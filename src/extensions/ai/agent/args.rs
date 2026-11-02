//! What each supported CLI is told, beyond the prompt it reads on stdin.
//!
//! The fixed flags turn off everything that makes the program an agent rather
//! than a model: tools, MCP servers, hooks, plugins, skills, context files, and
//! saved sessions. They are the reason each CLI is its own kind.

use crate::config::{ProviderKind, Thinking};

const CLAUDE: &[&str] = &[
    "-p",
    "--output-format",
    "text",
    "--safe-mode",
    "--tools",
    "",
    "--no-session-persistence",
];

const PI: &[&str] = &[
    "-p",
    "--no-tools",
    "--no-extensions",
    "--no-mcp",
    "--no-skills",
    "--no-prompt-templates",
    "--no-context-files",
    "--no-session",
];

/// The program a kind runs when no path is configured.
pub fn program(kind: ProviderKind) -> Option<&'static str> {
    match kind {
        ProviderKind::Claude => Some("claude"),
        ProviderKind::Pi => Some("pi"),
        ProviderKind::Anthropic | ProviderKind::Openai | ProviderKind::Ollama => None,
    }
}

pub fn arguments(kind: ProviderKind, model: Option<&str>, thinking: Thinking) -> Vec<String> {
    let (fixed, model_flag): (&[&str], &str) = match kind {
        ProviderKind::Claude => (CLAUDE, "--model"),
        ProviderKind::Pi => (PI, "--model"),
        ProviderKind::Anthropic | ProviderKind::Openai | ProviderKind::Ollama => return Vec::new(),
    };
    let mut args: Vec<String> = fixed.iter().map(|arg| arg.to_string()).collect();
    if let Some(model) = model {
        args.extend([model_flag.to_string(), model.to_string()]);
    }
    if let Some(level) = effort(kind, thinking) {
        let flag = match kind {
            ProviderKind::Pi => "--thinking",
            _ => "--effort",
        };
        args.extend([flag.to_string(), level.to_string()]);
    }
    args
}

/// Claude has no "off" effort, so off leaves its own default. Pi has one, and
/// is told so, or it would think at whatever level the user set for it.
fn effort(kind: ProviderKind, thinking: Thinking) -> Option<&'static str> {
    match (kind, thinking) {
        (ProviderKind::Pi, Thinking::Off) => Some("off"),
        (_, Thinking::Off) => None,
        (_, Thinking::Low) => Some("low"),
        (_, Thinking::Medium) => Some("medium"),
        (_, Thinking::High) => Some("high"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tail(args: &[String], fixed: &[&str]) -> Vec<String> {
        assert_eq!(&args[..fixed.len()], fixed, "the fixed flags come first");
        args[fixed.len()..].to_vec()
    }

    #[test]
    fn claude_runs_with_its_agent_setup_off() {
        let args = arguments(ProviderKind::Claude, None, Thinking::Off);
        assert_eq!(
            args,
            [
                "-p",
                "--output-format",
                "text",
                "--safe-mode",
                "--tools",
                "",
                "--no-session-persistence"
            ]
        );
    }

    #[test]
    fn claude_gets_the_model_and_an_effort_per_level() {
        let args = arguments(ProviderKind::Claude, Some("sonnet"), Thinking::Off);
        assert_eq!(tail(&args, CLAUDE), ["--model", "sonnet"]);
        for (thinking, level) in [
            (Thinking::Low, "low"),
            (Thinking::Medium, "medium"),
            (Thinking::High, "high"),
        ] {
            let args = arguments(ProviderKind::Claude, None, thinking);
            assert_eq!(tail(&args, CLAUDE), ["--effort", level]);
        }
    }

    #[test]
    fn pi_runs_with_its_agent_setup_off() {
        let args = arguments(ProviderKind::Pi, None, Thinking::Low);
        let fixed = tail(&args, PI);
        assert_eq!(fixed, ["--thinking", "low"]);
        for flag in [
            "--no-tools",
            "--no-mcp",
            "--no-context-files",
            "--no-session",
        ] {
            assert!(args.iter().any(|arg| arg == flag), "missing {flag}");
        }
    }

    #[test]
    fn pi_is_told_thinking_is_off_and_gets_the_model() {
        let args = arguments(ProviderKind::Pi, Some("gpt-5"), Thinking::Off);
        assert_eq!(tail(&args, PI), ["--model", "gpt-5", "--thinking", "off"]);
        let args = arguments(ProviderKind::Pi, None, Thinking::High);
        assert_eq!(tail(&args, PI), ["--thinking", "high"]);
    }

    #[test]
    fn only_agent_kinds_name_a_program() {
        assert_eq!(program(ProviderKind::Claude), Some("claude"));
        assert_eq!(program(ProviderKind::Pi), Some("pi"));
        assert_eq!(program(ProviderKind::Ollama), None);
        assert!(arguments(ProviderKind::Ollama, Some("m"), Thinking::High).is_empty());
    }
}
