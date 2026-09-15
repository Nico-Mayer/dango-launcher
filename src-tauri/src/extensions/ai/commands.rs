//! What the AI extension offers, and where it comes from.
//!
//! A command is a title and a prompt. The set below ships with Dango, and the
//! configuration file can override any of them, turn one off, or add its own.
//! Nothing here talks to a model: this decides what exists and with what
//! settings, and the extension does the rest.

use std::collections::BTreeMap;

use crate::config::{CommandSettings, Config, ProviderKind, ProviderSettings};

use super::provider::{CliSpec, Request, Thinking};

pub const EXTENSION_ID: &str = "dango.ai";

/// What becomes of the answer once it is complete.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Output {
    /// Left on screen with actions to paste or copy it.
    #[default]
    View,
    Paste,
    Copy,
}

impl Output {
    fn parse(text: &str) -> Option<Self> {
        match text {
            "view" => Some(Output::View),
            "paste" => Some(Output::Paste),
            "copy" => Some(Output::Copy),
            _ => None,
        }
    }
}

/// One command as the user will meet it, after the shipped set and the file
/// have been merged.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AiCommand {
    pub id: String,
    pub title: String,
    pub prompt: String,
    pub provider: Option<String>,
    pub model: Option<String>,
    pub thinking: Option<Thinking>,
    pub output: Output,
}

struct Shipped {
    id: &'static str,
    title: &'static str,
    prompt: &'static str,
}

/// The prompts ask for prose and nothing else, because the detail view shows
/// the answer as written: a model that replies with headings and bullet syntax
/// would show its markdown verbatim.
const SHIPPED: &[Shipped] = &[
    Shipped {
        id: "improve-writing",
        title: "Improve Writing",
        prompt: "Improve the writing below. Keep its meaning, its language, and roughly its length. Reply with the improved text only, as plain prose.\n\n{{ selection }}",
    },
    Shipped {
        id: "fix-spelling",
        title: "Fix Spelling and Grammar",
        prompt: "Correct the spelling and grammar of the text below. Change nothing else. Reply with the corrected text only, as plain prose.\n\n{{ selection }}",
    },
    Shipped {
        id: "make-shorter",
        title: "Make Shorter",
        prompt: "Shorten the text below while keeping every point it makes. Reply with the shorter text only, as plain prose.\n\n{{ selection }}",
    },
    Shipped {
        id: "make-longer",
        title: "Make Longer",
        prompt: "Expand the text below with detail that follows from it. Invent no facts. Reply with the longer text only, as plain prose.\n\n{{ selection }}",
    },
    Shipped {
        id: "make-simpler",
        title: "Make Simpler",
        prompt: "Rewrite the text below in plain, simple language a non-native reader can follow. Keep its meaning. Reply with the simpler text only, as plain prose.\n\n{{ selection }}",
    },
    Shipped {
        id: "make-professional",
        title: "Make Professional",
        prompt: "Rewrite the text below in a professional tone. Keep its meaning and stay concise. Reply with the rewritten text only, as plain prose.\n\n{{ selection }}",
    },
    Shipped {
        id: "summarize",
        title: "Summarize",
        prompt: "Summarize the text below in a few sentences. Reply with the summary only, as plain prose.\n\n{{ selection }}",
    },
    Shipped {
        id: "explain",
        title: "Explain This",
        prompt: "Explain what the text below means and does, for someone seeing it for the first time. Reply as plain prose.\n\n{{ selection }}",
    },
    Shipped {
        id: "translate",
        title: "Translate",
        prompt: "Translate the text below into {{ language }}. Keep its tone. Reply with the translation only, as plain prose.\n\n{{ selection }}",
    },
];

/// The merged command set, plus whatever in the file could not be used. A
/// problem is reported the way a configuration problem is; it never costs the
/// user the commands that are fine.
pub struct Built {
    pub commands: Vec<AiCommand>,
    pub problems: Vec<String>,
}

pub fn build(config: &Config) -> Built {
    let settings = config
        .extensions
        .get(EXTENSION_ID)
        .map(|extension| extension.commands.clone())
        .unwrap_or_default();

    let mut problems = Vec::new();
    let mut commands = Vec::new();
    let mut used: Vec<&str> = Vec::new();

    for shipped in SHIPPED {
        used.push(shipped.id);
        let Some(overrides) = settings.get(shipped.id) else {
            commands.push(AiCommand {
                id: shipped.id.into(),
                title: shipped.title.into(),
                prompt: shipped.prompt.into(),
                provider: None,
                model: None,
                thinking: None,
                output: Output::default(),
            });
            continue;
        };
        if overrides.enabled == Some(false) {
            continue;
        }
        match merge(shipped.id, Some(shipped), overrides) {
            Ok(command) => commands.push(command),
            Err(problem) => problems.push(problem),
        }
    }

    for (id, overrides) in &settings {
        if used.contains(&id.as_str()) || overrides.enabled == Some(false) {
            continue;
        }
        match merge(id, None, overrides) {
            Ok(command) => commands.push(command),
            Err(problem) => problems.push(problem),
        }
    }

    Built { commands, problems }
}

fn merge(
    id: &str,
    shipped: Option<&Shipped>,
    overrides: &CommandSettings,
) -> Result<AiCommand, String> {
    let title = overrides
        .title
        .clone()
        .or_else(|| shipped.map(|shipped| shipped.title.to_string()));
    let prompt = overrides
        .prompt
        .clone()
        .or_else(|| shipped.map(|shipped| shipped.prompt.to_string()));

    let (Some(title), Some(prompt)) = (title, prompt) else {
        return Err(format!(
            "{EXTENSION_ID}: the command '{id}' needs a title and a prompt"
        ));
    };
    if prompt.trim().is_empty() {
        return Err(format!("{EXTENSION_ID}: the command '{id}' has no prompt"));
    }

    let thinking = match &overrides.thinking {
        None => None,
        Some(text) => Some(Thinking::parse(text).ok_or_else(|| {
            format!("{EXTENSION_ID}: the command '{id}' has an unknown thinking level '{text}'")
        })?),
    };
    let output = match &overrides.output {
        None => shipped.map(|_| Output::default()).unwrap_or_default(),
        Some(text) => Output::parse(text).ok_or_else(|| {
            format!("{EXTENSION_ID}: the command '{id}' has an unknown output '{text}'")
        })?,
    };

    Ok(AiCommand {
        id: id.into(),
        title,
        prompt,
        provider: overrides.provider.clone(),
        model: overrides.model.clone(),
        thinking,
        output,
    })
}

/// The providers and the defaults a command falls back to.
#[derive(Debug, Clone, Default)]
pub struct Providers {
    entries: BTreeMap<String, ProviderSettings>,
    default_provider: Option<String>,
    default_thinking: Thinking,
}

/// Why a request could not even be made. Separate from a request that failed,
/// because nothing was sent.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum NotReady {
    #[error("No AI provider is set up yet. Add one to your config file.")]
    NoProvider,
    #[error("No default AI provider is set. Add one to your config file.")]
    NoDefault,
    #[error("This command uses a provider that isn't set up. Check your config file.")]
    UnknownProvider,
    #[error("No model is set for {provider}. Add one to your config file.")]
    NoModel { provider: String },
}

impl Providers {
    pub fn from_config(config: &Config) -> Self {
        let Some(extension) = config.extensions.get(EXTENSION_ID) else {
            return Self::default();
        };
        let default_thinking = config
            .preference(EXTENSION_ID, None, "thinking")
            .and_then(|text| Thinking::parse(&text))
            .unwrap_or_default();
        Self {
            entries: extension.providers.clone(),
            default_provider: config.preference(EXTENSION_ID, None, "provider"),
            default_thinking,
        }
    }

    /// The provider a command uses: the one it names, the configured default,
    /// or the only one there is.
    fn choose(&self, named: Option<&str>) -> Result<(&str, &ProviderSettings), NotReady> {
        if self.entries.is_empty() {
            return Err(NotReady::NoProvider);
        }
        let name = match named.or(self.default_provider.as_deref()) {
            Some(name) => name,
            None if self.entries.len() == 1 => self.entries.keys().next().unwrap().as_str(),
            // Providers exist, so saying none is set up would send the user
            // looking for the wrong thing: what is missing is which one to use.
            None => return Err(NotReady::NoDefault),
        };
        let entry = self.entries.get(name).ok_or(NotReady::UnknownProvider)?;
        // A kind this build does not know, and a command provider with no
        // command, are the same thing from the user's side: an entry that
        // cannot be used.
        let unusable = matches!(entry.kind, ProviderKind::Unknown(_))
            || (entry.kind == ProviderKind::Cli && entry.command.is_none());
        if unusable {
            return Err(NotReady::UnknownProvider);
        }
        let (name, entry) = self.entries.get_key_value(name).unwrap();
        Ok((name.as_str(), entry))
    }

    /// Everything about a request except the prompt and the key, which the
    /// extension supplies: it renders the one and reads the other per request.
    pub fn resolve(&self, command: &AiCommand) -> Result<Request, NotReady> {
        let (name, entry) = self.choose(command.provider.as_deref())?;
        let model = match command.model.clone().or_else(|| entry.model.clone()) {
            Some(model) => model,
            // A program picks its own model unless the arguments name one, so
            // there is nothing to insist on here.
            None if entry.kind == ProviderKind::Cli => String::new(),
            None => {
                return Err(NotReady::NoModel {
                    provider: name.to_string(),
                })
            }
        };
        Ok(Request {
            provider: name.to_string(),
            kind: entry.kind.clone(),
            base_url: entry.base_url.clone(),
            model,
            thinking: command.thinking.unwrap_or(self.default_thinking),
            prompt: String::new(),
            key: None,
            cli: entry.command.as_ref().map(|command| CliSpec {
                command: command.clone(),
                args: entry.args.clone(),
            }),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::templates::Template;

    fn config(json: &str) -> Config {
        Config::parse(json).unwrap()
    }

    fn find<'a>(built: &'a Built, id: &str) -> Option<&'a AiCommand> {
        built.commands.iter().find(|command| command.id == id)
    }

    #[test]
    fn every_shipped_prompt_is_a_template_that_parses() {
        for shipped in SHIPPED {
            let template = Template::parse(shipped.prompt)
                .unwrap_or_else(|error| panic!("{}: {error}", shipped.id));
            assert!(
                template.uses("selection"),
                "{} does not act on the selection",
                shipped.id
            );
        }
    }

    #[test]
    fn the_shipped_set_needs_no_configuration() {
        let built = build(&config("{}"));
        assert_eq!(built.commands.len(), SHIPPED.len());
        assert!(built.problems.is_empty());
        assert_eq!(
            find(&built, "improve-writing").unwrap().title,
            "Improve Writing"
        );
        assert_eq!(
            find(&built, "improve-writing").unwrap().output,
            Output::View
        );
    }

    #[test]
    fn only_translate_asks_the_user_for_anything() {
        let built = build(&config("{}"));
        for command in &built.commands {
            let template = Template::parse(&command.prompt).unwrap();
            let asks = !template.prompts().is_empty();
            assert_eq!(
                asks,
                command.id == "translate",
                "{} asks: {asks}",
                command.id
            );
        }
    }

    #[test]
    fn a_shipped_command_can_be_overridden_in_place() {
        let built = build(&config(
            r#"{ "extensions": { "dango.ai": { "commands": {
                "improve-writing": { "provider": "local", "model": "qwen3:8b", "thinking": "low", "output": "paste" }
            } } } }"#,
        ));
        let command = find(&built, "improve-writing").unwrap();
        assert_eq!(
            command.title, "Improve Writing",
            "the shipped title is kept"
        );
        assert!(
            command.prompt.contains("{{ selection }}"),
            "the shipped prompt is kept"
        );
        assert_eq!(command.provider.as_deref(), Some("local"));
        assert_eq!(command.model.as_deref(), Some("qwen3:8b"));
        assert_eq!(command.thinking, Some(Thinking::Low));
        assert_eq!(command.output, Output::Paste);
    }

    #[test]
    fn a_rewritten_prompt_replaces_the_shipped_one() {
        let built = build(&config(
            r#"{ "extensions": { "dango.ai": { "commands": {
                "summarize": { "prompt": "One sentence only.\n\n{{ selection }}" }
            } } } }"#,
        ));
        assert_eq!(
            find(&built, "summarize").unwrap().prompt,
            "One sentence only.\n\n{{ selection }}"
        );
    }

    #[test]
    fn a_shipped_command_can_be_turned_off() {
        let built = build(&config(
            r#"{ "extensions": { "dango.ai": { "commands": { "make-longer": { "enabled": false } } } } }"#,
        ));
        assert!(find(&built, "make-longer").is_none());
        assert_eq!(built.commands.len(), SHIPPED.len() - 1, "others survived");
        assert!(built.problems.is_empty());
    }

    #[test]
    fn a_title_and_a_prompt_add_a_command_of_the_users_own() {
        let built = build(&config(
            r#"{ "extensions": { "dango.ai": { "commands": {
                "review-rust": { "title": "Review Rust", "prompt": "Review.\n\n{{ selection }}" }
            } } } }"#,
        ));
        let command = find(&built, "review-rust").unwrap();
        assert_eq!(command.title, "Review Rust");
        assert_eq!(command.output, Output::View);
        assert!(built.problems.is_empty());
    }

    #[test]
    fn an_entry_that_is_neither_is_reported_and_skipped() {
        let built = build(&config(
            r#"{ "extensions": { "dango.ai": { "commands": {
                "half-written": { "title": "Half Written" },
                "review-rust": { "title": "Review Rust", "prompt": "Review.\n\n{{ selection }}" }
            } } } }"#,
        ));
        assert!(find(&built, "half-written").is_none());
        assert!(
            find(&built, "review-rust").is_some(),
            "the good one survived"
        );
        assert_eq!(built.problems.len(), 1);
        assert!(built.problems[0].contains("half-written"));
    }

    #[test]
    fn an_unknown_thinking_level_or_output_is_reported_and_skipped() {
        let built = build(&config(
            r#"{ "extensions": { "dango.ai": { "commands": {
                "summarize": { "thinking": "deep" },
                "explain": { "output": "speak" }
            } } } }"#,
        ));
        assert!(find(&built, "summarize").is_none());
        assert!(find(&built, "explain").is_none());
        assert_eq!(built.problems.len(), 2);
    }

    #[test]
    fn an_entry_with_only_a_hotkey_keeps_the_shipped_command() {
        let built = build(&config(
            r#"{ "extensions": { "dango.ai": { "commands": { "explain": { "hotkey": "hyper+e" } } } } }"#,
        ));
        assert!(find(&built, "explain").is_some());
        assert!(built.problems.is_empty());
    }

    fn command(
        provider: Option<&str>,
        model: Option<&str>,
        thinking: Option<Thinking>,
    ) -> AiCommand {
        AiCommand {
            id: "test".into(),
            title: "Test".into(),
            prompt: "{{ selection }}".into(),
            provider: provider.map(str::to_string),
            model: model.map(str::to_string),
            thinking,
            output: Output::View,
        }
    }

    const TWO_PROVIDERS: &str = r#"{ "extensions": { "dango.ai": {
        "preferences": { "provider": "anthropic", "thinking": "medium" },
        "providers": {
            "anthropic": { "kind": "anthropic", "model": "claude-sonnet-5" },
            "local": { "kind": "ollama", "baseUrl": "http://localhost:11434", "model": "qwen3:8b" }
        }
    } } }"#;

    #[test]
    fn a_command_naming_nothing_takes_the_defaults() {
        let providers = Providers::from_config(&config(TWO_PROVIDERS));
        let request = providers.resolve(&command(None, None, None)).unwrap();
        assert_eq!(request.provider, "anthropic");
        assert_eq!(request.model, "claude-sonnet-5");
        assert_eq!(request.thinking, Thinking::Medium);
    }

    #[test]
    fn a_command_naming_a_provider_uses_that_ones_model() {
        let providers = Providers::from_config(&config(TWO_PROVIDERS));
        let request = providers
            .resolve(&command(Some("local"), None, None))
            .unwrap();
        assert_eq!(request.provider, "local");
        assert_eq!(request.model, "qwen3:8b");
        assert_eq!(request.kind, ProviderKind::Ollama);
        assert_eq!(request.base_url.as_deref(), Some("http://localhost:11434"));
    }

    #[test]
    fn a_command_naming_a_model_keeps_the_default_provider() {
        let providers = Providers::from_config(&config(TWO_PROVIDERS));
        let request = providers
            .resolve(&command(None, Some("claude-opus-5"), None))
            .unwrap();
        assert_eq!(request.provider, "anthropic");
        assert_eq!(request.model, "claude-opus-5");
    }

    #[test]
    fn a_command_naming_a_thinking_level_overrides_the_default() {
        let providers = Providers::from_config(&config(TWO_PROVIDERS));
        let request = providers
            .resolve(&command(None, None, Some(Thinking::High)))
            .unwrap();
        assert_eq!(request.thinking, Thinking::High);
    }

    #[test]
    fn the_only_provider_is_the_default_without_saying_so() {
        let providers = Providers::from_config(&config(
            r#"{ "extensions": { "dango.ai": { "providers": {
                "local": { "kind": "ollama", "model": "qwen3:8b" }
            } } } }"#,
        ));
        let request = providers.resolve(&command(None, None, None)).unwrap();
        assert_eq!(request.provider, "local");
        assert_eq!(
            request.thinking,
            Thinking::Off,
            "thinking is off by default"
        );
    }

    #[test]
    fn no_provider_at_all_says_to_set_one_up() {
        let providers = Providers::from_config(&config("{}"));
        let error = providers.resolve(&command(None, None, None)).unwrap_err();
        assert_eq!(
            error.to_string(),
            "No AI provider is set up yet. Add one to your config file."
        );
    }

    #[test]
    fn several_providers_and_no_default_asks_for_a_default() {
        let providers = Providers::from_config(&config(
            r#"{ "extensions": { "dango.ai": { "providers": {
                "anthropic": { "kind": "anthropic", "model": "claude-sonnet-5" },
                "local": { "kind": "ollama", "model": "qwen3:8b" }
            } } } }"#,
        ));
        let error = providers.resolve(&command(None, None, None)).unwrap_err();
        assert_eq!(
            error.to_string(),
            "No default AI provider is set. Add one to your config file."
        );
    }

    #[test]
    fn a_command_naming_its_provider_needs_no_default() {
        let providers = Providers::from_config(&config(
            r#"{ "extensions": { "dango.ai": { "providers": {
                "anthropic": { "kind": "anthropic", "model": "claude-sonnet-5" },
                "local": { "kind": "ollama", "model": "qwen3:8b" }
            } } } }"#,
        ));
        let request = providers
            .resolve(&command(Some("local"), None, None))
            .unwrap();
        assert_eq!(request.provider, "local");
    }

    #[test]
    fn a_provider_that_is_not_configured_says_so() {
        let providers = Providers::from_config(&config(TWO_PROVIDERS));
        let error = providers
            .resolve(&command(Some("openrouter"), None, None))
            .unwrap_err();
        assert_eq!(
            error.to_string(),
            "This command uses a provider that isn't set up. Check your config file."
        );
    }

    #[test]
    fn a_provider_kind_dango_does_not_know_is_not_usable() {
        let providers = Providers::from_config(&config(
            r#"{ "extensions": { "dango.ai": { "providers": {
                "future": { "kind": "mistral", "model": "m" }
            } } } }"#,
        ));
        let error = providers.resolve(&command(None, None, None)).unwrap_err();
        assert_eq!(
            error.to_string(),
            "This command uses a provider that isn't set up. Check your config file."
        );
    }

    #[test]
    fn a_command_provider_resolves_to_its_program() {
        let providers = Providers::from_config(&config(
            r#"{ "extensions": { "dango.ai": { "providers": {
                "claude": { "kind": "cli", "command": "claude", "args": ["-p"] }
            } } } }"#,
        ));
        let request = providers.resolve(&command(None, None, None)).unwrap();
        assert_eq!(request.kind, ProviderKind::Cli);
        let cli = request.cli.expect("the program");
        assert_eq!(cli.command, "claude");
        assert_eq!(cli.args, ["-p"]);
        assert_eq!(request.model, "", "a program needs no model named for it");
    }

    #[test]
    fn a_command_provider_with_no_command_is_not_usable() {
        let providers = Providers::from_config(&config(
            r#"{ "extensions": { "dango.ai": { "providers": { "broken": { "kind": "cli" } } } } }"#,
        ));
        let error = providers.resolve(&command(None, None, None)).unwrap_err();
        assert_eq!(
            error.to_string(),
            "This command uses a provider that isn't set up. Check your config file."
        );
    }

    #[test]
    fn a_provider_with_no_model_anywhere_says_so() {
        let providers = Providers::from_config(&config(
            r#"{ "extensions": { "dango.ai": { "providers": { "local": { "kind": "ollama" } } } } }"#,
        ));
        let error = providers.resolve(&command(None, None, None)).unwrap_err();
        assert_eq!(
            error.to_string(),
            "No model is set for local. Add one to your config file."
        );
    }
}
