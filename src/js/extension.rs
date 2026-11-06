use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::extension::manifest::{InvocationMode, Manifest};
use crate::extension::{ActionOutcome, ActivationResult, Extension, FormValues};
use crate::invocation::{Command, InvocationContext};
use crate::protocol::{View, ViewTree, PROTOCOL_VERSION, SEARCH_ACTION};

use super::host::JsHost;
use super::live::Live;
use super::load::MAIN_FILE;
use super::manifest::JsManifest;
use super::root::{CachedProvider, CachedRows, RootItem};
use super::session::{JsError, Target};

pub const NO_CODE: &str = "This command's extension has no code for it";
pub const STOPPED_BY_ERROR: &str = "This command stopped because of an error, see dango.log";
pub const OUT_OF_MEMORY: &str = "This command used too much memory, so Dango stopped it";
const NO_ACTION: &str = "That action isn't available";

/// How long a session nothing on screen follows may run: a root items build,
/// or a root item's action that opens no view. Ended from a task of its own,
/// since code that loops holds the task it runs in.
const UNWATCHED_LIMIT: Duration = if cfg!(test) {
    Duration::from_secs(1)
} else {
    Duration::from_secs(30)
};

/// An extension loaded from a folder of the extensions directory.
pub struct JsExtension {
    inner: Arc<Inner>,
}

struct Inner {
    manifest: Manifest,
    folder: PathBuf,
    refresh: Option<Duration>,
    host: Arc<JsHost>,
    /// The session behind the views it shows, so their actions reach it.
    live: Mutex<Option<Arc<Live>>>,
    rows: Arc<CachedRows>,
    /// Builds root items while the extension is active.
    builder: Mutex<Option<tokio::task::JoinHandle<()>>>,
    /// The session of a root items build while it runs.
    building: Mutex<Option<Arc<Live>>>,
}

impl JsExtension {
    pub fn new(read: JsManifest, folder: PathBuf, host: Arc<JsHost>) -> Self {
        Self {
            inner: Arc::new(Inner {
                manifest: read.manifest,
                folder,
                refresh: read.refresh,
                host,
                live: Mutex::new(None),
                rows: Arc::default(),
                builder: Mutex::new(None),
                building: Mutex::new(None),
            }),
        }
    }

    pub fn manifest_id(&self) -> &str {
        &self.inner.manifest.id
    }

    pub fn folder(&self) -> &Path {
        &self.inner.folder
    }

    pub fn refresh(&self) -> Option<Duration> {
        self.inner.refresh
    }
}

impl Extension for JsExtension {
    fn manifest(&self) -> &Manifest {
        &self.inner.manifest
    }

    fn activate(&self) -> ActivationResult {
        if self.inner.manifest.root_items {
            let inner = self.inner.clone();
            let builder = self.inner.host.runtime.spawn(async move {
                loop {
                    inner.build_root_items().await;
                    match inner.refresh {
                        Some(every) => tokio::time::sleep(every).await,
                        None => break,
                    }
                }
            });
            if let Some(previous) = self.inner.builder.lock().unwrap().replace(builder) {
                previous.abort();
            }
        }
        Ok(())
    }

    /// Aborting the builder can't stop code running inside it, so its
    /// session is ended too, as is the one behind the extension's views.
    fn deactivate(&self) {
        if let Some(builder) = self.inner.builder.lock().unwrap().take() {
            builder.abort();
        }
        for slot in [&self.inner.building, &self.inner.live] {
            if let Some(live) = slot.lock().unwrap().take() {
                live.end();
            }
        }
    }

    fn root_provider(&self) -> Option<Arc<dyn crate::search::RootProvider>> {
        self.inner
            .manifest
            .root_items
            .then(|| Arc::new(CachedProvider(self.inner.rows.clone())) as _)
    }

    fn command(&self, command_id: &str) -> Option<Arc<dyn Command>> {
        let decl = self
            .inner
            .manifest
            .commands
            .iter()
            .find(|command| command.id == command_id)?;
        Some(Arc::new(JsCommand {
            inner: self.inner.clone(),
            command_id: decl.id.clone(),
            mode: decl.mode,
        }))
    }

    fn perform_action(&self, item_id: &str, action_id: &str, values: &FormValues) -> ActionOutcome {
        let runtime = self.inner.host.runtime.clone();
        if action_id == SEARCH_ACTION {
            return runtime.block_on(self.inner.search(item_id));
        }
        let request = serde_json::json!({
            "itemId": item_id,
            "actionId": action_id,
            "values": values,
        })
        .to_string();
        runtime.block_on(self.inner.action(&request))
    }
}

/// What an action's code returned, as the prelude normalised it.
#[derive(serde::Deserialize, Default)]
struct ActionResult {
    replace: Option<View>,
    ticker: Option<String>,
    #[serde(default)]
    back: bool,
    copy: Option<String>,
    #[serde(default)]
    missing: bool,
}

struct JsCommand {
    inner: Arc<Inner>,
    command_id: String,
    mode: InvocationMode,
}

impl Command for JsCommand {
    fn invoke(&self, ctx: &InvocationContext) {
        let runtime = self.inner.host.runtime.clone();
        runtime.block_on(self.run(ctx));
    }
}

impl JsCommand {
    async fn run(&self, ctx: &InvocationContext) {
        let live = match self.inner.start(Some(ctx.clone())).await {
            Ok(live) => Arc::new(live),
            Err(error) => return self.inner.report(ctx, &self.command_id, error),
        };
        self.inner.adopt(&live);
        let result = live.call(Target::Export(&self.command_id), None).await;
        match (self.mode, result) {
            (_, Err(error)) => {
                live.end();
                self.inner.report(ctx, &self.command_id, error);
            }
            (InvocationMode::NoView, Ok(returned)) => {
                live.end();
                match message(returned.as_deref()) {
                    Some(message) => ctx.succeed_with(message),
                    None => ctx.succeed(),
                }
            }
            (InvocationMode::View, Ok(_)) if !live.pushed() => {
                live.end();
                ctx.succeed();
            }
            (InvocationMode::View, Ok(_)) => {}
        }
    }
}

impl Inner {
    /// Makes `live` the session behind the extension's views, ending the one
    /// before it, and lets it go once its work is over.
    fn adopt(self: &Arc<Self>, live: &Arc<Live>) {
        if let Some(previous) = self.live.lock().unwrap().replace(live.clone()) {
            previous.end();
        }
        let inner = Arc::downgrade(self);
        live.watch(move |ended| {
            let Some(inner) = inner.upgrade() else { return };
            let mut slot = inner.live.lock().unwrap();
            if slot.as_ref().is_some_and(|live| Arc::ptr_eq(live, ended)) {
                *slot = None;
            }
        });
    }

    /// Ends `live` after `UNWATCHED_LIMIT` unless it has become the session
    /// behind the extension's views, which the watcher follows instead.
    fn limit(self: &Arc<Self>, live: &Arc<Live>) {
        let inner = Arc::downgrade(self);
        let live = Arc::downgrade(live);
        self.host.runtime.spawn(async move {
            tokio::time::sleep(UNWATCHED_LIMIT).await;
            let Some(live) = live.upgrade() else { return };
            let adopted = inner.upgrade().is_some_and(|inner| {
                let slot = inner.live.lock().unwrap();
                slot.as_ref().is_some_and(|shown| Arc::ptr_eq(shown, &live))
            });
            if !adopted {
                live.end();
            }
        });
    }

    /// Runs an action in the session behind the extension's views, or in a
    /// fresh one when none is live, as for a root item.
    async fn action(self: &Arc<Self>, request: &str) -> ActionOutcome {
        let live = self
            .live
            .lock()
            .unwrap()
            .clone()
            .filter(|live| !live.is_ended());
        let (live, fresh) = match live {
            Some(live) => (live, false),
            None => match self.start(None).await {
                Ok(live) => {
                    let live = Arc::new(live);
                    self.limit(&live);
                    (live, true)
                }
                Err(error) => return self.action_failed(error),
            },
        };
        live.bridge().begin_action();
        let result = live
            .call(Target::Global("__dangoAction"), Some(request))
            .await;
        let opened = live.bridge().finish_action();
        if fresh {
            match opened {
                true => self.adopt(&live),
                false => live.end(),
            }
        }
        let returned = match result {
            Ok(returned) => returned,
            Err(error) => return self.action_failed(error),
        };
        if opened {
            return ActionOutcome::Started;
        }
        let result: ActionResult = returned
            .as_deref()
            .and_then(|json| serde_json::from_str(json).ok())
            .unwrap_or_default();
        if result.missing {
            return ActionOutcome::Failed(NO_ACTION.into());
        }
        if let Some(view) = result.replace {
            return ActionOutcome::Replaced(Box::new(ViewTree {
                protocol_version: PROTOCOL_VERSION,
                view,
            }));
        }
        if let Some(message) = result.ticker {
            return ActionOutcome::Ticker(message);
        }
        if let Some(text) = result.copy {
            return ActionOutcome::CopyToClipboard(text);
        }
        if result.back {
            return ActionOutcome::Back;
        }
        ActionOutcome::Done
    }

    /// Hands a query to the search handler of the session's view. Its answer
    /// replaces the view; no answer, as for a superseded search, changes
    /// nothing.
    async fn search(&self, query: &str) -> ActionOutcome {
        let Some(live) = self
            .live
            .lock()
            .unwrap()
            .clone()
            .filter(|live| !live.is_ended())
        else {
            return ActionOutcome::Started;
        };
        let request = serde_json::json!({ "query": query }).to_string();
        match live
            .call(Target::Global("__dangoSearch"), Some(&request))
            .await
        {
            Ok(Some(json)) => match serde_json::from_str::<View>(&json) {
                Ok(view) => ActionOutcome::Replaced(Box::new(ViewTree {
                    protocol_version: PROTOCOL_VERSION,
                    view,
                })),
                Err(_) => ActionOutcome::Started,
            },
            Ok(None) => ActionOutcome::Started,
            Err(error) => self.action_failed(error),
        }
    }

    fn action_failed(&self, error: JsError) -> ActionOutcome {
        let id = &self.manifest.id;
        match error {
            JsError::Interrupted => ActionOutcome::Started,
            JsError::Failure(message) => ActionOutcome::Failed(message),
            JsError::MissingExport(_) => ActionOutcome::Failed(NO_ACTION.into()),
            JsError::OutOfMemory => {
                crate::log::append(&format!("extension {id}: an action ran out of memory"));
                ActionOutcome::Failed(OUT_OF_MEMORY.into())
            }
            JsError::Exception { message, stack } => {
                let stack = stack.unwrap_or_default();
                crate::log::append(&format!(
                    "extension {id}: an action failed: {message}
{stack}"
                ));
                ActionOutcome::Failed(STOPPED_BY_ERROR.into())
            }
            JsError::Engine(message) => {
                crate::log::append(&format!("extension {id}: an action failed: {message}"));
                ActionOutcome::Failed(STOPPED_BY_ERROR.into())
            }
        }
    }

    /// Calls `rootItems()` in a session of its own and keeps the rows. A
    /// failed build keeps the rows from the last good one.
    async fn build_root_items(self: &Arc<Self>) {
        let id = &self.manifest.id;
        let built = match self.start(None).await {
            Ok(live) => {
                let live = Arc::new(live);
                *self.building.lock().unwrap() = Some(live.clone());
                self.limit(&live);
                let result = live.call(Target::Export("rootItems"), None).await;
                live.end();
                self.building.lock().unwrap().take();
                result
            }
            Err(error) => Err(error),
        };
        let rows = built.and_then(|json| {
            serde_json::from_str::<Vec<RootItem>>(json.as_deref().unwrap_or("[]"))
                .map_err(|error| JsError::Engine(format!("rootItems returned bad rows: {error}")))
        });
        match rows {
            Ok(rows) => self.rows.replace(
                rows.into_iter()
                    .map(|row| row.candidate(id, &self.folder))
                    .collect(),
            ),
            Err(error) => crate::log::append(&format!(
                "extension {id}: root items weren't built: {error}"
            )),
        }
    }

    async fn start(&self, context: Option<InvocationContext>) -> Result<Live, JsError> {
        let source = std::fs::read_to_string(self.folder.join(MAIN_FILE))
            .map_err(|error| JsError::Engine(format!("{MAIN_FILE} can't be read: {error}")))?;
        Live::start(
            &source,
            &self.manifest.id,
            self.folder.clone(),
            self.host.clone(),
            context,
        )
        .await
    }

    fn report(&self, ctx: &InvocationContext, command_id: &str, error: JsError) {
        let id = &self.manifest.id;
        match error {
            JsError::Interrupted => {}
            JsError::Failure(message) => ctx.fail(message),
            JsError::MissingExport(_) => ctx.fail(NO_CODE),
            JsError::OutOfMemory => {
                crate::log::append(&format!("extension {id}: {command_id} ran out of memory"));
                ctx.fail(OUT_OF_MEMORY);
            }
            JsError::Exception { message, stack } => {
                let stack = stack.unwrap_or_default();
                crate::log::append(&format!(
                    "extension {id}: {command_id} failed: {message}\n{stack}"
                ));
                ctx.fail(STOPPED_BY_ERROR);
            }
            JsError::Engine(message) => {
                crate::log::append(&format!("extension {id}: {command_id} failed: {message}"));
                ctx.fail(STOPPED_BY_ERROR);
            }
        }
    }
}

/// A returned string is the success message; anything else is no message.
fn message(returned: Option<&str>) -> Option<String> {
    serde_json::from_str::<String>(returned?).ok()
}

#[cfg(test)]
pub(crate) mod tests {
    use std::sync::Mutex;

    use super::*;
    use crate::invocation::{Outcome, Sink};
    use crate::js::load::tests::{temp, test_host, write_extension};
    use crate::protocol::{View, ViewTree};

    #[derive(Default)]
    pub struct Recorded {
        pub views: Mutex<Vec<ViewTree>>,
        pub outcome: Mutex<Option<Outcome>>,
        pub cancelled: std::sync::atomic::AtomicBool,
    }

    impl Recorded {
        pub fn cancel(&self) {
            self.cancelled
                .store(true, std::sync::atomic::Ordering::SeqCst);
        }
    }

    impl Sink for Recorded {
        fn superseded(&self) -> bool {
            self.cancelled.load(std::sync::atomic::Ordering::SeqCst)
        }
        fn push_view(&self, tree: ViewTree) {
            self.views.lock().unwrap().push(tree);
        }
        fn finish(&self, outcome: Outcome) {
            *self.outcome.lock().unwrap() = Some(outcome);
        }
    }

    pub fn runtime() -> tokio::runtime::Runtime {
        tokio::runtime::Runtime::new().unwrap()
    }

    pub fn extension(runtime: &tokio::runtime::Runtime, mode: &str, main: &str) -> JsExtension {
        extension_with(test_host(runtime.handle().clone()), mode, main)
    }

    pub fn extension_with(host: Arc<JsHost>, mode: &str, main: &str) -> JsExtension {
        let dir = temp();
        let folder = write_extension(&dir, "ext", main);
        if mode == "no-view" {
            let manifest = std::fs::read_to_string(folder.join("manifest.json")).unwrap();
            std::fs::write(
                folder.join("manifest.json"),
                manifest.replace(r#""mode": "view""#, r#""mode": "no-view""#),
            )
            .unwrap();
        }
        crate::js::load::load(&folder, &host).unwrap().unwrap()
    }

    pub fn invoke(extension: &JsExtension) -> Arc<Recorded> {
        let recorded = Arc::new(Recorded::default());
        let ctx = InvocationContext::for_test("ext.run", recorded.clone());
        extension.command("run").unwrap().invoke(&ctx);
        recorded
    }

    #[test]
    fn a_view_command_shows_loading_then_content() {
        let runtime = runtime();
        let extension = extension(
            &runtime,
            "view",
            r#"export async function run() {
                dango.push({ kind: "list", loading: true, items: [] });
                await new Promise(resolve => setTimeout(resolve, 10));
                dango.push({ kind: "list", items: [1, 2, 3, 4, 5].map(n => ({ id: `${n}`, title: `Item ${n}` })) });
            }"#,
        );

        let recorded = invoke(&extension);

        let views = recorded.views.lock().unwrap();
        assert_eq!(views.len(), 2);
        let (View::List(first), View::List(second)) = (&views[0].view, &views[1].view) else {
            panic!("expected two lists");
        };
        assert!(first.loading);
        assert_eq!(second.items.len(), 5);
        assert_eq!(*recorded.outcome.lock().unwrap(), None);
    }

    #[test]
    fn a_no_view_command_succeeds_with_its_message() {
        let runtime = runtime();
        let extension = extension(
            &runtime,
            "no-view",
            "export function run() { return 'VPN connected'; }",
        );

        let recorded = invoke(&extension);

        assert_eq!(
            *recorded.outcome.lock().unwrap(),
            Some(Outcome::Success(Some("VPN connected".into())))
        );
    }

    #[test]
    fn a_reported_failure_is_shown_as_given() {
        let runtime = runtime();
        let extension = extension(
            &runtime,
            "no-view",
            "export function run() { dango.fail('VPN is not installed'); }",
        );

        let recorded = invoke(&extension);

        assert_eq!(
            *recorded.outcome.lock().unwrap(),
            Some(Outcome::Failure("VPN is not installed".into()))
        );
    }

    #[test]
    fn a_command_without_code_says_so() {
        let runtime = runtime();
        let extension = extension(&runtime, "no-view", "export const nothing = 1;");

        let recorded = invoke(&extension);

        assert_eq!(
            *recorded.outcome.lock().unwrap(),
            Some(Outcome::Failure(NO_CODE.into()))
        );
    }

    #[test]
    fn an_uncaught_error_shows_the_generic_message() {
        let runtime = runtime();
        let extension = extension(
            &runtime,
            "no-view",
            "export async function run() { throw new TypeError('bad'); }",
        );

        let recorded = invoke(&extension);

        assert_eq!(
            *recorded.outcome.lock().unwrap(),
            Some(Outcome::Failure(STOPPED_BY_ERROR.into()))
        );
    }

    #[test]
    fn running_out_of_memory_says_so() {
        let runtime = runtime();
        let extension = extension(
            &runtime,
            "no-view",
            "export function run() { const all = []; for (;;) all.push('x'.repeat(1 << 20)); }",
        );

        let recorded = invoke(&extension);

        assert_eq!(
            *recorded.outcome.lock().unwrap(),
            Some(Outcome::Failure(OUT_OF_MEMORY.into()))
        );
    }

    #[test]
    fn a_looping_command_stops_once_abandoned() {
        let runtime = runtime();
        let extension = Arc::new(extension(
            &runtime,
            "view",
            "export function run() { for (;;) {} }",
        ));
        let recorded = Arc::new(Recorded::default());
        let ctx = InvocationContext::for_test("ext.run", recorded.clone());
        let running = {
            let extension = extension.clone();
            std::thread::spawn(move || extension.command("run").unwrap().invoke(&ctx))
        };

        std::thread::sleep(std::time::Duration::from_millis(100));
        recorded.cancel();

        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while !running.is_finished() {
            assert!(
                std::time::Instant::now() < deadline,
                "the loop kept running"
            );
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
    }

    #[test]
    fn a_timer_never_pushes_after_the_view_is_left() {
        let runtime = runtime();
        let extension = extension(
            &runtime,
            "view",
            r#"export function run() {
                dango.push({ kind: "list", items: [] });
                setTimeout(() => dango.push({ kind: "list", items: [] }), 300);
            }"#,
        );

        let recorded = invoke(&extension);
        recorded.cancel();
        std::thread::sleep(std::time::Duration::from_millis(600));

        assert_eq!(recorded.views.lock().unwrap().len(), 1);
    }

    #[test]
    fn console_calls_reach_the_log_without_failing() {
        let runtime = runtime();
        let extension = extension(
            &runtime,
            "no-view",
            "export function run() { console.log('hi', { n: 1 }); console.error('bad'); return 'ok'; }",
        );

        let recorded = invoke(&extension);

        assert_eq!(
            *recorded.outcome.lock().unwrap(),
            Some(Outcome::Success(Some("ok".into())))
        );
    }

    #[test]
    fn exec_hands_a_program_s_output_to_the_extension() {
        let runtime = runtime();
        let call = if cfg!(windows) {
            "dango.exec('cmd', ['/c', 'echo hi'])"
        } else {
            "dango.exec('sh', ['-c', 'echo hi'])"
        };
        let extension = extension(
            &runtime,
            "no-view",
            &format!("export async function run() {{ const r = await {call}; return `${{r.code}} ${{r.stdout.trim()}}`; }}"),
        );

        let recorded = invoke(&extension);

        assert_eq!(
            *recorded.outcome.lock().unwrap(),
            Some(Outcome::Success(Some("0 hi".into())))
        );
    }

    #[test]
    fn open_copy_paste_and_files_reach_the_system() {
        let runtime = runtime();
        let calls: Arc<Mutex<Vec<String>>> = Arc::default();
        let record = |name: &'static str| {
            let calls = calls.clone();
            Arc::new(move |text: &str| {
                calls.lock().unwrap().push(format!("{name} {text}"));
                Ok(())
            }) as Arc<crate::js::host::TextAction>
        };
        let host = Arc::new(JsHost {
            open: record("open"),
            copy: record("copy"),
            paste: record("paste"),
            ..Arc::into_inner(test_host(runtime.handle().clone())).unwrap()
        });
        let extension = extension_with(
            host,
            "no-view",
            r#"export async function run() {
                await dango.open("https://example.com");
                await dango.copy("copied");
                await dango.paste("pasted");
                const file = dango.env.folder + "/note.txt";
                await dango.fs.writeText(file, "kept");
                return `${dango.env.id} ${await dango.fs.readText(file)}`;
            }"#,
        );

        let recorded = invoke(&extension);

        assert_eq!(
            *recorded.outcome.lock().unwrap(),
            Some(Outcome::Success(Some("ext kept".into())))
        );
        assert_eq!(
            *calls.lock().unwrap(),
            vec!["open https://example.com", "copy copied", "paste pasted"]
        );
    }

    #[test]
    fn storage_keeps_values_per_extension() {
        let runtime = runtime();
        let extension = extension(
            &runtime,
            "no-view",
            r#"export async function run() {
                await dango.storage.set("count", { n: 2 });
                const value = await dango.storage.get("count");
                const missing = await dango.storage.get("nope");
                return `${value.n} ${missing} ${(await dango.storage.keys()).join()}`;
            }"#,
        );

        let recorded = invoke(&extension);

        assert_eq!(
            *recorded.outcome.lock().unwrap(),
            Some(Outcome::Success(Some("2 null count".into())))
        );
    }

    #[test]
    fn preferences_reach_the_next_session() {
        let runtime = runtime();
        let user = Arc::new(Mutex::new("nico"));
        let read = user.clone();
        let host = Arc::new(JsHost {
            preferences: Arc::new(move |_| {
                let mut map = serde_json::Map::new();
                map.insert("user".into(), (*read.lock().unwrap()).into());
                map
            }),
            ..Arc::into_inner(test_host(runtime.handle().clone())).unwrap()
        });
        let extension = extension_with(
            host,
            "no-view",
            "export function run() { return dango.preferences.user; }",
        );

        let first = invoke(&extension);
        *user.lock().unwrap() = "anna";
        let second = invoke(&extension);

        assert_eq!(
            *first.outcome.lock().unwrap(),
            Some(Outcome::Success(Some("nico".into())))
        );
        assert_eq!(
            *second.outcome.lock().unwrap(),
            Some(Outcome::Success(Some("anna".into())))
        );
    }

    #[test]
    fn a_secret_reaches_its_extension_and_a_broken_file_can_be_caught() {
        let runtime = runtime();
        let host = Arc::new(JsHost {
            secret: Arc::new(|id, name| match (id, name) {
                ("ext", "token") => Ok(Some("glpat-123".into())),
                ("ext", "broken") => Err("auth.toml: line 2 isn't valid TOML".into()),
                _ => Ok(None),
            }),
            ..Arc::into_inner(test_host(runtime.handle().clone())).unwrap()
        });
        let extension = extension_with(
            host,
            "no-view",
            r#"export async function run() {
                let caught = "";
                try { await dango.secret("broken"); } catch (error) { caught = error.message; }
                return `${await dango.secret("token")} ${await dango.secret("other")} ${caught}`;
            }"#,
        );

        let recorded = invoke(&extension);

        assert_eq!(
            *recorded.outcome.lock().unwrap(),
            Some(Outcome::Success(Some(
                "glpat-123 undefined auth.toml can't be read".into()
            )))
        );
    }

    fn action_ids(recorded: &Recorded) -> Vec<String> {
        let views = recorded.views.lock().unwrap();
        let View::List(list) = &views[0].view else {
            panic!("expected a list");
        };
        list.items[0]
            .actions
            .iter()
            .map(|action| action.id.clone())
            .collect()
    }

    #[test]
    fn an_action_s_result_becomes_its_outcome() {
        let runtime = runtime();
        let extension = extension(
            &runtime,
            "view",
            r#"export function run() {
                const act = (title, onAction) => ({ title, onAction });
                dango.push({ kind: "list", items: [{ id: "a", title: "A", actions: [
                    act("Replace", () => ({ replace: { kind: "list", items: [] } })),
                    act("Ticker", () => ({ ticker: "Saved" })),
                    act("Back", () => ({ back: true })),
                    act("Done", () => {}),
                    act("Copy", () => ({ copy: "text" })),
                    act("Fail", () => dango.fail("No luck")),
                ] }] });
            }"#,
        );
        let recorded = invoke(&extension);
        let ids = action_ids(&recorded);
        let run = |index: usize| extension.perform_action("a", &ids[index], &FormValues::new());

        assert!(matches!(run(0), ActionOutcome::Replaced(_)));
        assert_eq!(run(1), ActionOutcome::Ticker("Saved".into()));
        assert_eq!(run(2), ActionOutcome::Back);
        assert_eq!(run(3), ActionOutcome::Done);
        assert_eq!(run(4), ActionOutcome::CopyToClipboard("text".into()));
        assert_eq!(run(5), ActionOutcome::Failed("No luck".into()));
    }

    #[test]
    fn an_unknown_action_is_not_available() {
        let runtime = runtime();
        let extension = extension(&runtime, "view", "export function run() {}");

        assert_eq!(
            extension.perform_action("a", "nope", &FormValues::new()),
            ActionOutcome::Failed(NO_ACTION.into())
        );
    }

    #[test]
    fn a_pushed_view_keeps_the_session_for_the_list_below() {
        use std::sync::atomic::{AtomicBool, Ordering};

        let runtime = runtime();
        let detail = Arc::new(Recorded::default());
        let opened = detail.clone();
        let showing = Arc::new(AtomicBool::new(true));
        let shown = showing.clone();
        let host = Arc::new(JsHost {
            start_view: Arc::new(move |_, _| {
                Some(Arc::new(InvocationContext::for_test("ext", opened.clone())))
            }),
            showing: Arc::new(move |_| shown.load(Ordering::SeqCst)),
            ..Arc::into_inner(test_host(runtime.handle().clone())).unwrap()
        });
        let extension = extension_with(
            host,
            "view",
            r##"export function run() {
                dango.push({ kind: "list", items: [{ id: "a", title: "A", actions: [
                    { title: "Open", onAction: () => dango.push({ kind: "detail", markdown: "# A" }) },
                    { title: "Ticker", onAction: async () => {
                        await new Promise(resolve => setTimeout(resolve, 200));
                        return { ticker: "Still here" };
                    } },
                ] }] });
            }"##,
        );
        let list = invoke(&extension);
        let ids = action_ids(&list);

        let pushed = extension.perform_action("a", &ids[0], &FormValues::new());
        list.cancel();
        detail.cancel();
        std::thread::sleep(std::time::Duration::from_millis(200));
        let again = extension.perform_action("a", &ids[1], &FormValues::new());

        assert_eq!(pushed, ActionOutcome::Started);
        assert!(matches!(
            detail.views.lock().unwrap()[0].view,
            View::Detail(_)
        ));
        assert_eq!(again, ActionOutcome::Ticker("Still here".into()));
    }

    #[test]
    fn a_form_submit_hands_its_values_to_the_handler() {
        let runtime = runtime();
        let extension = extension(
            &runtime,
            "view",
            r#"export function run() {
                dango.push({ kind: "form", fields: [{ id: "title", label: "Title", kind: "text" }],
                    actions: [{ title: "Create", onAction: ({ values }) => ({ ticker: values.title }) }] });
            }"#,
        );
        let recorded = invoke(&extension);
        let id = {
            let views = recorded.views.lock().unwrap();
            let View::Form(form) = &views[0].view else {
                panic!("expected a form");
            };
            form.actions[0].id.clone()
        };
        let values = FormValues::from([("title".to_string(), "Fix login".to_string())]);

        assert_eq!(
            extension.perform_action("", &id, &values),
            ActionOutcome::Ticker("Fix login".into())
        );
    }

    fn rooted(host: Arc<JsHost>, main: &str, refresh: Option<std::time::Duration>) -> JsExtension {
        let dir = temp();
        let folder = write_extension(&dir, "ext", main);
        let text = r#"{ "manifestVersion": 1, "id": "ext", "name": "Test", "rootItems": {} }"#;
        let mut read = crate::js::manifest::parse(text, &folder).unwrap();
        read.refresh = refresh;
        JsExtension::new(read, folder, host)
    }

    async fn rows_after(
        provider: &Arc<dyn crate::search::RootProvider>,
        wanted: usize,
    ) -> Vec<String> {
        for _ in 0..200 {
            let rows = provider.items(String::new()).await;
            if rows.len() >= wanted {
                return rows.into_iter().map(|row| row.title).collect();
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
        Vec::new()
    }

    const COUNTING: &str = r#"
        export async function rootItems() {
            const builds = ((await dango.storage.get("builds")) ?? 0) + 1;
            await dango.storage.set("builds", builds);
            if (builds === 2) throw new Error("flaky");
            return [{ id: `repo-${builds}`, title: `Repo ${builds}`, actions: [{ id: "open", title: "Open" }] }];
        }
        export function onAction(itemId, actionId) { return { ticker: `${itemId} ${actionId}` }; }
    "#;

    fn builds(host: &JsHost) -> String {
        host.store
            .as_ref()
            .unwrap()
            .extension_value("ext", "builds")
            .unwrap()
            .unwrap_or_default()
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn root_items_are_built_once_and_answer_queries_without_code() {
        let host = test_host(tokio::runtime::Handle::current());
        let extension = rooted(host.clone(), COUNTING, None);
        extension.activate().unwrap();
        let provider = extension.root_provider().unwrap();

        let rows = rows_after(&provider, 1).await;
        for query in [
            "r", "re", "rep", "repo", "repo ", "repo 1", "x", "xy", "xyz", "q",
        ] {
            provider.items(query.into()).await;
        }

        assert_eq!(rows, vec!["Repo 1"]);
        assert_eq!(builds(&host), "1");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn root_items_refresh_and_a_failed_build_keeps_the_last_rows() {
        let host = test_host(tokio::runtime::Handle::current());
        let extension = rooted(
            host.clone(),
            COUNTING,
            Some(std::time::Duration::from_millis(100)),
        );
        extension.activate().unwrap();
        let provider = extension.root_provider().unwrap();

        assert_eq!(rows_after(&provider, 1).await, vec!["Repo 1"]);
        for _ in 0..200 {
            if builds(&host) == "2" {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
        let after_failure = rows_after(&provider, 1).await;
        for _ in 0..200 {
            if provider.items(String::new()).await[0].title == "Repo 3" {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
        extension.deactivate();

        assert_eq!(after_failure, vec!["Repo 1"]);
        assert_eq!(provider.items(String::new()).await[0].title, "Repo 3");
    }

    #[test]
    fn enter_on_a_root_item_calls_on_action() {
        let runtime = runtime();
        let extension = rooted(test_host(runtime.handle().clone()), COUNTING, None);

        assert_eq!(
            extension.perform_action("repo-1", "open", &FormValues::new()),
            ActionOutcome::Ticker("repo-1 open".into())
        );
    }

    #[test]
    fn a_superseded_search_answers_nothing() {
        let runtime = runtime();
        let extension = Arc::new(extension(
            &runtime,
            "view",
            r#"export function run() {
                dango.push({ kind: "list", items: [], onSearch: async (query) => {
                    await new Promise(resolve => setTimeout(resolve, query === "ru" ? 300 : 10));
                    return { kind: "list", items: [{ id: query, title: query }] };
                } });
            }"#,
        ));
        let recorded = invoke(&extension);
        {
            let views = recorded.views.lock().unwrap();
            let View::List(list) = &views[0].view else {
                panic!("expected a list");
            };
            assert_eq!(list.filtering, crate::protocol::Filtering::Command);
        }

        let slow = {
            let extension = extension.clone();
            std::thread::spawn(move || {
                extension.perform_action("ru", SEARCH_ACTION, &FormValues::new())
            })
        };
        std::thread::sleep(std::time::Duration::from_millis(50));
        let fast = extension.perform_action("rust", SEARCH_ACTION, &FormValues::new());
        let slow = slow.join().unwrap();

        let ActionOutcome::Replaced(tree) = fast else {
            panic!("expected the newer search to answer, got {fast:?}");
        };
        let View::List(list) = tree.view else {
            panic!("expected a list");
        };
        assert_eq!(list.items[0].id, "rust");
        assert_eq!(slow, ActionOutcome::Started);
    }

    #[test]
    fn an_action_cut_short_by_leaving_changes_nothing() {
        let runtime = runtime();
        let extension = extension(
            &runtime,
            "view",
            r#"export function run() {
                dango.push({ kind: "list", items: [{ id: "a", title: "A", actions: [
                    { title: "Slow", onAction: () => new Promise(resolve => setTimeout(resolve, 5000)) },
                ] }] });
            }"#,
        );
        let recorded = invoke(&extension);
        let ids = action_ids(&recorded);
        recorded.cancel();

        let started = std::time::Instant::now();
        let outcome = extension.perform_action("a", &ids[0], &FormValues::new());

        assert_eq!(outcome, ActionOutcome::Started);
        assert!(started.elapsed() < std::time::Duration::from_secs(2));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn deactivating_stops_a_looping_root_items_build() {
        let host = test_host(tokio::runtime::Handle::current());
        let extension = rooted(host, "export function rootItems() { for (;;) {} }", None);
        extension.activate().unwrap();
        for _ in 0..100 {
            if extension.inner.building.lock().unwrap().is_some() {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
        let building = extension.inner.building.lock().unwrap().clone().unwrap();

        extension.deactivate();

        for _ in 0..100 {
            if Arc::strong_count(&building) == 1 {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
        assert!(building.is_ended());
        assert_eq!(Arc::strong_count(&building), 1, "the build kept running");
    }

    #[test]
    fn a_looping_root_action_is_stopped_after_its_limit() {
        let runtime = runtime();
        let extension = rooted(
            test_host(runtime.handle().clone()),
            "export function onAction() { for (;;) {} }",
            None,
        );

        let started = std::time::Instant::now();
        let outcome = extension.perform_action("repo-1", "open", &FormValues::new());

        assert_eq!(outcome, ActionOutcome::Started);
        assert!(started.elapsed() < UNWATCHED_LIMIT * 3);
    }

    #[test]
    fn deactivating_ends_the_session_behind_the_views() {
        let runtime = runtime();
        let extension = extension(
            &runtime,
            "view",
            r#"export function run() { dango.push({ kind: "list", items: [] }); setInterval(() => {}, 10); }"#,
        );
        invoke(&extension);
        let live = extension.inner.live.lock().unwrap().clone().unwrap();

        extension.deactivate();

        assert!(live.is_ended());
        assert!(extension.inner.live.lock().unwrap().is_none());
    }
}
