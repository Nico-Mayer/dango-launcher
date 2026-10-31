use std::collections::HashMap;

use crate::extension::{ActionOutcome, FormValues};
use crate::protocol::{Action, FieldKind, Filtering, ListItem, View, ViewTree, PROTOCOL_VERSION};

use super::effect::{ActionRequest, Effect, Origin};
use super::input::{Chord, Event, Input, Intent, ResultItem, TemplateInspection};

/// All interaction state of the launcher. A pure reducer: input goes in,
/// effects for the shell come out, and `snapshot` says what to draw.
pub struct Launcher {
    pub(super) development: bool,
    pub(super) activations: u64,
    pub(super) probe: Option<u64>,
    revisions: u64,
    pub(super) query: String,
    pub(super) query_revision: u64,
    pub(super) results: Vec<ResultItem>,
    /// What the keyboard or a click last picked. Results arriving without it
    /// fall back to the first row instead of leaving nothing selected.
    picked: Option<String>,
    pub(super) stack: Vec<PushedView>,
    /// The extension whose command pushed what is on the stack, so an action
    /// chosen inside its view goes back to it.
    owner: Option<String>,
    /// The run whose tree is on top. A streaming command replaces its own view
    /// as the answer grows rather than pushing another.
    shown_invocation: Option<u64>,
    last_invocation: Option<u64>,
    /// Cancelling a streaming command does not recall a tree already in
    /// flight, which would otherwise land as a new view after the user left.
    /// Invocation ids only go up, so one high-water mark covers every run at
    /// or before the one abandoned.
    abandoned_invocation: Option<u64>,
    panel: Option<PanelState>,
    pub(super) failure: Option<String>,
    /// The title of a command invoked from root that has neither finished nor
    /// shown a view yet.
    pub(super) working: Option<String>,
    pub(super) protocol_error: bool,
}

pub(super) struct PushedView {
    pub(super) tree: ViewTree,
    pub(super) query: String,
    pub(super) query_revision: u64,
    picked: Option<String>,
    pub(super) form: FormState,
}

#[derive(Default)]
pub(super) struct FormState {
    pub(super) values: FormValues,
    pub(super) inspections: HashMap<String, TemplateInspection>,
    pub(super) revision: u64,
}

struct PanelState {
    picked: Option<String>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Mode {
    Error,
    Panel,
    Root,
    List,
    Detail,
    Form,
}

impl Launcher {
    pub fn new(development: bool) -> Self {
        Self {
            development,
            activations: 0,
            probe: None,
            revisions: 0,
            query: String::new(),
            query_revision: 0,
            results: Vec::new(),
            picked: None,
            stack: Vec::new(),
            owner: None,
            shown_invocation: None,
            last_invocation: None,
            abandoned_invocation: None,
            panel: None,
            failure: None,
            working: None,
            protocol_error: false,
        }
    }

    pub fn apply(&mut self, input: Input) -> Vec<Effect> {
        let mut effects = Vec::new();
        match input {
            Input::Intent(intent) => self.intent(intent, &mut effects),
            Input::Event(event) => self.event(event, &mut effects),
        }
        if self.panel.is_some() && self.available_actions().is_empty() {
            self.panel = None;
        }
        effects
    }

    fn intent(&mut self, intent: Intent, fx: &mut Vec<Effect>) {
        match intent {
            Intent::Ready => {
                fx.push(Effect::Search(self.query.clone()));
                fx.push(Effect::FinishWarmup);
            }
            Intent::Painted { probe } => fx.push(Effect::ReportPaint(probe)),
            Intent::QueryChanged { text } => self.query_changed(text, fx),
            Intent::FieldChanged { id, value } => self.field_changed(id, value, fx),
            Intent::Key { chord } => self.key(&chord, fx),
            Intent::Activate { id } => self.activate(&id, fx),
            Intent::TogglePanel => {
                if self.panel.is_some() {
                    self.panel = None;
                } else if self.mode() != Mode::Error {
                    self.open_panel();
                }
            }
            Intent::ClosePanel => self.panel = None,
            Intent::Blur => fx.push(Effect::Dismiss),
        }
    }

    fn event(&mut self, event: Event, fx: &mut Vec<Effect>) {
        match event {
            Event::Results { query, items } => {
                // A late answer for an older query was cancelled and must not show.
                if query == self.query {
                    self.results = items;
                }
            }
            Event::Rendered {
                owner,
                invocation,
                tree,
            } => self.rendered(owner, invocation, tree, fx),
            Event::ActionFinished { request, outcome } => {
                self.action_finished(request, outcome, fx)
            }
            Event::Failed(message) => {
                self.failure = Some(message);
                self.working = None;
            }
            Event::TemplateInspected {
                field,
                source,
                inspection,
            } => {
                if let Some(view) = self.stack.last_mut() {
                    if view.form.values.get(&field) == Some(&source) {
                        view.form.inspections.insert(field, inspection);
                    }
                }
            }
            Event::Activated { held, probe } => {
                self.activations += 1;
                self.probe = probe;
                if held {
                    fx.push(Effect::Search(self.query.clone()));
                }
            }
            Event::Reset => self.reset(fx),
        }
    }

    /// A failure deliberately survives this.
    ///
    /// An action that pastes hides the launcher before it does the work, so it
    /// can only report a failure once the window is already gone, and whether
    /// that report arrives before or after this reset is a race. The message is
    /// carried to the next activation instead, and cleared as soon as the user
    /// types, acts, or presses Escape.
    fn reset(&mut self, fx: &mut Vec<Effect>) {
        self.query.clear();
        self.query_revision = self.next_revision();
        self.results.clear();
        self.picked = None;
        self.stack.clear();
        self.owner = None;
        // Deliberately not abandoning: a hide for work resets through here too,
        // on behalf of a command that is still wanted and whose next view is
        // what brings the launcher back.
        self.shown_invocation = None;
        self.panel = None;
        self.protocol_error = false;
        self.working = None;
        fx.push(Effect::Search(String::new()));
    }

    fn rendered(&mut self, owner: String, invocation: u64, tree: ViewTree, fx: &mut Vec<Effect>) {
        if self
            .abandoned_invocation
            .is_some_and(|abandoned| invocation <= abandoned)
        {
            return;
        }
        self.last_invocation = Some(invocation);
        self.working = None;
        if tree.protocol_version != PROTOCOL_VERSION {
            self.protocol_error = true;
            return;
        }
        self.owner = Some(owner);
        if self.shown_invocation == Some(invocation) && !self.stack.is_empty() {
            self.replace_top(tree, fx);
        } else {
            self.push(tree, fx);
        }
        self.shown_invocation = Some(invocation);
    }

    fn action_finished(
        &mut self,
        request: ActionRequest,
        outcome: ActionOutcome,
        fx: &mut Vec<Effect>,
    ) {
        match (outcome, request.origin) {
            // A plain success already hid the launcher. A start reports through
            // the render channel and pushes its own view.
            (ActionOutcome::Done | ActionOutcome::Started, _) => {}
            (ActionOutcome::CopyToClipboard(text), _) => {
                fx.push(Effect::CopyText(text));
                fx.push(Effect::Dismiss);
            }
            // A root action that produces a view opens it. A snippet needing
            // arguments has to ask before it can do anything.
            (ActionOutcome::Replaced(tree), Origin::Root) => {
                self.owner = Some(request.extension_id);
                self.stack.clear();
                self.push(*tree, fx);
                self.shown_invocation = None;
            }
            (ActionOutcome::Removed(_), Origin::Root) => {
                fx.push(Effect::Search(self.query.clone()))
            }
            (ActionOutcome::Replaced(tree) | ActionOutcome::Removed(tree), Origin::View) => {
                if self.stack.is_empty() {
                    self.push(*tree, fx);
                } else {
                    self.replace_top(*tree, fx);
                }
                self.shown_invocation = None;
            }
            (ActionOutcome::Failed(message), _) => self.failure = Some(message),
        }
    }

    fn push(&mut self, tree: ViewTree, fx: &mut Vec<Effect>) {
        self.panel = None;
        let query_revision = self.next_revision();
        let mut view = PushedView {
            tree,
            query: String::new(),
            query_revision,
            picked: None,
            form: FormState::default(),
        };
        self.seed_form(&mut view, fx);
        self.stack.push(view);
    }

    fn replace_top(&mut self, tree: ViewTree, fx: &mut Vec<Effect>) {
        let Some(mut view) = self.stack.pop() else {
            return;
        };
        view.tree = tree;
        self.seed_form(&mut view, fx);
        self.stack.push(view);
    }

    /// A form takes its values from each tree it is given, so a field the user
    /// never touches still submits what it was showing.
    fn seed_form(&mut self, view: &mut PushedView, fx: &mut Vec<Effect>) {
        let View::Form(form) = &view.tree.view else {
            return;
        };
        view.form = FormState {
            values: form
                .fields
                .iter()
                .map(|field| (field.id.clone(), field.value.clone().unwrap_or_default()))
                .collect(),
            inspections: HashMap::new(),
            revision: self.next_revision(),
        };
        for field in form.fields.iter().filter(|f| f.kind == FieldKind::Template) {
            fx.push(Effect::InspectTemplate {
                field: field.id.clone(),
                source: field.value.clone().unwrap_or_default(),
            });
        }
    }

    fn next_revision(&mut self) -> u64 {
        self.revisions += 1;
        self.revisions
    }

    fn query_changed(&mut self, text: String, fx: &mut Vec<Effect>) {
        match self.stack.last_mut() {
            None => {
                self.query = text.clone();
                self.picked = None;
                self.failure = None;
                fx.push(Effect::Search(text));
            }
            Some(view) => {
                view.query = text;
                view.picked = None;
            }
        }
    }

    fn field_changed(&mut self, id: String, value: String, fx: &mut Vec<Effect>) {
        let Some(view) = self.stack.last_mut() else {
            return;
        };
        let View::Form(form) = &view.tree.view else {
            return;
        };
        let Some(field) = form.fields.iter().find(|field| field.id == id) else {
            return;
        };
        if field.kind == FieldKind::Template {
            fx.push(Effect::InspectTemplate {
                field: id.clone(),
                source: value.clone(),
            });
        }
        view.form.values.insert(id, value);
    }

    fn key(&mut self, chord: &Chord, fx: &mut Vec<Effect>) {
        let escape = *chord == Chord::plain("Escape");
        let enter = *chord == Chord::plain("Enter");
        let up = *chord == Chord::plain("ArrowUp");
        let down = *chord == Chord::plain("ArrowDown");
        let mode = self.mode();
        match mode {
            Mode::Error => {
                if escape {
                    self.protocol_error = false;
                }
                return;
            }
            Mode::Panel => {
                if escape {
                    self.panel = None;
                } else if up || down {
                    let ids = self.available_actions().iter().map(|a| a.id.as_str());
                    let next = step(ids, self.panel_selected(), down);
                    if let Some(panel) = self.panel.as_mut() {
                        panel.picked = next;
                    }
                } else if enter {
                    if let Some(id) = self.panel_selected() {
                        self.run_available(&id, fx);
                    }
                }
                return;
            }
            Mode::Root | Mode::List | Mode::Detail | Mode::Form => {}
        }

        if escape {
            self.escape(fx);
        } else if (up || down) && mode == Mode::Root {
            let next = step(
                self.root_order().map(|item| item.id.as_str()),
                self.root_selected().map(|item| item.id.clone()),
                down,
            );
            self.picked = next;
        } else if (up || down) && mode == Mode::List {
            let next = step(
                self.view_items().into_iter().map(|item| item.id.as_str()),
                self.view_selected().map(|item| item.id.clone()),
                down,
            );
            if let Some(view) = self.stack.last_mut() {
                view.picked = next;
            }
        } else if enter || (mode == Mode::Form && *chord == Chord::command("Enter")) {
            self.confirm(fx);
        } else if *chord == Chord::command("k") {
            self.open_panel();
        } else if let Some(action) = self.available_actions().iter().find(|action| {
            action
                .shortcut
                .as_ref()
                .is_some_and(|s| Chord::from(s) == *chord)
        }) {
            let id = action.id.clone();
            self.run_available(&id, fx);
        }
    }

    fn escape(&mut self, fx: &mut Vec<Effect>) {
        self.failure = None;
        if !self.stack.is_empty() {
            // A command may still be working behind this view, and the user has
            // left it: stop it rather than paying for an answer nobody reads.
            fx.push(Effect::CancelInvocation);
            self.abandoned_invocation = self.last_invocation;
            self.shown_invocation = None;
            self.stack.pop();
            let revision = self.next_revision();
            match self.stack.last_mut() {
                Some(view) => view.query_revision = revision,
                None => {
                    self.owner = None;
                    self.query_revision = revision;
                }
            }
        } else if !self.query.is_empty() {
            self.query.clear();
            self.query_revision = self.next_revision();
            fx.push(Effect::Search(String::new()));
        } else {
            fx.push(Effect::Dismiss);
        }
    }

    /// Enter on what is showing. A root result that is a command is invoked,
    /// anything else runs its primary action. A command carries no actions,
    /// which is what tells the two apart.
    fn confirm(&mut self, fx: &mut Vec<Effect>) {
        match self.mode() {
            Mode::Root => {
                let Some(item) = self.root_selected().cloned() else {
                    return;
                };
                match item.actions.first() {
                    Some(action) => self.run_root_action(&item, &action.id, fx),
                    None => {
                        self.failure = None;
                        self.panel = None;
                        self.working = Some(item.title.clone());
                        fx.push(Effect::InvokeCommand {
                            command_id: item.id,
                            query: self.query.clone(),
                        });
                    }
                }
            }
            Mode::List => {
                let Some(item) = self.view_selected() else {
                    return;
                };
                if let Some(action) = item.actions.first() {
                    let (action, item) = (action.id.clone(), item.id.clone());
                    self.run_view_action(action, Some(item), None, fx);
                }
            }
            Mode::Detail => {
                if let Some(action) = self.view_actions().first() {
                    let action = action.id.clone();
                    self.run_view_action(action, None, None, fx);
                }
            }
            Mode::Form => {
                let Some(view) = self.stack.last() else {
                    return;
                };
                let View::Form(form) = &view.tree.view else {
                    return;
                };
                if let Some(action) = form.actions.first() {
                    let (action, item, values) = (
                        action.id.clone(),
                        form.item_id.clone(),
                        view.form.values.clone(),
                    );
                    self.run_view_action(action, item, Some(values), fx);
                }
            }
            Mode::Error | Mode::Panel => {}
        }
    }

    fn activate(&mut self, id: &str, fx: &mut Vec<Effect>) {
        match self.mode() {
            Mode::Panel => {
                if self
                    .available_actions()
                    .iter()
                    .any(|action| action.id == id)
                {
                    self.run_available(id, fx);
                }
            }
            Mode::Root => {
                if self.results.iter().any(|item| item.id == id) {
                    self.picked = Some(id.to_string());
                    self.confirm(fx);
                }
            }
            Mode::List => {
                if self.view_items().iter().any(|item| item.id == id) {
                    if let Some(view) = self.stack.last_mut() {
                        view.picked = Some(id.to_string());
                    }
                    self.confirm(fx);
                }
            }
            Mode::Error | Mode::Detail | Mode::Form => {}
        }
    }

    fn open_panel(&mut self) {
        if !self.available_actions().is_empty() {
            self.panel = Some(PanelState { picked: None });
        }
    }

    /// Runs one of the actions the panel offers, on whatever it applies to.
    fn run_available(&mut self, action: &str, fx: &mut Vec<Effect>) {
        if self.stack.is_empty() {
            if let Some(item) = self.root_selected().cloned() {
                self.run_root_action(&item, action, fx);
            }
            return;
        }
        let item = self.view_item_id();
        self.run_view_action(action.to_string(), item, None, fx);
    }

    fn run_root_action(&mut self, item: &ResultItem, action: &str, fx: &mut Vec<Effect>) {
        self.failure = None;
        self.panel = None;
        fx.push(Effect::RunAction(ActionRequest {
            origin: Origin::Root,
            extension_id: item.extension_id.clone(),
            item_id: item.id.clone(),
            action_id: action.to_string(),
            values: None,
            query: Some(self.query.clone()),
        }));
    }

    /// An action chosen inside a pushed view goes to the extension that owns
    /// the running command, through the same path a root result uses.
    fn run_view_action(
        &mut self,
        action: String,
        item: Option<String>,
        values: Option<FormValues>,
        fx: &mut Vec<Effect>,
    ) {
        let Some(owner) = self.owner.clone() else {
            return;
        };
        self.failure = None;
        self.panel = None;
        fx.push(Effect::RunAction(ActionRequest {
            origin: Origin::View,
            extension_id: owner,
            item_id: item.unwrap_or_default(),
            action_id: action,
            values,
            query: None,
        }));
    }

    pub(super) fn mode(&self) -> Mode {
        if self.protocol_error {
            Mode::Error
        } else if self.panel.is_some() {
            Mode::Panel
        } else {
            self.surface()
        }
    }

    /// What is showing under the action panel.
    pub(super) fn surface(&self) -> Mode {
        match self.stack.last().map(|view| &view.tree.view) {
            None => Mode::Root,
            Some(View::List(_)) => Mode::List,
            Some(View::Detail(_)) => Mode::Detail,
            Some(View::Form(_)) => Mode::Form,
        }
    }

    /// Root results in the order they are drawn: pinned, then suggestions,
    /// then everything else.
    pub(super) fn root_order(&self) -> impl Iterator<Item = &ResultItem> {
        let pinned = self.results.iter().filter(|item| item.pinned);
        let suggested = self
            .results
            .iter()
            .filter(|item| !item.pinned && item.suggested);
        let rest = self
            .results
            .iter()
            .filter(|item| !item.pinned && !item.suggested);
        pinned.chain(suggested).chain(rest)
    }

    pub(super) fn root_selected(&self) -> Option<&ResultItem> {
        self.picked
            .as_deref()
            .and_then(|picked| self.results.iter().find(|item| item.id == picked))
            .or_else(|| self.root_order().next())
    }

    /// A pushed list's items after launcher-side narrowing. A view that owns its
    /// own filtering would get each query change instead, which no command
    /// needs yet.
    pub(super) fn view_items(&self) -> Vec<&ListItem> {
        let Some(view) = self.stack.last() else {
            return Vec::new();
        };
        let View::List(list) = &view.tree.view else {
            return Vec::new();
        };
        if list.filtering != Filtering::Launcher || view.query.is_empty() {
            return list.items.iter().collect();
        }
        let needle = view.query.to_lowercase();
        list.items
            .iter()
            .filter(|item| item.title.to_lowercase().contains(&needle))
            .collect()
    }

    pub(super) fn view_selected(&self) -> Option<&ListItem> {
        let items = self.view_items();
        let picked = self.stack.last().and_then(|view| view.picked.as_deref());
        picked
            .and_then(|picked| items.iter().find(|item| item.id == picked))
            .or_else(|| items.first())
            .copied()
    }

    fn view_actions(&self) -> &[Action] {
        match self.stack.last().map(|view| &view.tree.view) {
            Some(View::Detail(detail)) => &detail.actions,
            Some(View::Form(form)) => &form.actions,
            Some(View::List(_)) | None => &[],
        }
    }

    /// What the action panel offers. A list item's own actions win over the
    /// view's, because the item is what the user has selected.
    pub(super) fn available_actions(&self) -> &[Action] {
        match self.surface() {
            Mode::Root => self.root_selected().map_or(&[], |item| &item.actions),
            Mode::List => self.view_selected().map_or(&[], |item| &item.actions),
            _ => self.view_actions(),
        }
    }

    pub(super) fn panel_selected(&self) -> Option<String> {
        let panel = self.panel.as_ref()?;
        let actions = self.available_actions();
        panel
            .picked
            .as_deref()
            .filter(|picked| actions.iter().any(|action| action.id == *picked))
            .or_else(|| actions.first().map(|action| action.id.as_str()))
            .map(str::to_string)
    }

    pub(super) fn panel_open(&self) -> bool {
        self.panel.is_some()
    }

    /// Which item an action applies to. Lists use their selection, forms carry
    /// the record that opened them, and detail views have none.
    fn view_item_id(&self) -> Option<String> {
        match self.stack.last().map(|view| &view.tree.view) {
            Some(View::List(_)) => self.view_selected().map(|item| item.id.clone()),
            Some(View::Form(form)) => form.item_id.clone(),
            Some(View::Detail(_)) | None => None,
        }
    }
}

/// The id one row up or down from `current`, stopping at either end.
fn step<'a>(
    ids: impl Iterator<Item = &'a str>,
    current: Option<String>,
    down: bool,
) -> Option<String> {
    let ids: Vec<&str> = ids.collect();
    let index = current
        .as_deref()
        .and_then(|current| ids.iter().position(|id| *id == current));
    let next = match (index, down) {
        (None, _) => 0,
        (Some(i), true) => (i + 1).min(ids.len().saturating_sub(1)),
        (Some(i), false) => i.saturating_sub(1),
    };
    ids.get(next).map(|id| id.to_string())
}
