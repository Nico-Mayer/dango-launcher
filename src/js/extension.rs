use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::extension::manifest::{InvocationMode, Manifest};
use crate::extension::Extension;
use crate::invocation::{Command, InvocationContext};

use super::host::JsHost;
use super::live::Live;
use super::load::MAIN_FILE;
use super::manifest::JsManifest;
use super::session::{JsError, Target};

pub const NO_CODE: &str = "This command's extension has no code for it";
pub const STOPPED_BY_ERROR: &str = "This command stopped because of an error, see dango.log";
pub const OUT_OF_MEMORY: &str = "This command used too much memory, so Dango stopped it";

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

    async fn start(&self, context: Option<InvocationContext>) -> Result<Live, JsError> {
        let source = std::fs::read_to_string(self.folder.join(MAIN_FILE))
            .map_err(|error| JsError::Engine(format!("{MAIN_FILE} can't be read: {error}")))?;
        Live::start(&source, &self.manifest.id, self.host.clone(), context).await
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
        crate::js::load::load(&folder, &test_host(runtime.handle().clone()))
            .unwrap()
            .unwrap()
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
}
