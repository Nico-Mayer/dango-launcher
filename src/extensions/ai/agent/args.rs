//! What each supported CLI is told, beyond the prompt it reads on stdin.
//!
//! The fixed flags turn off everything that makes the program an agent rather
//! than a model: tools, MCP servers, hooks, plugins, skills, context files, and
//! saved sessions. They are the reason each CLI is its own kind.

use crate::config::{ProviderKind, Thinking};

/// One CLI's spelling of everything Dango asks of it.
pub struct Cli {
    /// The program run when no path is configured.
    pub program: &'static str,
    fixed: &'static [&'static str],
    model_flag: &'static str,
    effort_flag: &'static str,
    /// The level that turns thinking off. A CLI without one keeps its default.
    off: Option<&'static str>,
    pub environment: &'static [(&'static str, &'static str)],
}

const CLAUDE: Cli = Cli {
    program: "claude",
    fixed: &[
        "-p",
        "--output-format",
        "text",
        "--safe-mode",
        "--tools",
        "",
        "--no-session-persistence",
    ],
    model_flag: "--model",
    effort_flag: "--effort",
    off: None,
    // An update started by a run would be cut off when the run ends.
    environment: &[("DISABLE_AUTOUPDATER", "1")],
};

const PI: Cli = Cli {
    program: "pi",
    fixed: &[
        "-p",
        "--no-tools",
        "--no-extensions",
        "--no-mcp",
        "--no-skills",
        "--no-prompt-templates",
        "--no-context-files",
        "--no-session",
    ],
    model_flag: "--model",
    effort_flag: "--thinking",
    // Told explicitly, or pi thinks at whatever level the user set for it.
    off: Some("off"),
    environment: &[],
};

pub fn cli(kind: ProviderKind) -> Option<&'static Cli> {
    match kind {
        ProviderKind::Claude => Some(&CLAUDE),
        ProviderKind::Pi => Some(&PI),
        ProviderKind::Anthropic | ProviderKind::Openai | ProviderKind::Ollama => None,
    }
}

impl Cli {
    pub fn arguments(&self, model: Option<&str>, thinking: Thinking) -> Vec<String> {
        let mut args: Vec<String> = self.fixed.iter().map(|arg| arg.to_string()).collect();
        if let Some(model) = model {
            args.extend([self.model_flag.to_string(), model.to_string()]);
        }
        let level = match thinking {
            Thinking::Off => self.off,
            Thinking::Low => Some("low"),
            Thinking::Medium => Some("medium"),
            Thinking::High => Some("high"),
        };
        if let Some(level) = level {
            args.extend([self.effort_flag.to_string(), level.to_string()]);
        }
        args
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tail(args: &[String], cli: &Cli) -> Vec<String> {
        assert_eq!(
            &args[..cli.fixed.len()],
            cli.fixed,
            "the fixed flags come first"
        );
        args[cli.fixed.len()..].to_vec()
    }

    #[test]
    fn claude_runs_with_its_agent_setup_off() {
        let args = CLAUDE.arguments(None, Thinking::Off);
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
        let args = CLAUDE.arguments(Some("sonnet"), Thinking::Off);
        assert_eq!(tail(&args, &CLAUDE), ["--model", "sonnet"]);
        for (thinking, level) in [
            (Thinking::Low, "low"),
            (Thinking::Medium, "medium"),
            (Thinking::High, "high"),
        ] {
            let args = CLAUDE.arguments(None, thinking);
            assert_eq!(tail(&args, &CLAUDE), ["--effort", level]);
        }
    }

    #[test]
    fn pi_runs_with_its_agent_setup_off() {
        let args = PI.arguments(None, Thinking::Low);
        assert_eq!(tail(&args, &PI), ["--thinking", "low"]);
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
        let args = PI.arguments(Some("gpt-5"), Thinking::Off);
        assert_eq!(tail(&args, &PI), ["--model", "gpt-5", "--thinking", "off"]);
        let args = PI.arguments(None, Thinking::High);
        assert_eq!(tail(&args, &PI), ["--thinking", "high"]);
    }

    #[test]
    fn claude_runs_without_its_auto_updater() {
        assert_eq!(CLAUDE.environment, [("DISABLE_AUTOUPDATER", "1")]);
        assert!(PI.environment.is_empty());
    }

    #[test]
    fn only_agent_kinds_have_a_cli() {
        assert_eq!(
            cli(ProviderKind::Claude).map(|cli| cli.program),
            Some("claude")
        );
        assert_eq!(cli(ProviderKind::Pi).map(|cli| cli.program), Some("pi"));
        assert!(cli(ProviderKind::Ollama).is_none());
    }
}
