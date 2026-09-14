//! The `dango.ai` extension: selected text in, transformed text out.
//!
//! A command here is a prompt template and a few settings, so the interesting
//! parts are borrowed rather than built: `crate::templates` decides what the
//! prompt needs, `crate::text` reads the selection and puts the answer back, and
//! the invocation channel carries the answer to the screen as it arrives.
//!
//! Both ways of starting a run end in the same place. Invoked from root search
//! or a hotkey, the command owns an invocation and streams into it. Started by
//! submitting an argument form, there is no invocation yet, so one is asked for
//! and the work moves to its own thread; the launcher hears about both the same
//! way.

use std::sync::{Arc, Mutex};

use crate::extension::{
    ActionOutcome, CommandDecl, Extension, FormValues, InvocationMode, Manifest, NAMED_ICON,
};
use crate::extensions::clipboard::ClipboardSource;
use crate::invocation::{Command, InvocationContext};
use crate::protocol::{
    Action, DetailView, FieldKind, FormField, FormView, View, ViewTree, PROTOCOL_VERSION,
};
use crate::templates::{Template, Values};
use crate::text::TextTarget;

use super::commands::{build, AiCommand, Output, Providers, EXTENSION_ID};
use super::provider::{Completions, Request};
use super::stream::{drive, Ended, FIRST_FRAGMENT_TIMEOUT};
use super::Keys;

pub const ACTION_RUN: &str = "run";
pub const ACTION_PASTE: &str = "paste";
pub const ACTION_COPY: &str = "copy";

/// Argument fields are prefixed on the way out and stripped on the way back, so
/// a prompt argument cannot collide with a field of the form's own.
const ARGUMENT_PREFIX: &str = "arg:";

const ICON: &str = "sparkles";

/// Somewhere to start work that is not an invocation: a submitted form has an
/// answer to stream but no command running behind it.
pub trait Runner: Send + Sync {
    fn start(&self) -> Option<Arc<InvocationContext>>;
}

pub struct AiExtension {
    manifest: Manifest,
    inner: Arc<Inner>,
}

struct Inner {
    commands: Vec<AiCommand>,
    providers: Providers,
    keys: Arc<dyn Keys>,
    client: Arc<dyn Completions>,
    /// `None` where text cannot be read or inserted, which leaves the commands
    /// that work from the clipboard alone and stops the rest before they ask a
    /// model anything.
    text: Option<Arc<dyn TextTarget>>,
    clipboard: Option<Arc<dyn ClipboardSource>>,
    runner: Option<Arc<dyn Runner>>,
    /// The last complete answer, so the result view's actions have something to
    /// paste or copy. One answer at a time, because one command runs at a time.
    answer: Mutex<Option<String>>,
}

impl AiExtension {
    pub fn new(
        config: &crate::config::Config,
        keys: Arc<dyn Keys>,
        client: Arc<dyn Completions>,
        text: Option<Arc<dyn TextTarget>>,
        clipboard: Option<Arc<dyn ClipboardSource>>,
        runner: Option<Arc<dyn Runner>>,
    ) -> (Self, Vec<String>) {
        let built = build(config);
        let manifest = manifest(&built.commands);
        let extension = Self {
            manifest,
            inner: Arc::new(Inner {
                commands: built.commands,
                providers: Providers::from_config(config),
                keys,
                client,
                text,
                clipboard,
                runner,
                answer: Mutex::new(None),
            }),
        };
        (extension, built.problems)
    }
}

impl Extension for AiExtension {
    fn manifest(&self) -> &Manifest {
        &self.manifest
    }

    fn command(&self, command_id: &str) -> Option<Arc<dyn Command>> {
        let command = self.inner.find(command_id)?;
        Some(Arc::new(RunCommand {
            inner: self.inner.clone(),
            id: command.id.clone(),
        }))
    }

    fn perform_action(&self, item_id: &str, action_id: &str, values: &FormValues) -> ActionOutcome {
        match action_id {
            ACTION_RUN => self.inner.start(item_id, arguments(values)),
            ACTION_PASTE => self.inner.paste(),
            ACTION_COPY => match self.inner.answer.lock().unwrap().clone() {
                Some(answer) => ActionOutcome::CopyToClipboard(answer),
                None => ActionOutcome::Failed(NO_ANSWER.into()),
            },
            _ => ActionOutcome::Failed("That action isn't available.".into()),
        }
    }
}

const NO_ANSWER: &str = "The model returned no text. Try again.";
const NO_SELECTION: &str = "Select some text first, then run this command.";
const UNREADABLE_PROMPT: &str = "This command's prompt can't be read. Check your config file.";
const WORKING: &str = "Working…";

impl Inner {
    fn find(&self, command_id: &str) -> Option<&AiCommand> {
        self.commands.iter().find(|command| command.id == command_id)
    }

    /// Starts a run that no invocation is behind yet, which is what submitting
    /// an argument form does.
    fn start(self: &Arc<Self>, command_id: &str, arguments: FormValues) -> ActionOutcome {
        let Some(runner) = &self.runner else {
            return ActionOutcome::Failed("This command can't run right now.".into());
        };
        let Some(ctx) = runner.start() else {
            return ActionOutcome::Failed("This command can't run right now.".into());
        };
        let inner = self.clone();
        let command_id = command_id.to_string();
        std::thread::spawn(move || inner.run(&command_id, arguments, &ctx));
        ActionOutcome::Started
    }

    /// One run, start to finish, on whatever thread is already carrying it.
    fn run(&self, command_id: &str, arguments: FormValues, ctx: &InvocationContext) {
        let Some(command) = self.find(command_id) else {
            ctx.fail("This command isn't available any more. Check your config file.");
            return;
        };
        let Some(request) = self.prepare(command, arguments, ctx) else {
            return;
        };

        ctx.push_view(detail_tree(WORKING, true, false));

        let chunks = self.client.stream(request);
        let ended = drive(
            &chunks,
            FIRST_FRAGMENT_TIMEOUT,
            &|| ctx.is_cancelled(),
            &mut |so_far| ctx.push_view(detail_tree(so_far, true, false)),
        );

        match ended {
            Ended::Cancelled => {}
            Ended::Failed(error) => ctx.fail(error.to_string()),
            Ended::Answer(answer) if answer.trim().is_empty() => ctx.fail(NO_ANSWER),
            Ended::Answer(answer) => self.deliver(command.output, answer, ctx),
        }
    }

    /// Everything that has to be true before a model is asked anything: the
    /// provider resolves, the prompt reads, the user has been asked for what it
    /// needs, there is a selection to work on, and there is a key.
    fn prepare(
        &self,
        command: &AiCommand,
        arguments: FormValues,
        ctx: &InvocationContext,
    ) -> Option<Request> {
        let mut request = match self.providers.resolve(command) {
            Ok(request) => request,
            Err(error) => {
                ctx.fail(error.to_string());
                return None;
            }
        };

        let Ok(template) = Template::parse(&command.prompt) else {
            ctx.fail(UNREADABLE_PROMPT);
            return None;
        };

        if arguments.is_empty() && !template.prompts().is_empty() {
            ctx.push_view(argument_tree(command, &template));
            return None;
        }

        let mut values = Values {
            arguments,
            ..Default::default()
        };
        values.query = values.arguments.remove("query");

        if template.uses("selection") {
            match self.selection() {
                Ok(Some(selection)) => values.selection = Some(selection),
                Ok(None) => {
                    ctx.fail(NO_SELECTION);
                    return None;
                }
                Err(message) => {
                    ctx.fail(message);
                    return None;
                }
            }
        }
        if template.uses("clipboard") {
            values.clipboard = self.text.as_ref().and_then(|text| text.clipboard_text());
        }

        request.prompt = match template.render(&values) {
            Ok(rendered) => rendered.text,
            Err(_) => {
                ctx.fail(UNREADABLE_PROMPT);
                return None;
            }
        };

        if !request.is_local() {
            match self.keys.key(&request.provider) {
                Ok(Some(key)) => request.key = Some(key),
                Ok(None) => {
                    ctx.fail(
                        super::provider::AiError::MissingKey {
                            provider: request.provider.clone(),
                        }
                        .to_string(),
                    );
                    return None;
                }
                Err(error) => {
                    crate::log::append(&format!("ai: {error}"));
                    ctx.fail(super::provider::AiError::KeyFileUnreadable.to_string());
                    return None;
                }
            }
        }

        Some(request)
    }

    /// Reading the selection is only attempted for a prompt that asks for it,
    /// because on Windows it costs a clipboard round trip.
    fn selection(&self) -> Result<Option<String>, String> {
        let Some(text) = &self.text else {
            return Err(crate::text::TextError::PermissionMissing.to_string());
        };
        match text.selection() {
            Ok(Some(selection)) if !selection.trim().is_empty() => Ok(Some(selection)),
            Ok(_) => Ok(None),
            Err(error) => Err(error.to_string()),
        }
    }

    fn deliver(&self, output: Output, answer: String, ctx: &InvocationContext) {
        *self.answer.lock().unwrap() = Some(answer.clone());
        match output {
            Output::View => ctx.push_view(detail_tree(&answer, false, true)),
            Output::Paste => match self.insert(&answer) {
                Ok(()) => ctx.succeed(),
                Err(message) => {
                    ctx.push_view(detail_tree(&answer, false, true));
                    ctx.fail(message);
                }
            },
            Output::Copy => match &self.clipboard {
                Some(clipboard) => {
                    clipboard.set_text(&answer);
                    ctx.succeed();
                }
                None => {
                    ctx.push_view(detail_tree(&answer, false, true));
                    ctx.fail(crate::text::TextError::Clipboard.to_string());
                }
            },
        }
    }

    fn paste(&self) -> ActionOutcome {
        let Some(answer) = self.answer.lock().unwrap().clone() else {
            return ActionOutcome::Failed(NO_ANSWER.into());
        };
        match self.insert(&answer) {
            Ok(()) => ActionOutcome::Done,
            Err(message) => ActionOutcome::Failed(message),
        }
    }

    fn insert(&self, answer: &str) -> Result<(), String> {
        let Some(text) = &self.text else {
            return Err(crate::text::TextError::PermissionMissing.to_string());
        };
        text.insert(answer, None).map_err(|error| error.to_string())
    }
}

struct RunCommand {
    inner: Arc<Inner>,
    id: String,
}

impl Command for RunCommand {
    fn invoke(&self, ctx: &InvocationContext) {
        self.inner.run(&self.id, FormValues::new(), ctx);
    }
}

fn arguments(values: &FormValues) -> FormValues {
    values
        .iter()
        .filter_map(|(key, value)| {
            key.strip_prefix(ARGUMENT_PREFIX)
                .map(|name| (name.to_string(), value.clone()))
        })
        .collect()
}

/// The answer, or the fact that there is not one yet. While it is still
/// arriving there is nothing to act on, so the actions appear with the finished
/// answer rather than inviting a paste of half of it.
fn detail_tree(markdown: &str, loading: bool, actionable: bool) -> ViewTree {
    let actions = match actionable {
        true => vec![
            Action {
                id: ACTION_PASTE.into(),
                title: "Paste".into(),
                shortcut: None,
            },
            Action {
                id: ACTION_COPY.into(),
                title: "Copy".into(),
                shortcut: None,
            },
        ],
        false => Vec::new(),
    };
    ViewTree {
        protocol_version: PROTOCOL_VERSION,
        view: View::Detail(DetailView {
            markdown: markdown.into(),
            loading,
            actions,
        }),
    }
}

fn argument_tree(command: &AiCommand, template: &Template) -> ViewTree {
    ViewTree {
        protocol_version: PROTOCOL_VERSION,
        view: View::Form(FormView {
            item_id: Some(command.id.clone()),
            fields: template
                .prompts()
                .iter()
                .map(|name| FormField {
                    id: format!("{ARGUMENT_PREFIX}{name}"),
                    label: name.clone(),
                    kind: FieldKind::Text,
                    value: None,
                })
                .collect(),
            actions: vec![Action {
                id: ACTION_RUN.into(),
                title: command.title.clone(),
                shortcut: None,
            }],
        }),
    }
}

pub(crate) fn manifest(commands: &[AiCommand]) -> Manifest {
    Manifest {
        manifest_version: 1,
        id: EXTENSION_ID.into(),
        name: "AI".into(),
        icon: Some(format!("{NAMED_ICON}{ICON}")),
        tint: Some("purple".into()),
        commands: commands
            .iter()
            .map(|command| CommandDecl {
                id: command.id.clone(),
                title: command.title.clone(),
                mode: InvocationMode::View,
                subtitle: Some("AI".into()),
                icon: Some(format!("{NAMED_ICON}{ICON}")),
                keywords: vec!["ai".into()],
                alias: None,
            })
            .collect(),
        preferences: vec![],
        root_items: false,
        services: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::extensions::ai::provider::testing::Scripted;
    use crate::extensions::ai::provider::AiError;
    use crate::invocation::{Outcome, Sink};
    use crate::text::TextError;

    #[derive(Default)]
    struct Recorded {
        trees: Mutex<Vec<ViewTree>>,
        outcome: Mutex<Option<Outcome>>,
        cancelled: bool,
    }

    impl Sink for Recorded {
        fn push_view(&self, tree: ViewTree) {
            self.trees.lock().unwrap().push(tree);
        }
        fn finish(&self, outcome: Outcome) {
            *self.outcome.lock().unwrap() = Some(outcome);
        }
        fn superseded(&self) -> bool {
            self.cancelled
        }
    }

    impl Recorded {
        fn markdown(&self) -> Vec<String> {
            self.trees
                .lock()
                .unwrap()
                .iter()
                .filter_map(|tree| match &tree.view {
                    View::Detail(detail) => Some(detail.markdown.clone()),
                    _ => None,
                })
                .collect()
        }

        fn last(&self) -> ViewTree {
            self.trees.lock().unwrap().last().unwrap().clone()
        }

        fn failure(&self) -> Option<String> {
            match self.outcome.lock().unwrap().clone() {
                Some(Outcome::Failure(message)) => Some(message),
                _ => None,
            }
        }

        fn succeeded(&self) -> bool {
            matches!(*self.outcome.lock().unwrap(), Some(Outcome::Success))
        }
    }

    #[derive(Default)]
    struct FakeText {
        selection: Option<String>,
        clipboard: Option<String>,
        error: Option<TextError>,
        inserted: Mutex<Vec<String>>,
        reads: Mutex<usize>,
    }

    impl TextTarget for FakeText {
        fn insert(&self, text: &str, _caret: Option<usize>) -> Result<(), TextError> {
            self.inserted.lock().unwrap().push(text.to_string());
            Ok(())
        }
        fn expand(&self, _b: usize, _t: &str, _c: Option<usize>) -> Result<(), TextError> {
            Ok(())
        }
        fn paste_content(
            &self,
            _content: &crate::extensions::clipboard::Content,
        ) -> Result<(), TextError> {
            Ok(())
        }
        fn selection(&self) -> Result<Option<String>, TextError> {
            *self.reads.lock().unwrap() += 1;
            match &self.error {
                Some(TextError::PermissionMissing) => Err(TextError::PermissionMissing),
                Some(_) => Err(TextError::Clipboard),
                None => Ok(self.selection.clone()),
            }
        }
        fn clipboard_text(&self) -> Option<String> {
            self.clipboard.clone()
        }
    }

    #[derive(Default)]
    struct FakeClipboard {
        written: Mutex<Vec<String>>,
    }

    impl ClipboardSource for FakeClipboard {
        fn formats(&self) -> Vec<String> {
            vec![]
        }
        fn text(&self) -> Option<String> {
            None
        }
        fn image(&self) -> Option<Vec<u8>> {
            None
        }
        fn set_text(&self, text: &str) {
            self.written.lock().unwrap().push(text.to_string());
        }
        fn set_image(&self, _png: &[u8]) {}
    }

    struct FixedKeys(Option<&'static str>);

    impl Keys for FixedKeys {
        fn key(&self, _provider: &str) -> Result<Option<String>, super::super::AuthError> {
            Ok(self.0.map(str::to_string))
        }
    }

    const LOCAL: &str = r#"{ "extensions": { "dango.ai": { "providers": {
        "local": { "kind": "ollama", "model": "qwen3:8b" }
    } } } }"#;

    const HOSTED: &str = r#"{ "extensions": { "dango.ai": { "providers": {
        "anthropic": { "kind": "anthropic", "model": "claude-sonnet-5" }
    } } } }"#;

    struct Fixture {
        extension: AiExtension,
        text: Arc<FakeText>,
        clipboard: Arc<FakeClipboard>,
        client: Arc<Scripted>,
    }

    fn fixture(config_json: &str, client: Scripted, text: FakeText) -> Fixture {
        fixture_with_keys(config_json, client, text, FixedKeys(Some("sk-test")))
    }

    fn fixture_with_keys(
        config_json: &str,
        client: Scripted,
        text: FakeText,
        keys: FixedKeys,
    ) -> Fixture {
        let config = Config::parse(config_json).unwrap();
        let client = Arc::new(client);
        let text = Arc::new(text);
        let clipboard = Arc::new(FakeClipboard::default());
        let (extension, problems) = AiExtension::new(
            &config,
            Arc::new(keys),
            client.clone(),
            Some(text.clone()),
            Some(clipboard.clone()),
            None,
        );
        assert!(problems.is_empty(), "{problems:?}");
        Fixture {
            extension,
            text,
            clipboard,
            client,
        }
    }

    fn run(fixture: &Fixture, command_id: &str, sink: Arc<Recorded>) {
        let ctx = InvocationContext::for_test(command_id, sink);
        fixture.inner().run(command_id, FormValues::new(), &ctx);
    }

    impl Fixture {
        fn inner(&self) -> &Arc<Inner> {
            &self.extension.inner
        }
    }

    fn selected(text: &str) -> FakeText {
        FakeText {
            selection: Some(text.to_string()),
            ..Default::default()
        }
    }

    #[test]
    fn the_selection_is_put_into_the_prompt() {
        let fixture = fixture(
            LOCAL,
            Scripted::answering(&["Better text."]),
            selected("bad text"),
        );
        let sink = Arc::new(Recorded::default());
        run(&fixture, "improve-writing", sink.clone());

        let request = fixture.client.last_request().expect("a request was made");
        assert!(
            request.prompt.contains("bad text"),
            "the selection is missing: {}",
            request.prompt
        );
        assert_eq!(request.model, "qwen3:8b");
        assert_eq!(sink.markdown().last().unwrap(), "Better text.");
    }

    #[test]
    fn the_first_view_says_it_is_working_and_carries_no_actions() {
        let fixture = fixture(LOCAL, Scripted::answering(&["Better."]), selected("text"));
        let sink = Arc::new(Recorded::default());
        run(&fixture, "improve-writing", sink.clone());

        let View::Detail(first) = &sink.trees.lock().unwrap()[0].view.clone() else {
            panic!("expected a detail view");
        };
        assert_eq!(first.markdown, "Working…");
        assert!(first.loading, "the first view is not marked loading");
        assert!(first.actions.is_empty(), "a half-written answer offers actions");
    }

    #[test]
    fn the_finished_answer_offers_paste_and_copy() {
        let fixture = fixture(LOCAL, Scripted::answering(&["Better."]), selected("text"));
        let sink = Arc::new(Recorded::default());
        run(&fixture, "improve-writing", sink.clone());

        let View::Detail(last) = sink.last().view else {
            panic!("expected a detail view");
        };
        assert!(!last.loading);
        let titles: Vec<String> = last.actions.iter().map(|a| a.title.clone()).collect();
        assert_eq!(titles, ["Paste", "Copy"]);
    }

    #[test]
    fn a_prompt_that_does_not_ask_for_the_selection_never_reads_it() {
        let config = r#"{ "extensions": { "dango.ai": {
            "providers": { "local": { "kind": "ollama", "model": "qwen3:8b" } },
            "commands": { "joke": { "title": "Joke", "prompt": "Tell me a joke." } }
        } } }"#;
        let fixture = fixture(config, Scripted::answering(&["Why did..."]), selected("text"));
        run(&fixture, "joke", Arc::new(Recorded::default()));
        assert_eq!(*fixture.text.reads.lock().unwrap(), 0, "the selection was read");
    }

    #[test]
    fn nothing_selected_stops_before_any_request() {
        let fixture = fixture(LOCAL, Scripted::answering(&["Better."]), FakeText::default());
        let sink = Arc::new(Recorded::default());
        run(&fixture, "improve-writing", sink.clone());

        assert_eq!(
            sink.failure().as_deref(),
            Some("Select some text first, then run this command.")
        );
        assert!(fixture.client.last_request().is_none(), "a request was made");
    }

    #[test]
    fn a_prompt_with_an_argument_asks_before_it_runs() {
        let fixture = fixture(LOCAL, Scripted::answering(&["Hola."]), selected("hello"));
        let sink = Arc::new(Recorded::default());
        run(&fixture, "translate", sink.clone());

        let View::Form(form) = sink.last().view else {
            panic!("expected a form");
        };
        assert_eq!(form.item_id.as_deref(), Some("translate"));
        assert_eq!(form.fields[0].id, "arg:language");
        assert_eq!(form.actions[0].id, ACTION_RUN);
        assert!(fixture.client.last_request().is_none(), "asked and ran anyway");
    }

    #[test]
    fn submitting_the_form_renders_the_prompt_with_its_values() {
        let fixture = fixture(LOCAL, Scripted::answering(&["Hola."]), selected("hello"));
        let mut values = FormValues::new();
        values.insert("arg:language".into(), "Spanish".into());
        let ctx = InvocationContext::for_test("translate", Arc::new(Recorded::default()));
        fixture
            .inner()
            .run("translate", arguments(&values), &ctx);

        let request = fixture.client.last_request().expect("a request was made");
        assert!(request.prompt.contains("Spanish"), "{}", request.prompt);
        assert!(request.prompt.contains("hello"), "{}", request.prompt);
    }

    #[test]
    fn a_hosted_provider_with_no_key_stops_before_any_request() {
        let fixture = fixture_with_keys(
            HOSTED,
            Scripted::answering(&["Better."]),
            selected("text"),
            FixedKeys(None),
        );
        let sink = Arc::new(Recorded::default());
        run(&fixture, "improve-writing", sink.clone());

        assert_eq!(
            sink.failure().as_deref(),
            Some("anthropic needs a key. Add one to auth.json in your config folder.")
        );
        assert!(fixture.client.last_request().is_none());
    }

    #[test]
    fn a_local_provider_is_asked_without_a_key() {
        let fixture = fixture_with_keys(
            LOCAL,
            Scripted::answering(&["Better."]),
            selected("text"),
            FixedKeys(None),
        );
        run(&fixture, "improve-writing", Arc::new(Recorded::default()));
        let request = fixture.client.last_request().expect("a request was made");
        assert!(request.key.is_none(), "a key was sent to a local model");
    }

    #[test]
    fn a_command_set_to_paste_puts_the_answer_where_the_user_was() {
        let config = r#"{ "extensions": { "dango.ai": {
            "providers": { "local": { "kind": "ollama", "model": "qwen3:8b" } },
            "commands": { "improve-writing": { "output": "paste" } }
        } } }"#;
        let fixture = fixture(config, Scripted::answering(&["Better text."]), selected("bad"));
        let sink = Arc::new(Recorded::default());
        run(&fixture, "improve-writing", sink.clone());

        assert_eq!(*fixture.text.inserted.lock().unwrap(), ["Better text."]);
        assert!(sink.succeeded(), "the launcher was left open");
    }

    #[test]
    fn a_command_set_to_copy_leaves_the_answer_on_the_clipboard() {
        let config = r#"{ "extensions": { "dango.ai": {
            "providers": { "local": { "kind": "ollama", "model": "qwen3:8b" } },
            "commands": { "summarize": { "output": "copy" } }
        } } }"#;
        let fixture = fixture(config, Scripted::answering(&["Short."]), selected("long text"));
        let sink = Arc::new(Recorded::default());
        run(&fixture, "summarize", sink.clone());

        assert_eq!(*fixture.clipboard.written.lock().unwrap(), ["Short."]);
        assert!(sink.succeeded());
        assert!(fixture.text.inserted.lock().unwrap().is_empty(), "it pasted too");
    }

    #[test]
    fn an_empty_answer_is_neither_pasted_nor_copied() {
        let config = r#"{ "extensions": { "dango.ai": {
            "providers": { "local": { "kind": "ollama", "model": "qwen3:8b" } },
            "commands": { "improve-writing": { "output": "paste" } }
        } } }"#;
        let fixture = fixture(config, Scripted::answering(&["  "]), selected("bad"));
        let sink = Arc::new(Recorded::default());
        run(&fixture, "improve-writing", sink.clone());

        assert_eq!(
            sink.failure().as_deref(),
            Some("The model returned no text. Try again.")
        );
        assert!(fixture.text.inserted.lock().unwrap().is_empty());
        assert!(fixture.clipboard.written.lock().unwrap().is_empty());
    }

    #[test]
    fn the_result_views_actions_paste_and_copy_the_answer() {
        let fixture = fixture(LOCAL, Scripted::answering(&["Better."]), selected("bad"));
        run(&fixture, "improve-writing", Arc::new(Recorded::default()));

        assert_eq!(
            fixture
                .extension
                .perform_action("", ACTION_COPY, &FormValues::new()),
            ActionOutcome::CopyToClipboard("Better.".into())
        );
        assert_eq!(
            fixture
                .extension
                .perform_action("", ACTION_PASTE, &FormValues::new()),
            ActionOutcome::Done
        );
        assert_eq!(*fixture.text.inserted.lock().unwrap(), ["Better."]);
    }

    #[test]
    fn a_failed_request_reports_the_providers_message() {
        let fixture = fixture(
            LOCAL,
            Scripted::failing(AiError::LocalUnreachable),
            selected("text"),
        );
        let sink = Arc::new(Recorded::default());
        run(&fixture, "improve-writing", sink.clone());
        assert_eq!(
            sink.failure().as_deref(),
            Some("The local model couldn't be reached. Check that your model server is running.")
        );
    }

    #[test]
    fn a_cancelled_run_shows_nothing_further() {
        let fixture = fixture(LOCAL, Scripted::answering(&["Better."]), selected("text"));
        let sink = Arc::new(Recorded {
            cancelled: true,
            ..Default::default()
        });
        let ctx = InvocationContext::for_test("improve-writing", sink.clone());
        fixture
            .inner()
            .run("improve-writing", FormValues::new(), &ctx);

        assert!(sink.failure().is_none(), "a cancelled run reported a failure");
        assert!(!sink.succeeded());
    }

    #[test]
    fn the_manifest_declares_every_built_command() {
        let fixture = fixture(LOCAL, Scripted::answering(&[]), FakeText::default());
        let manifest = fixture.extension.manifest();
        assert_eq!(manifest.id, EXTENSION_ID);
        assert_eq!(manifest.name, "AI");
        assert!(manifest.validate().is_ok());
        assert!(manifest
            .commands
            .iter()
            .any(|command| command.title == "Improve Writing"));
        assert!(manifest
            .commands
            .iter()
            .all(|command| command.mode == InvocationMode::View));
    }
}
