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

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::extension::{
    ActionOutcome, CommandDecl, Extension, FormValues, InvocationMode, Manifest, NAMED_ICON,
};
use crate::extensions::clipboard::ClipboardSource;
use crate::invocation::{Command, InvocationContext, Runner};
use crate::protocol::{
    Action, DetailView, FieldKind, FormField, FormView, Modifier, Shortcut, View, ViewTree,
    PROTOCOL_VERSION,
};
use crate::tasks::{TaskHandle, TaskStatus, Tasks};
use crate::templates::{Template, Values};
use crate::text::TextTarget;

use super::commands::{build, AiCommand, Output, Providers, EXTENSION_ID};
use super::provider::{Completions, Request};
use super::stream::{drive, limit_for, Ended};
use super::Keys;

pub const ACTION_RUN: &str = "run";
pub const ACTION_PASTE: &str = "paste";
pub const ACTION_COPY: &str = "copy";
pub const ACTION_BACKGROUND: &str = "background";
pub const ACTION_CANCEL: &str = "cancel";
pub const ACTION_SHOW: &str = "show";

/// Argument fields are prefixed on the way out and stripped on the way back, so
/// a prompt argument cannot collide with a field of the form's own.
const ARGUMENT_PREFIX: &str = "arg:";

const ICON: &str = "sparkles";

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
    tasks: Arc<Tasks>,
    /// Whether the launcher is on screen. An answer that arrives while it is
    /// not is kept rather than pasted into whatever the user is doing.
    visible: Arc<dyn Fn() -> bool + Send + Sync>,
    /// The run in progress and the view it streams into, swapped when the
    /// user opens the same command again.
    live: Mutex<Option<Live>>,
    /// Answers that finished away from their view, by command, until shown.
    kept: Mutex<HashMap<String, String>>,
}

struct Live {
    command_id: String,
    task: TaskHandle,
    ctx: InvocationContext,
    text: String,
}

/// What the extension needs besides the configuration. Kept so a config
/// reload can build the extension again: its commands come from the file, so an
/// edit means a new instance rather than a mutated one.
#[derive(Clone)]
pub struct Parts {
    pub keys: Arc<dyn Keys>,
    pub client: Arc<dyn Completions>,
    pub text: Option<Arc<dyn TextTarget>>,
    pub clipboard: Option<Arc<dyn ClipboardSource>>,
    pub runner: Option<Arc<dyn Runner>>,
    pub tasks: Arc<Tasks>,
    pub visible: Arc<dyn Fn() -> bool + Send + Sync>,
}

impl AiExtension {
    pub fn new(config: &crate::config::Config, parts: &Parts) -> (Self, Vec<String>) {
        let built = build(config);
        let manifest = manifest(&built.commands);
        let parts = parts.clone();
        let extension = Self {
            manifest,
            inner: Arc::new(Inner {
                commands: built.commands,
                providers: Providers::from_config(config),
                keys: parts.keys,
                client: parts.client,
                text: parts.text,
                clipboard: parts.clipboard,
                runner: parts.runner,
                answer: Mutex::new(None),
                tasks: parts.tasks,
                visible: parts.visible,
                live: Mutex::new(None),
                kept: Mutex::new(HashMap::new()),
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
            ACTION_BACKGROUND => ActionOutcome::Back,
            ACTION_CANCEL => self.inner.cancel(),
            ACTION_SHOW => self.inner.show_kept(item_id),
            _ => ActionOutcome::Failed("That action isn't available".into()),
        }
    }
}

const NO_ANSWER: &str = "The model returned no text";
const NO_SELECTION: &str = "Select some text first";
const UNREADABLE_PROMPT: &str = "Fix this command's prompt in your config file";

impl Inner {
    fn find(&self, command_id: &str) -> Option<&AiCommand> {
        self.commands
            .iter()
            .find(|command| command.id == command_id)
    }

    /// Starts a run that no invocation is behind yet, which is what submitting
    /// an argument form does.
    fn start(self: &Arc<Self>, command_id: &str, arguments: FormValues) -> ActionOutcome {
        let Some(runner) = &self.runner else {
            return ActionOutcome::Failed("This command can't run right now".into());
        };
        let title = self.find(command_id).map(|command| command.title.as_str());
        let Some(ctx) = runner.start(title) else {
            return ActionOutcome::Failed("This command can't run right now".into());
        };
        let inner = self.clone();
        let command_id = command_id.to_string();
        std::thread::spawn(move || inner.run(&command_id, arguments, &ctx));
        ActionOutcome::Started
    }

    /// One run, start to finish, on whatever thread is already carrying it. Once
    /// its prompt is ready it is a background task: leaving its view or hiding
    /// the launcher no longer stops it, only Cancel or another command does.
    fn run(&self, command_id: &str, arguments: FormValues, ctx: &InvocationContext) {
        if self.reshow(command_id, ctx) {
            return;
        }
        let Some(command) = self.find(command_id) else {
            ctx.fail("That command is no longer available");
            return;
        };
        let Some(request) = self.prepare(command, arguments, ctx) else {
            return;
        };

        if let Some(previous) = self.live.lock().unwrap().take() {
            previous.task.cancel();
        }
        let task = self.tasks.start(EXTENSION_ID, command_id, &command.title);
        task.report(|state| {
            state.line = format!("Running {}…", command.title);
            state.cancel = Some(ACTION_CANCEL.into());
        });
        let waiting = format!("Waiting for {}…", request.provider);
        *self.live.lock().unwrap() = Some(Live {
            command_id: command_id.to_string(),
            task: task.clone(),
            ctx: ctx.clone(),
            text: String::new(),
        });
        self.stream_into_view(&task, &waiting);

        let limit = limit_for(&request);
        let chunks = self.client.stream(request);
        let cancelled = task.cancelled();
        let ended = drive(&chunks, limit, &|| *cancelled.borrow(), &mut |so_far| {
            self.stream_into_view(&task, so_far)
        });

        let (view, away) = {
            let mut live = self.live.lock().unwrap();
            let Some(view) = live.take_if(|live| live.task.id() == task.id()) else {
                drop(live);
                task.report(|state| state.status = TaskStatus::Cancelled);
                return;
            };
            let away = view.ctx.is_cancelled() || !(self.visible)();
            // Kept before the run is let go, so running the command again finds
            // either the run or its answer, never neither.
            if let (true, Ended::Answer(answer)) = (away, &ended) {
                if command.output != Output::Copy && !answer.trim().is_empty() {
                    self.kept
                        .lock()
                        .unwrap()
                        .insert(command.id.clone(), answer.trim().to_string());
                }
            }
            (view, away)
        };
        // Hidden but held, the view comes back on the next open, so it must not
        // still look like it is loading.
        let held = (away && !view.ctx.is_cancelled()).then_some(&view.ctx);
        match ended {
            Ended::Cancelled => {
                task.report(|state| state.status = TaskStatus::Cancelled);
            }
            Ended::Failed(error) if away => {
                if let Some(held) = held {
                    held.push_view(detail_tree(&view.text, false, false));
                }
                task.report(|state| {
                    state.status = TaskStatus::Failed {
                        sentence: error.to_string(),
                    }
                });
            }
            Ended::Failed(error) => {
                view.ctx.fail(error.to_string());
                task.report(|state| state.status = TaskStatus::Cancelled);
            }
            Ended::Answer(answer) if away => self.finish_away(command, &answer, &task, held),
            // In view the answer reports itself, so the task ends without a toast.
            Ended::Answer(answer) => {
                self.deliver(command.output, &answer, &view.ctx);
                task.report(|state| state.status = TaskStatus::Cancelled);
            }
        }
    }

    /// The same command again while it runs: its view comes back with what has
    /// arrived so far, and the run streams there from now on. Once it finished
    /// away, its kept answer opens instead of a new request.
    fn reshow(&self, command_id: &str, ctx: &InvocationContext) -> bool {
        let mut live = self.live.lock().unwrap();
        if let Some(live) = live.as_mut().filter(|live| live.command_id == command_id) {
            live.ctx = ctx.clone();
            ctx.push_view(running_tree(&live.text));
            return true;
        }
        let Some(answer) = self.kept.lock().unwrap().remove(command_id) else {
            return false;
        };
        self.show_answer(answer, ctx);
        true
    }

    fn show_answer(&self, answer: String, ctx: &InvocationContext) {
        ctx.push_view(detail_tree(&answer, false, true));
        *self.answer.lock().unwrap() = Some(answer);
    }

    fn stream_into_view(&self, task: &TaskHandle, text: &str) {
        let mut live = self.live.lock().unwrap();
        if let Some(live) = live.as_mut().filter(|live| live.task.id() == task.id()) {
            live.text = text.to_string();
            live.ctx.push_view(running_tree(text));
        }
    }

    /// Nobody is looking any more, so nothing is pasted into whatever has the
    /// focus now: the answer was kept for "Show answer", or is copied when that
    /// is the command's choice.
    fn finish_away(
        &self,
        command: &AiCommand,
        answer: &str,
        task: &TaskHandle,
        held: Option<&InvocationContext>,
    ) {
        let answer = answer.trim();
        if let Some(held) = held {
            match answer.is_empty() {
                true => held.push_view(detail_tree("", false, false)),
                false => self.show_answer(answer.to_string(), held),
            }
        }
        if answer.is_empty() {
            task.report(|state| {
                state.status = TaskStatus::Failed {
                    sentence: NO_ANSWER.into(),
                }
            });
            return;
        }
        if command.output == Output::Copy {
            let Some(clipboard) = &self.clipboard else {
                task.report(|state| {
                    state.status = TaskStatus::Failed {
                        sentence: crate::text::TextError::Clipboard.to_string(),
                    }
                });
                return;
            };
            clipboard.set_text(answer);
            task.report(|state| {
                state.status = TaskStatus::Done {
                    sentence: "Copied to clipboard".into(),
                }
            });
            return;
        }
        task.report(|state| {
            state.status = TaskStatus::Done {
                sentence: format!("Answer ready: {}", command.title),
            };
            state.show = Some(ACTION_SHOW.into());
        });
    }

    fn cancel(&self) -> ActionOutcome {
        if let Some(live) = self.live.lock().unwrap().as_ref() {
            live.task.cancel();
        }
        ActionOutcome::Back
    }

    /// Opens an answer that finished while its view was not on screen.
    fn show_kept(&self, command_id: &str) -> ActionOutcome {
        let Some(answer) = self.kept.lock().unwrap().remove(command_id) else {
            return ActionOutcome::Failed(NO_ANSWER.into());
        };
        let title = self.find(command_id).map(|command| command.title.as_str());
        let Some(ctx) = self.runner.as_ref().and_then(|runner| runner.start(title)) else {
            self.kept
                .lock()
                .unwrap()
                .insert(command_id.to_string(), answer);
            return ActionOutcome::Failed("This command can't run right now".into());
        };
        self.show_answer(answer, &ctx);
        ActionOutcome::Started
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

        if request.needs_key() {
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
            return Err(crate::text::TextError::Unavailable.to_string());
        };
        match text.selection() {
            Ok(Some(selection)) if !selection.trim().is_empty() => Ok(Some(selection)),
            Ok(_) => Ok(None),
            Err(error) => Err(error.to_string()),
        }
    }

    fn deliver(&self, output: Output, answer: &str, ctx: &InvocationContext) {
        let answer = answer.trim();
        if answer.is_empty() {
            ctx.fail(NO_ANSWER);
            return;
        }
        *self.answer.lock().unwrap() = Some(answer.to_string());
        match output {
            Output::View => ctx.push_view(detail_tree(answer, false, true)),
            Output::Paste => match self.insert(answer) {
                Ok(()) => ctx.succeed(),
                Err(message) => {
                    ctx.push_view(detail_tree(answer, false, true));
                    ctx.fail(message);
                }
            },
            Output::Copy => match &self.clipboard {
                Some(clipboard) => {
                    clipboard.set_text(answer);
                    ctx.succeed();
                }
                None => {
                    ctx.push_view(detail_tree(answer, false, true));
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
            return Err(crate::text::TextError::Unavailable.to_string());
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
                ..Default::default()
            },
            Action {
                id: ACTION_COPY.into(),
                title: "Copy".into(),
                shortcut: None,
                ..Default::default()
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
            ..Default::default()
        }),
    }
}

/// A request still answering: Enter sends it to the background, and Cancel
/// stops it, so a stray Enter never throws away a paid answer.
fn running_tree(markdown: &str) -> ViewTree {
    let mut tree = detail_tree(markdown, true, false);
    if let View::Detail(detail) = &mut tree.view {
        detail.actions = vec![
            Action {
                id: ACTION_BACKGROUND.into(),
                title: "Continue in background".into(),
                ..Default::default()
            },
            Action {
                id: ACTION_CANCEL.into(),
                title: "Cancel".into(),
                shortcut: Some(Shortcut {
                    key: ".".into(),
                    modifiers: vec![Modifier::Cmd],
                }),
                ..Default::default()
            },
        ];
    }
    tree
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
                    ..Default::default()
                })
                .collect(),
            actions: vec![Action {
                id: ACTION_RUN.into(),
                title: command.title.clone(),
                shortcut: None,
                ..Default::default()
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
        commands: commands
            .iter()
            .map(|command| CommandDecl {
                id: command.id.clone(),
                title: command.title.clone(),
                mode: InvocationMode::View,
                icon: Some(format!("{NAMED_ICON}{ICON}")),
                keywords: vec!["ai".into()],
                alias: None,
            })
            .collect(),
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
        fn formats(&self) -> Option<Vec<String>> {
            Some(vec![])
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
        fn files(&self) -> Option<Vec<String>> {
            None
        }
        fn set_image(&self, _png: &[u8]) {}
        fn set_files(&self, _paths: &[String]) {}
    }

    struct FixedKeys(Option<&'static str>);

    impl Keys for FixedKeys {
        fn key(&self, _provider: &str) -> Result<Option<String>, super::super::AuthError> {
            Ok(self.0.map(str::to_string))
        }
    }

    const LOCAL: &str = r#"
        ai = { providers = { local = { kind = "ollama", model = "qwen3:8b" } } }
    "#;

    const HOSTED: &str = r#"
        [ai]
        providers = { anthropic = { kind = "anthropic", model = "claude-sonnet-5" } }
    "#;

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
            &Parts {
                keys: Arc::new(keys),
                client: client.clone(),
                text: Some(text.clone()),
                clipboard: Some(clipboard.clone()),
                runner: None,
                tasks: crate::tasks::Tasks::new(|_| {}),
                visible: Arc::new(|| true),
            },
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
        assert_eq!(request.model.as_deref(), Some("qwen3:8b"));
        assert_eq!(sink.markdown().last().unwrap(), "Better text.");
    }

    #[test]
    fn the_first_view_names_the_provider_and_offers_the_background() {
        let fixture = fixture(LOCAL, Scripted::answering(&["Better."]), selected("text"));
        let sink = Arc::new(Recorded::default());
        run(&fixture, "improve-writing", sink.clone());

        let View::Detail(first) = &sink.trees.lock().unwrap()[0].view.clone() else {
            panic!("expected a detail view");
        };
        assert_eq!(first.markdown, "Waiting for local…");
        assert!(first.loading, "the first view is not marked loading");
        let ids: Vec<&str> = first.actions.iter().map(|a| a.id.as_str()).collect();
        assert_eq!(
            ids,
            [ACTION_BACKGROUND, ACTION_CANCEL],
            "Enter sends it to the background, never cancels or pastes"
        );
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
        let config = r#"
            [ai]
            providers = { local = { kind = "ollama", model = "qwen3:8b" } }
            commands = { joke = { title = "Joke", prompt = "Tell me a joke." } }
        "#;
        let fixture = fixture(
            config,
            Scripted::answering(&["Why did..."]),
            selected("text"),
        );
        run(&fixture, "joke", Arc::new(Recorded::default()));
        assert_eq!(
            *fixture.text.reads.lock().unwrap(),
            0,
            "the selection was read"
        );
    }

    #[test]
    fn nothing_selected_stops_before_any_request() {
        let fixture = fixture(
            LOCAL,
            Scripted::answering(&["Better."]),
            FakeText::default(),
        );
        let sink = Arc::new(Recorded::default());
        run(&fixture, "improve-writing", sink.clone());

        assert_eq!(sink.failure().as_deref(), Some("Select some text first"));
        assert!(
            fixture.client.last_request().is_none(),
            "a request was made"
        );
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
        assert!(
            fixture.client.last_request().is_none(),
            "asked and ran anyway"
        );
    }

    #[test]
    fn submitting_the_form_renders_the_prompt_with_its_values() {
        let fixture = fixture(LOCAL, Scripted::answering(&["Hola."]), selected("hello"));
        let mut values = FormValues::new();
        values.insert("arg:language".into(), "Spanish".into());
        let ctx = InvocationContext::for_test("translate", Arc::new(Recorded::default()));
        fixture.inner().run("translate", arguments(&values), &ctx);

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
            Some("anthropic needs a key. Add one to auth.toml in your config folder.")
        );
        assert!(fixture.client.last_request().is_none());
    }

    #[test]
    fn a_local_provider_is_asked_without_a_key() {
        let fixture = fixture_with_keys(
            LOCAL,
            Scripted::answering(&["Better."]),
            selected("text"),
            FixedKeys(Some("sk-test")),
        );
        run(&fixture, "improve-writing", Arc::new(Recorded::default()));
        let request = fixture.client.last_request().expect("a request was made");
        assert!(request.key.is_none(), "a key was sent to a local model");
    }

    #[test]
    fn an_agent_program_is_asked_without_a_key_file() {
        let fixture = fixture_with_keys(
            r#"ai = { providers = { claude = { kind = "claude" } } }"#,
            Scripted::answering(&["Better."]),
            selected("text"),
            FixedKeys(None),
        );
        run(&fixture, "improve-writing", Arc::new(Recorded::default()));
        let request = fixture.client.last_request().expect("a request was made");
        assert!(request.key.is_none());
    }

    #[test]
    fn an_openai_server_on_this_machine_is_asked_without_a_key() {
        let fixture = fixture_with_keys(
            r#"
                [ai.providers.studio]
                kind = "openai"
                model = "qwen3"
                base-url = "http://localhost:1234/v1"
            "#,
            Scripted::answering(&["Better."]),
            selected("text"),
            FixedKeys(Some("sk-test")),
        );
        run(&fixture, "improve-writing", Arc::new(Recorded::default()));
        let request = fixture.client.last_request().expect("a request was made");
        assert!(request.key.is_none(), "a key was sent to a local server");
    }

    #[test]
    fn a_command_set_to_paste_puts_the_answer_where_the_user_was() {
        let config = r#"
            [ai]
            providers = { local = { kind = "ollama", model = "qwen3:8b" } }
            commands = { improve-writing = { output = "paste" } }
        "#;
        let fixture = fixture(
            config,
            Scripted::answering(&["Better text."]),
            selected("bad"),
        );
        let sink = Arc::new(Recorded::default());
        run(&fixture, "improve-writing", sink.clone());

        assert_eq!(*fixture.text.inserted.lock().unwrap(), ["Better text."]);
        assert!(sink.succeeded(), "the launcher was left open");
    }

    #[test]
    fn a_command_set_to_copy_leaves_the_answer_on_the_clipboard() {
        let config = r#"
            [ai]
            providers = { local = { kind = "ollama", model = "qwen3:8b" } }
            commands = { summarize = { output = "copy" } }
        "#;
        let fixture = fixture(
            config,
            Scripted::answering(&["Short."]),
            selected("long text"),
        );
        let sink = Arc::new(Recorded::default());
        run(&fixture, "summarize", sink.clone());

        assert_eq!(*fixture.clipboard.written.lock().unwrap(), ["Short."]);
        assert!(sink.succeeded());
        assert!(
            fixture.text.inserted.lock().unwrap().is_empty(),
            "it pasted too"
        );
    }

    #[test]
    fn an_empty_answer_is_neither_pasted_nor_copied() {
        let config = r#"
            [ai]
            providers = { local = { kind = "ollama", model = "qwen3:8b" } }
            commands = { improve-writing = { output = "paste" } }
        "#;
        let fixture = fixture(config, Scripted::answering(&["  "]), selected("bad"));
        let sink = Arc::new(Recorded::default());
        run(&fixture, "improve-writing", sink.clone());

        assert_eq!(
            sink.failure().as_deref(),
            Some("The model returned no text")
        );
        assert!(fixture.text.inserted.lock().unwrap().is_empty());
        assert!(fixture.clipboard.written.lock().unwrap().is_empty());
    }

    #[test]
    fn the_answer_is_pasted_without_surrounding_whitespace() {
        let config = r#"
            [ai]
            providers = { local = { kind = "ollama", model = "qwen3:8b" } }
            commands = { improve-writing = { output = "paste" } }
        "#;
        let fixture = fixture(
            config,
            Scripted::answering(&[
                "
 Better text.",
                "
",
            ]),
            selected("bad"),
        );
        run(&fixture, "improve-writing", Arc::new(Recorded::default()));

        assert_eq!(*fixture.text.inserted.lock().unwrap(), ["Better text."]);
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
            Some("Couldn't reach your local model server")
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

        assert!(
            sink.failure().is_none(),
            "a cancelled run reported a failure"
        );
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

    struct Background {
        fixture: Fixture,
        reports: Arc<Mutex<Vec<crate::tasks::TaskState>>>,
        visible: Arc<std::sync::atomic::AtomicBool>,
        shown: Arc<Recorded>,
        titled: Arc<Mutex<Option<String>>>,
    }

    struct ShowRunner(Arc<Recorded>, Arc<Mutex<Option<String>>>);

    impl Runner for ShowRunner {
        fn start(&self, title: Option<&str>) -> Option<Arc<InvocationContext>> {
            *self.1.lock().unwrap() = title.map(str::to_string);
            Some(Arc::new(InvocationContext::for_test(
                EXTENSION_ID,
                self.0.clone(),
            )))
        }
    }

    fn background(config_json: &str, client: Scripted) -> Background {
        let config = Config::parse(config_json).unwrap();
        let client = Arc::new(client);
        let text = Arc::new(selected("text"));
        let clipboard = Arc::new(FakeClipboard::default());
        let shown = Arc::new(Recorded::default());
        let titled = Arc::new(Mutex::new(None));
        let reports = Arc::new(Mutex::new(Vec::new()));
        let sink = reports.clone();
        let visible = Arc::new(std::sync::atomic::AtomicBool::new(true));
        let probe = visible.clone();
        let (extension, _) = AiExtension::new(
            &config,
            &Parts {
                keys: Arc::new(FixedKeys(Some("sk-test"))),
                client: client.clone(),
                text: Some(text.clone()),
                clipboard: Some(clipboard.clone()),
                runner: Some(Arc::new(ShowRunner(shown.clone(), titled.clone()))),
                tasks: crate::tasks::Tasks::new(move |state| sink.lock().unwrap().push(state)),
                visible: Arc::new(move || probe.load(std::sync::atomic::Ordering::SeqCst)),
            },
        );
        Background {
            fixture: Fixture {
                extension,
                text,
                clipboard,
                client,
            },
            reports,
            visible,
            shown,
            titled,
        }
    }

    impl Background {
        fn last_status(&self) -> Option<crate::tasks::TaskStatus> {
            self.reports
                .lock()
                .unwrap()
                .last()
                .map(|state| state.status.clone())
        }

        fn last_report(&self) -> crate::tasks::TaskState {
            self.reports.lock().unwrap().last().cloned().unwrap()
        }

        fn wait_for_live(&self) {
            self.wait_until(|live| live.is_some());
        }

        fn wait_until(&self, done: impl Fn(&Option<Live>) -> bool) {
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
            while !done(&self.fixture.inner().live.lock().unwrap()) {
                assert!(
                    std::time::Instant::now() < deadline,
                    "the run never got there"
                );
                std::thread::yield_now();
            }
        }
    }

    fn left_behind() -> Arc<Recorded> {
        Arc::new(Recorded {
            cancelled: true,
            ..Default::default()
        })
    }

    #[test]
    fn a_run_left_behind_keeps_its_answer_for_later() {
        let b = background(LOCAL, Scripted::answering(&["Better."]));
        let sink = left_behind();
        run(&b.fixture, "improve-writing", sink.clone());

        let report = b.last_report();
        assert_eq!(
            report.status,
            crate::tasks::TaskStatus::Done {
                sentence: "Answer ready: Improve Writing".into()
            }
        );
        assert_eq!(report.show.as_deref(), Some(ACTION_SHOW));
        assert!(!sink.succeeded() && sink.failure().is_none());

        let outcome =
            b.fixture
                .extension
                .perform_action("improve-writing", ACTION_SHOW, &FormValues::new());
        assert_eq!(outcome, ActionOutcome::Started);
        assert_eq!(
            b.shown.markdown().last().map(String::as_str),
            Some("Better.")
        );
        assert_eq!(
            b.titled.lock().unwrap().as_deref(),
            Some("Improve Writing"),
            "the view is named after the command, not the extension"
        );
    }

    #[test]
    fn a_paste_command_finishing_while_hidden_pastes_nothing() {
        let b = background(
            r#"
            [ai]
            providers = { local = { kind = "ollama", model = "qwen3:8b" } }
            commands = { improve-writing = { output = "paste" } }
            "#,
            Scripted::answering(&["Better."]),
        );
        b.visible.store(false, std::sync::atomic::Ordering::SeqCst);
        let held = Arc::new(Recorded::default());
        run(&b.fixture, "improve-writing", held.clone());

        assert!(b.fixture.text.inserted.lock().unwrap().is_empty());
        assert!(matches!(
            b.last_status(),
            Some(crate::tasks::TaskStatus::Done { .. })
        ));
        let last = held.trees.lock().unwrap().last().cloned().unwrap();
        let View::Detail(detail) = last.view else {
            panic!("the held view shows a detail");
        };
        assert!(!detail.loading, "the view the next open shows is finished");
        assert_eq!(detail.markdown, "Better.");
    }

    #[test]
    fn a_failure_while_hidden_leaves_no_loading_view() {
        let b = background(
            LOCAL,
            Scripted::failing(super::super::provider::AiError::LocalUnreachable),
        );
        b.visible.store(false, std::sync::atomic::Ordering::SeqCst);
        let held = Arc::new(Recorded::default());
        run(&b.fixture, "improve-writing", held.clone());

        assert!(matches!(
            b.last_status(),
            Some(crate::tasks::TaskStatus::Failed { .. })
        ));
        let last = held.trees.lock().unwrap().last().cloned().unwrap();
        let View::Detail(detail) = last.view else {
            panic!("the held view shows a detail");
        };
        assert!(!detail.loading);
    }

    #[test]
    fn the_same_command_again_shows_its_kept_answer_once() {
        let b = background(LOCAL, Scripted::answering(&["Better."]));
        run(&b.fixture, "improve-writing", left_behind());

        let again = Arc::new(Recorded::default());
        run(&b.fixture, "improve-writing", again.clone());
        assert_eq!(again.markdown(), ["Better."]);
        assert_eq!(
            b.fixture.client.seen.lock().unwrap().len(),
            1,
            "no second request"
        );

        let outcome =
            b.fixture
                .extension
                .perform_action("improve-writing", ACTION_SHOW, &FormValues::new());
        assert!(
            matches!(outcome, ActionOutcome::Failed(_)),
            "shown once, then gone"
        );

        run(&b.fixture, "improve-writing", Arc::new(Recorded::default()));
        assert_eq!(b.fixture.client.seen.lock().unwrap().len(), 2);
    }

    #[test]
    fn a_copy_command_finishing_away_still_copies() {
        let b = background(
            r#"
            [ai]
            providers = { local = { kind = "ollama", model = "qwen3:8b" } }
            commands = { summarize = { output = "copy" } }
            "#,
            Scripted::answering(&["Short."]),
        );
        run(&b.fixture, "summarize", left_behind());

        assert_eq!(*b.fixture.clipboard.written.lock().unwrap(), ["Short."]);
        assert_eq!(
            b.last_status(),
            Some(crate::tasks::TaskStatus::Done {
                sentence: "Copied to clipboard".into()
            })
        );
    }

    #[test]
    fn a_paste_command_finishing_in_view_pastes_and_posts_no_result() {
        let b = background(
            r#"
            [ai]
            providers = { local = { kind = "ollama", model = "qwen3:8b" } }
            commands = { improve-writing = { output = "paste" } }
            "#,
            Scripted::answering(&["Better."]),
        );
        run(&b.fixture, "improve-writing", Arc::new(Recorded::default()));

        assert_eq!(*b.fixture.text.inserted.lock().unwrap(), ["Better."]);
        assert_eq!(b.last_status(), Some(crate::tasks::TaskStatus::Cancelled));
    }

    #[test]
    fn cancel_stops_a_running_request() {
        let b = background(LOCAL, Scripted::holding(&["Bet"]));
        let inner = b.fixture.inner().clone();
        let sink = Arc::new(Recorded::default());
        let ctx = InvocationContext::for_test("improve-writing", sink.clone());
        let running =
            std::thread::spawn(move || inner.run("improve-writing", FormValues::new(), &ctx));
        b.wait_for_live();

        let outcome = b.fixture.extension.perform_action(
            "improve-writing",
            ACTION_CANCEL,
            &FormValues::new(),
        );
        assert_eq!(outcome, ActionOutcome::Back);
        running.join().unwrap();
        b.fixture.client.release();

        assert_eq!(b.last_status(), Some(crate::tasks::TaskStatus::Cancelled));
        assert!(!sink.succeeded() && sink.failure().is_none());
        assert!(b.fixture.inner().kept.lock().unwrap().is_empty());
    }

    #[test]
    fn the_same_command_again_shows_the_running_request() {
        let b = background(LOCAL, Scripted::holding(&["Bet"]));
        let inner = b.fixture.inner().clone();
        let first = Arc::new(Recorded::default());
        let ctx = InvocationContext::for_test("improve-writing", first.clone());
        let running =
            std::thread::spawn(move || inner.run("improve-writing", FormValues::new(), &ctx));
        b.wait_until(|live| live.as_ref().is_some_and(|live| live.text == "Bet"));

        let again = Arc::new(Recorded::default());
        run(&b.fixture, "improve-writing", again.clone());
        assert_eq!(again.markdown(), ["Bet"]);
        assert_eq!(
            b.fixture.client.seen.lock().unwrap().len(),
            1,
            "no second request"
        );

        b.fixture.client.release();
        running.join().unwrap();
        assert_eq!(again.markdown().last().map(String::as_str), Some("Bet"));
    }

    #[test]
    fn a_form_does_not_stop_a_running_request_but_another_command_does() {
        let b = background(LOCAL, Scripted::holding(&["Bet"]));
        let inner = b.fixture.inner().clone();
        let ctx = InvocationContext::for_test("improve-writing", Arc::new(Recorded::default()));
        let running =
            std::thread::spawn(move || inner.run("improve-writing", FormValues::new(), &ctx));
        b.wait_for_live();

        run(&b.fixture, "translate", Arc::new(Recorded::default()));
        assert_eq!(
            b.fixture
                .inner()
                .live
                .lock()
                .unwrap()
                .as_ref()
                .map(|live| live.command_id.clone()),
            Some("improve-writing".to_string()),
            "a form waiting for input left the run alone"
        );

        let inner = b.fixture.inner().clone();
        let ctx = InvocationContext::for_test("summarize", Arc::new(Recorded::default()));
        let second = std::thread::spawn(move || inner.run("summarize", FormValues::new(), &ctx));
        running.join().unwrap();
        b.fixture.client.release();
        second.join().unwrap();
        assert!(b
            .reports
            .lock()
            .unwrap()
            .iter()
            .any(|state| state.title == "Improve Writing"
                && state.status == crate::tasks::TaskStatus::Cancelled));
    }
}
