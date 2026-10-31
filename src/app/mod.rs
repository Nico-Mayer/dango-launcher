//! The running application: everything the launcher needs, and every rule for
//! showing, hiding, and feeding it. A `Shell` draws it.

mod startup;
#[cfg(test)]
mod tests;

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, Weak};
use std::time::Duration;

use tokio::runtime::Handle;
use tokio::task::JoinHandle;

use crate::config::{self, FileConfig};
use crate::extension::{self, ActionOutcome, ExtensionHost, FormValues};
use crate::extensions::clipboard::ClipboardSource;
use crate::extensions::{self, ai::AiExtension, applications::ApplicationsExtension};
use crate::hotkeys::{self, Pressed, Registry};
use crate::invocation::{self, InvocationContext, InvokeError, Invoker, Outcome, Output};
use crate::latency::LatencyProbe;
use crate::platform::{self, Permission};
use crate::ranking::{now_millis, FrecencyTable, InputHistory};
use crate::search::{Candidate, SearchPipeline, StaticCommandSource};
use crate::shell::{self, Shell, TrayCommand, TrayStatus};
use crate::{templates, text, ui};
use global_hotkey::hotkey::Modifiers;

pub use startup::{cache_dir, start, Startup};

/// The most results streamed to the renderer for one query.
const RESULT_LIMIT: usize = 50;

const DEFAULT_RESET_AFTER_SECS: u32 = 60;

const PERMISSION_POLL: Duration = Duration::from_secs(2);

pub struct Core {
    shell: Arc<dyn Shell>,
    runtime: Handle,
    config: Arc<FileConfig>,
    ui: Mutex<UiState>,
    latency: LatencyProbe,
    hold: Mutex<Option<Hold>>,
    hold_generation: AtomicU64,
    visible: AtomicBool,
    tray: Mutex<TrayStatus>,
    services: Services,
}

/// What the extensions and search need, built once at startup.
pub struct Services {
    pub host: Arc<Mutex<ExtensionHost>>,
    pub invoker: Arc<Invoker>,
    pub pipeline: SearchPipeline,
    pub frecency: Arc<FrecencyTable>,
    pub learned: Arc<InputHistory>,
    /// What each extension declared as its tint, so a result can be drawn in
    /// its owner's colour. Held apart from the host, whose lock has no business
    /// on the path that streams results.
    pub tints: std::collections::HashMap<String, String>,
    pub applications: Option<Arc<ApplicationsExtension>>,
    pub ai: Option<AiParts>,
    pub hotkeys: Registry,
    pub hyperkey: Mutex<Option<Box<dyn platform::Hyperkey>>>,
}

struct UiState {
    launcher: ui::Launcher,
    /// The snapshot the renderer last got.
    shown: Option<ui::Snapshot>,
}

/// An interrupting hide whose state the launcher is still keeping. The task
/// resets the launcher when the grace period runs out; showing aborts it.
struct Hold {
    generation: u64,
    expiry: JoinHandle<()>,
}

/// Why the launcher is going away, which decides what becomes of its state
/// and of a command still running.
enum HideReason {
    /// The user is done: an action finished, or Escape on an empty root.
    Done,
    /// The user was interrupted: the shortcut toggled, or focus went elsewhere.
    Interrupted,
    /// A command hid the launcher to paste or read a selection and is still wanted.
    ForWork,
}

/// What the AI extension needs besides the configuration. Kept so a config
/// reload can build the extension again: its commands come from the file, so an
/// edit means a new instance rather than a mutated one.
pub struct AiParts {
    pub keys: Arc<dyn extensions::ai::Keys>,
    pub client: Arc<dyn extensions::ai::Completions>,
    pub text: Option<Arc<dyn text::TextTarget>>,
    pub clipboard: Option<Arc<dyn ClipboardSource>>,
    pub runner: Arc<dyn invocation::Runner>,
}

impl AiParts {
    fn build(&self, config: &config::Config) -> (Arc<AiExtension>, Vec<String>) {
        let (extension, problems) = AiExtension::new(
            config,
            self.keys.clone(),
            self.client.clone(),
            self.text.clone(),
            self.clipboard.clone(),
            Some(self.runner.clone()),
        );
        (Arc::new(extension), problems)
    }
}

impl Core {
    fn new(
        shell: Arc<dyn Shell>,
        runtime: Handle,
        config: Arc<FileConfig>,
        tray: TrayStatus,
        services: Services,
    ) -> Self {
        Self {
            shell,
            runtime,
            config,
            ui: Mutex::new(UiState {
                launcher: ui::Launcher::new(cfg!(debug_assertions)),
                shown: None,
            }),
            latency: LatencyProbe::default(),
            hold: Mutex::new(None),
            hold_generation: AtomicU64::new(0),
            visible: AtomicBool::new(false),
            tray: Mutex::new(tray),
            services,
        }
    }

    pub fn runtime(&self) -> &Handle {
        &self.runtime
    }

    /// Everything the renderer does arrives here, in the order it happened.
    pub fn dispatch(self: &Arc<Self>, intent: ui::Intent) {
        if intent == ui::Intent::Ready {
            // A reloaded renderer has none of what was already sent.
            self.ui.lock().unwrap().shown = None;
        }
        self.apply(ui::Input::Intent(intent));
    }

    /// Feeds the launcher state machine, sends the renderer the new snapshot, and
    /// runs what the state machine asked for. The snapshot goes out under the
    /// lock, so snapshots reach the renderer in the order they were made.
    fn apply(self: &Arc<Self>, input: ui::Input) {
        let effects = {
            let mut state = self.ui.lock().unwrap();
            let effects = state.launcher.apply(input);
            let snapshot = state.launcher.snapshot();
            if state.shown.as_ref() != Some(&snapshot) {
                self.shell.publish(&snapshot);
                state.shown = Some(snapshot);
            }
            effects
        };
        // `apply` also runs on the main thread and in the hold timer, where there
        // is no runtime context for the effects to spawn into.
        let _runtime = self.runtime.enter();
        for effect in effects {
            self.run_effect(effect);
        }
    }

    fn run_effect(self: &Arc<Self>, effect: ui::Effect) {
        match effect {
            ui::Effect::Search(query) => self.search(query),
            ui::Effect::RunAction(request) => {
                let core = self.clone();
                self.runtime.spawn_blocking(move || {
                    let outcome = core.run_action(&request);
                    core.apply(ui::Input::Event(ui::Event::ActionFinished {
                        request,
                        outcome,
                    }));
                });
            }
            ui::Effect::InvokeCommand { command_id, query } => {
                match self.run_command(&command_id) {
                    Ok(()) => self.record_use(&command_id, Some(&query)),
                    Err(error) => {
                        self.apply(ui::Input::Event(ui::Event::Failed(error.to_string())))
                    }
                }
            }
            ui::Effect::CancelInvocation => self.services.invoker.abandon(),
            ui::Effect::Dismiss => {
                let core = self.clone();
                self.on_main_thread(move || {
                    if core.hide() {
                        core.latency.note("dismissed by the renderer");
                    }
                });
            }
            ui::Effect::CopyText(text) => self.on_main_thread(move || {
                use clipboard_rs::Clipboard;
                let copied = clipboard_rs::ClipboardContext::new()
                    .and_then(|clipboard| clipboard.set_text(text));
                if let Err(error) = copied {
                    eprintln!("[dango] could not copy to the clipboard: {error}");
                }
            }),
            ui::Effect::InspectTemplate { field, source } => {
                let inspection = inspect_template(&source);
                self.apply(ui::Input::Event(ui::Event::TemplateInspected {
                    field,
                    source,
                    inspection,
                }));
            }
            ui::Effect::FinishWarmup => {
                let shell = self.shell.clone();
                self.on_main_thread(move || {
                    // Once per renderer mount. More than one line here across a
                    // session means the renderer reloaded.
                    eprintln!("[dango] renderer mounted");
                    shell.finish_warmup();
                });
            }
            ui::Effect::ReportPaint(probe) => self.latency.finish(probe),
        }
    }

    fn on_main_thread(&self, work: impl FnOnce() + Send + 'static) {
        shell::on_main_thread(self.shell.as_ref(), work);
    }

    /// Runs a query and feeds merged results to the launcher as providers answer.
    fn search(self: &Arc<Self>, query: String) {
        let mut rx = self.services.pipeline.query(query);
        let core = self.clone();
        self.runtime.spawn(async move {
            while let Some(results) = rx.recv().await {
                let items = results
                    .items
                    .into_iter()
                    .map(|candidate| core.result_item(candidate))
                    .collect();
                core.apply(ui::Input::Event(ui::Event::Results {
                    query: results.query,
                    items,
                }));
            }
        });
    }

    fn result_item(&self, c: Candidate) -> ui::ResultItem {
        ui::ResultItem {
            tint: self.services.tints.get(&c.extension_id).cloned(),
            extension_id: c.extension_id,
            id: c.id,
            title: c.title,
            subtitle: c.subtitle,
            icon: c.icon,
            actions: c.actions,
            match_positions: c.match_positions,
            suggested: c.suggested,
            pinned: c.pinned,
        }
    }

    pub fn toggle(self: &Arc<Self>) {
        let core = self.clone();
        self.on_main_thread(move || {
            if core.visible.load(Ordering::SeqCst) {
                core.hide();
            } else {
                let probe = core.latency.start();
                core.show(probe);
            }
        });
    }

    pub fn show(self: &Arc<Self>, probe: Option<u64>) {
        let core = self.clone();
        self.on_main_thread(move || {
            let held = core.clear_hold();
            core.shell.show();
            core.visible.store(true, Ordering::SeqCst);
            core.apply(ui::Input::Event(ui::Event::Activated { held, probe }));
        });
    }

    /// Caller must already be on the main thread; `toggle` and `Dismiss` are
    /// what guarantee that. Returning a value is why this cannot marshal itself.
    fn hide(self: &Arc<Self>) -> bool {
        // Hiding drops the renderer's focus, which can fire a second dismiss.
        // Nothing must happen twice for that.
        if !self.visible.swap(false, Ordering::SeqCst) {
            return false;
        }
        self.shell.hide(true);
        self.settle_hide(HideReason::Interrupted);
        true
    }

    /// Hides the launcher without restoring the previous foreground, because the
    /// just-launched application should keep focus rather than the window that
    /// was in front before.
    fn hide_after_launch(self: &Arc<Self>) {
        let core = self.clone();
        self.on_main_thread(move || {
            if core.visible.swap(false, Ordering::SeqCst) {
                core.shell.hide(false);
            }
            core.settle_hide(HideReason::Done);
        });
    }

    /// The window lost focus. A window that is already hidden is left alone: a
    /// command's own hide drops focus too, and that hide has already settled
    /// what becomes of the launcher's state.
    pub fn hide_on_focus_loss(self: &Arc<Self>) {
        let core = self.clone();
        self.on_main_thread(move || {
            if !core.visible.swap(false, Ordering::SeqCst) {
                return;
            }
            core.shell.hide(false);
            core.settle_hide(HideReason::Interrupted);
        });
    }

    /// Gets the launcher out of the way on a running command's behalf, which
    /// reading the selection and inserting text both need: the keystrokes have
    /// to land in the application the user came from, not in the search field.
    ///
    /// Deliberately does not abandon anything. The launcher is going away
    /// because the command asked, so the command is still wanted; if it has
    /// something to show afterwards, its next view brings the launcher back.
    fn hide_for_work(self: &Arc<Self>) {
        let core = self.clone();
        self.on_main_thread(move || {
            core.visible.store(false, Ordering::SeqCst);
            core.shell.hide(false);
            core.settle_hide(HideReason::ForWork);
        });
    }

    fn settle_hide(self: &Arc<Self>, reason: HideReason) {
        match reason {
            HideReason::Interrupted => {
                let period = self.reset_after();
                if !period.is_zero() {
                    self.start_hold(period);
                    return;
                }
                self.clear_hold();
                self.services.invoker.abandon();
            }
            HideReason::Done => {
                self.clear_hold();
                self.services.invoker.abandon();
            }
            HideReason::ForWork => {
                self.clear_hold();
            }
        }
        self.apply(ui::Input::Event(ui::Event::Reset));
    }

    fn reset_after(&self) -> Duration {
        let seconds = self
            .config
            .shared()
            .read()
            .ok()
            .and_then(|config| config.launcher.reset_after)
            .unwrap_or(DEFAULT_RESET_AFTER_SECS);
        Duration::from_secs(seconds.into())
    }

    fn holding(&self) -> bool {
        self.hold.lock().unwrap().is_some()
    }

    fn clear_hold(&self) -> bool {
        let Some(hold) = self.hold.lock().unwrap().take() else {
            return false;
        };
        hold.expiry.abort();
        true
    }

    /// A command still working when the hold runs out has nowhere to put its
    /// output, and the user has moved on. Its later output is dropped rather
    /// than waiting to surprise them on the next activation.
    fn start_hold(self: &Arc<Self>, period: Duration) {
        let mut hold = self.hold.lock().unwrap();
        if let Some(previous) = hold.take() {
            previous.expiry.abort();
        }
        let generation = self.hold_generation.fetch_add(1, Ordering::SeqCst);
        let core = self.clone();
        let expiry = self.runtime.spawn(async move {
            tokio::time::sleep(period).await;
            {
                let mut hold = core.hold.lock().unwrap();
                // A show between waking and taking the lock has already ended this hold.
                if !matches!(hold.as_ref(), Some(current) if current.generation == generation) {
                    return;
                }
                hold.take();
            }
            core.services.invoker.abandon();
            core.apply(ui::Input::Event(ui::Event::Reset));
        });
        *hold = Some(Hold { generation, expiry });
    }

    /// Runs an action through the extension that contributed the item. Success
    /// hides the launcher; anything else goes back to the state machine.
    fn run_action(self: &Arc<Self>, request: &ui::ActionRequest) -> ActionOutcome {
        let outcome = self.services.host.lock().unwrap().perform_action(
            &request.extension_id,
            &request.item_id,
            &request.action_id,
            &request.values.clone().unwrap_or_default(),
        );
        if outcome == ActionOutcome::Done {
            // Any successful action counts as use. Revealing an application is
            // reaching for it just as much as launching it is.
            self.record_use(&request.item_id, request.query.as_deref());
            self.hide_after_launch();
        }
        outcome
    }

    /// Credits a use of a result and, when the launcher knows what was typed to
    /// reach it, learns that text for the item. Only the two root confirm paths
    /// call this: an action inside a pushed view passes no text, and a hotkey
    /// press never gets here, because a chord says nothing about what the user
    /// types to find an item.
    fn record_use(&self, item_id: &str, query: Option<&str>) {
        let now = now_millis();
        self.services.frecency.record_launch(item_id, now);
        if let Some(query) = query {
            self.services.learned.record(query, item_id, now);
        }
    }

    /// Runs a command through the invoker and streams its output the same way
    /// for a launcher selection and a hotkey press.
    fn run_command(self: &Arc<Self>, command_id: &str) -> Result<(), InvokeError> {
        let owner = self
            .services
            .host
            .lock()
            .ok()
            .and_then(|host| Some(host.registry().get(command_id)?.extension_id.clone()))
            .ok_or(InvokeError::Unavailable)?;
        let running = self.services.invoker.invoke(command_id)?;
        self.pump(owner, running);
        Ok(())
    }

    /// Carries one invocation's output to the renderer: a view goes out as a
    /// render event, success hides the launcher, a failure is reported. Every
    /// tree carries the invocation that produced it, so the launcher replaces
    /// the tree it is already showing for that invocation rather than stacking
    /// another on top.
    fn pump(self: &Arc<Self>, owner: String, running: invocation::Running) {
        let core = self.clone();
        let invocation = running.invocation;
        let mut output = running.output;
        self.runtime.spawn(async move {
            while let Some(next) = output.recv().await {
                match next {
                    Output::View(tree) => {
                        // A command that read the selection hid the launcher to
                        // do it. It has something to show now, so the launcher
                        // comes back rather than streaming into a window nobody
                        // can see. A held launcher is different: the user closed
                        // it and will find the output when they come back.
                        if !core.visible.load(Ordering::SeqCst) && !core.holding() {
                            core.show(None);
                        }
                        core.apply(ui::Input::Event(ui::Event::Rendered {
                            owner: owner.clone(),
                            invocation,
                            tree,
                        }));
                    }
                    Output::Finished(Outcome::Success) => {
                        core.hide_after_launch();
                        break;
                    }
                    Output::Finished(Outcome::Failure(message)) => {
                        core.apply(ui::Input::Event(ui::Event::Failed(message)));
                        break;
                    }
                }
            }
        });
    }

    /// A press on a registered chord.
    pub fn on_hotkey(self: &Arc<Self>, event: global_hotkey::GlobalHotKeyEvent) {
        match self.services.hotkeys.pressed(event.id(), event.state()) {
            Some(Pressed::Launcher) => self.toggle(),
            Some(Pressed::Target(hotkeys::Target::Command(id))) => self.dispatch_command(&id),
            Some(Pressed::Target(hotkeys::Target::Application(name))) => {
                self.dispatch_application(&name)
            }
            None => {}
        }
    }

    /// Invokes a command from its hotkey. A no-view command runs without showing
    /// the launcher, acting on the window that was focused at the press; a view
    /// command shows the launcher first so its pushed view is displayed.
    fn dispatch_command(self: &Arc<Self>, command_id: &str) {
        let shows_view = self
            .services
            .host
            .lock()
            .ok()
            .and_then(|host| {
                Some(host.registry().get(command_id)?.decl.mode == extension::InvocationMode::View)
            })
            .unwrap_or(false);

        if shows_view {
            self.show(None);
            let _ = self.run_command(command_id);
            return;
        }

        // The launcher never showed to record the window the press acts on.
        platform::note_insertion_target();
        let _ = self.run_command(command_id);
    }

    /// Launches an application from its hotkey: the same launch action Enter
    /// runs on the result, without the frecency credit. The name is resolved
    /// now rather than when the chord was registered, so a binding shared from
    /// another machine starts working the moment the application is installed.
    fn dispatch_application(self: &Arc<Self>, name: &str) {
        let Some(applications) = &self.services.applications else {
            self.report_problem("application hotkeys need the applications extension");
            return;
        };
        let target = match hotkeys::resolve_application(name, applications.find_by_name(name)) {
            hotkeys::Resolution::Missing(message) => {
                self.report_problem(&message);
                return;
            }
            hotkeys::Resolution::Launch {
                app: target,
                ambiguity,
            } => {
                if let Some(note) = ambiguity {
                    self.report_problem(&note);
                }
                target
            }
        };
        platform::note_insertion_target();

        let core = self.clone();
        // Off the shortcut handler's thread, as a result's launch runs off the
        // command thread: the launch may wait on the operating system.
        std::thread::spawn(move || {
            let outcome = core.services.host.lock().unwrap().perform_action(
                extensions::applications::EXTENSION_ID,
                &target.id,
                extensions::applications::ACTION_LAUNCH,
                &FormValues::new(),
            );
            match outcome {
                ActionOutcome::Done => core.hide_after_launch(),
                ActionOutcome::Failed(message) => {
                    core.report_problem(&format!("application \"{}\": {message}", target.name))
                }
                ActionOutcome::Started
                | ActionOutcome::CopyToClipboard(_)
                | ActionOutcome::Replaced(_)
                | ActionOutcome::Removed(_) => {}
            }
        });
    }

    /// A hotkey press or a file edit that went wrong has no launcher on screen
    /// to say so; the tray status line and the log are where it is visible.
    fn report_problem(&self, message: &str) {
        crate::log::append(message);
        self.set_config_status(true);
    }

    fn set_config_status(&self, error: bool) {
        let mut tray = self.tray.lock().unwrap();
        tray.config_error = error;
        self.shell.set_tray(&tray);
    }

    fn set_missing_permissions(&self, missing: Vec<Permission>) {
        let mut tray = self.tray.lock().unwrap();
        tray.missing = missing;
        self.shell.set_tray(&tray);
    }

    pub fn tray(&self) -> TrayStatus {
        self.tray.lock().unwrap().clone()
    }

    pub fn tray_command(self: &Arc<Self>, command: TrayCommand) {
        match command {
            TrayCommand::Toggle => self.toggle(),
            TrayCommand::Grant(permission) => {
                let outcome = extensions::permissions::grant(
                    platform::permissions().as_ref(),
                    &crate::opener::SystemOpener,
                    permission,
                );
                if let ActionOutcome::Failed(message) = outcome {
                    crate::log::append(&message);
                }
            }
            TrayCommand::Quit => self.quit(),
        }
    }

    pub fn quit(&self) {
        self.services.hotkeys.release_all();
        self.shell.quit();
    }

    /// Keeps the tray in step with the permissions. A grant or revoke happens in
    /// System Settings, where Dango cannot see it, so polling is the only signal.
    fn watch_permissions(self: &Arc<Self>) {
        let permissions = platform::permissions();
        let core = Arc::downgrade(self);
        let mut last = permissions.missing();
        let spawned = std::thread::Builder::new()
            .name("dango-permissions".into())
            .spawn(move || loop {
                std::thread::sleep(PERMISSION_POLL);
                let missing = permissions.missing();
                if missing == last {
                    continue;
                }
                last = missing.clone();
                let Some(core) = core.upgrade() else {
                    return;
                };
                core.set_missing_permissions(missing);
            });
        if let Err(error) = spawned {
            eprintln!("[dango] permission watching is off: {error}");
        }
    }

    /// Applies a reloaded configuration, on the main thread. Re-registers the
    /// launcher hotkey if it changed, brings enabled state and search in line,
    /// and updates the tray. A parse failure keeps the running config and is
    /// surfaced.
    fn apply_config_reload(&self, result: Result<config::Config, config::ConfigError>) {
        let config = match result {
            Ok(config) => config,
            Err(error) => {
                crate::log::append(&format!("config reload failed: {error}"));
                self.set_config_status(true);
                return;
            }
        };

        let launcher = hotkeys::launcher(&config);
        let autostart = config.launcher.autostart.unwrap_or(false);
        let autostart_changed = {
            let shared = self.config.shared();
            let previous = shared.read().unwrap();
            previous.launcher.autostart.unwrap_or(false) != autostart
        };
        self.config.replace(config);

        if let Err(error) = self.services.hotkeys.replace_launcher(launcher) {
            crate::log::append(&format!(
                "could not register the new launcher hotkey: {error}"
            ));
        }

        {
            let mut host = self.services.host.lock().unwrap();
            host.reload_enabled();
            // The AI extension's commands are in the file, so an edit means a new
            // instance. Done before the candidates and the hotkeys are rebuilt, so
            // both see the command set the file now describes.
            if let Some(parts) = &self.services.ai {
                let (extension, problems) = {
                    let config = self.config.shared();
                    let config = config.read().unwrap();
                    parts.build(&config)
                };
                for problem in &problems {
                    crate::log::append(problem);
                }
                if let Err(error) = host.replace(extension) {
                    crate::log::append(&format!("ai: {error}"));
                }
            }
            let candidates = aliased_candidates(&host, &self.config);
            self.services.pipeline.reload(
                Arc::new(StaticCommandSource(candidates)),
                host.root_providers(),
            );
        }

        let conflicts = {
            let config = self.config.shared();
            let config = config.read().unwrap();
            let mut conflicts = self.services.hotkeys.apply(&config);
            if let Some(message) = apply_hyperkey(&config, &self.services.hyperkey) {
                conflicts.push(message);
            }
            if autostart_changed {
                if let Err(message) = crate::autostart::apply(autostart) {
                    conflicts.push(message);
                }
            }
            conflicts
        };
        for conflict in &conflicts {
            crate::log::append(conflict);
        }
        self.set_config_status(!conflicts.is_empty());
    }
}

/// Starts an invocation for work an extension begins itself. A submitted
/// argument form has an answer to stream but no command running behind it, so
/// it asks for one and its output reaches the launcher the usual way.
struct ViewRunner {
    core: Weak<Core>,
    owner: &'static str,
}

impl invocation::Runner for ViewRunner {
    fn start(&self) -> Option<Arc<InvocationContext>> {
        let core = self.core.upgrade()?;
        let (ctx, running) = core.services.invoker.detached(self.owner);
        core.pump(self.owner.to_string(), running);
        Some(Arc::new(ctx))
    }
}

/// Getting the launcher off screen before a keystroke is sent to another
/// application. The window has to be hidden from the main thread, and the
/// insertion runs off it, so this hops back.
struct HideLauncher(Weak<Core>);

impl text::Launcher for HideLauncher {
    fn dismiss(&self) {
        if let Some(core) = self.0.upgrade() {
            core.hide_for_work();
        }
    }
}

/// Hops a closure onto the main thread.
///
/// macOS traps if key synthesis runs anywhere else: mapping a character to a
/// keycode goes through the Text Services Manager, which asserts the main
/// queue. The rest of an insertion has to stay off the main thread, because it
/// sleeps and the hide it waits for needs the main run loop to turn.
struct OnMainThread(Arc<dyn Shell>);

impl text::MainThread for OnMainThread {
    fn run(&self, work: Box<dyn FnOnce() + Send>) {
        shell::on_main_thread(self.0.as_ref(), work);
    }
}

/// Parses a template and answers with the arguments it would ask for, or why it
/// cannot be read. This is what makes a doubled-brace expression pasted from
/// another templating system visible before it is saved.
fn inspect_template(source: &str) -> ui::TemplateInspection {
    match templates::Template::parse(source) {
        Ok(template) => ui::TemplateInspection {
            arguments: template.prompts(),
            error: None,
        },
        Err(error) => ui::TemplateInspection {
            arguments: Vec::new(),
            error: Some(error.to_string()),
        },
    }
}

/// Command candidates with the config file's aliases applied over the manifest's.
fn aliased_candidates(host: &ExtensionHost, file_config: &FileConfig) -> Vec<Candidate> {
    let shared = file_config.shared();
    let config = shared.read().unwrap();
    host.command_candidates()
        .into_iter()
        .map(|mut candidate| {
            let prefix = format!("{}.", candidate.extension_id);
            if let Some(command_id) = candidate.id.strip_prefix(&prefix) {
                if let Some(alias) = config.alias(&candidate.extension_id, command_id) {
                    candidate.alias = Some(alias);
                }
            }
            candidate
        })
        .collect()
}

/// (Re)starts the hyperkey from a config: drops the current one, which stops its
/// hook and releases any modifiers it holds, then starts a fresh one if the
/// config asks for it. Returns a message to surface when a configured hyperkey
/// could not start.
fn apply_hyperkey(
    config: &config::Config,
    handle: &Mutex<Option<Box<dyn platform::Hyperkey>>>,
) -> Option<String> {
    let mut current = handle.lock().unwrap();
    *current = None;

    let hyperkey = config.hyperkey.as_ref()?;
    let Some(trigger) = platform::HyperkeyTrigger::from_key(&hyperkey.key) else {
        return Some(format!(
            "hyperkey: '{}' is not a remappable key",
            hyperkey.key
        ));
    };
    let mods = config.hyper_modifiers();
    let spec = platform::HyperkeySpec {
        trigger,
        emit: platform::HyperModifiers {
            ctrl: mods.contains(Modifiers::CONTROL),
            alt: mods.contains(Modifiers::ALT),
            shift: mods.contains(Modifiers::SHIFT),
            meta: mods.contains(Modifiers::SUPER),
        },
        tap: hyperkey.tap.clone(),
    };
    match platform::start_hyperkey(spec) {
        Some(running) => {
            *current = Some(running);
            None
        }
        None if platform::permissions()
            .missing()
            .contains(&Permission::InputMonitoring) =>
        {
            Some("hyperkey: needs the Input Monitoring permission".to_string())
        }
        None => Some(
            "hyperkey: could not start; unavailable on this platform or refused by the OS"
                .to_string(),
        ),
    }
}
