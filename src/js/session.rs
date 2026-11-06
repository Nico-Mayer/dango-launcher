use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use rquickjs::function::Rest;
use rquickjs::{
    AsyncContext, AsyncRuntime, CatchResultExt, CaughtError, Ctx, Module, Object, Value,
};

/// The global the evaluated `main.js` namespace is kept under, so later calls
/// into the same session find its exports without holding a `'js` value
/// outside the context.
const MAIN: &str = "__dangoMain";

const PRELUDE: &str = include_str!("prelude.js");

/// The Rust side of the `dango` global: the functions the prelude and
/// extension code call into, put on `__dangoNative` before `main.js` runs.
pub trait Natives: Send + Sync + 'static {
    fn install<'js>(self: Arc<Self>, ctx: &Ctx<'js>, native: &Object<'js>) -> rquickjs::Result<()>;
}

/// What a call runs: an export of `main.js`, or a function the prelude put on
/// the global object.
#[derive(Clone, Copy, Debug)]
pub enum Target<'a> {
    Export(&'a str),
    Global(&'a str),
}

/// What one session may allocate. Generous for list building and JSON, small
/// enough that a runaway extension can't take the machine's memory with it.
const MEMORY_LIMIT: usize = 64 * 1024 * 1024;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum JsError {
    #[error("{message}")]
    Exception {
        message: String,
        stack: Option<String>,
    },
    /// Extension code reported a failure on purpose, through `dango.fail`.
    #[error("{0}")]
    Failure(String),
    #[error("main.js exports no function named '{0}'")]
    MissingExport(String),
    #[error("the session ran out of memory")]
    OutOfMemory,
    #[error("the session was stopped")]
    Interrupted,
    #[error("{0}")]
    Engine(String),
}

impl JsError {
    /// QuickJS reports both a stop from the interrupt handler and a failed
    /// allocation as ordinary exceptions, so they are told apart here.
    fn classify(self, cancel: &AtomicBool) -> Self {
        if cancel.load(Ordering::SeqCst) {
            return Self::Interrupted;
        }
        match &self {
            Self::Exception { message, .. } if message.contains("out of memory") => {
                Self::OutOfMemory
            }
            Self::Engine(message) if message.contains("out of memory") => Self::OutOfMemory,
            _ => self,
        }
    }
}

impl From<rquickjs::Error> for JsError {
    fn from(error: rquickjs::Error) -> Self {
        Self::Engine(error.to_string())
    }
}

impl From<CaughtError<'_>> for JsError {
    fn from(error: CaughtError<'_>) -> Self {
        match error {
            CaughtError::Exception(exception) => {
                let message = exception.message().unwrap_or_default();
                let name: Option<String> = exception.as_object().get("name").ok();
                if name.as_deref() == Some("DangoFailure") {
                    return Self::Failure(message);
                }
                Self::Exception {
                    message,
                    stack: exception.stack(),
                }
            }
            CaughtError::Value(value) => Self::Exception {
                message: format!("{value:?}"),
                stack: None,
            },
            CaughtError::Error(error) => error.into(),
        }
    }
}

/// One unit of an extension's work: a fresh runtime with `main.js` loaded.
/// Dropping it drops the runtime, and with it every timer and pending future.
pub struct Session {
    context: AsyncContext,
    cancel: Arc<AtomicBool>,
    driver: tokio::task::JoinHandle<()>,
    _runtime: AsyncRuntime,
}

impl Session {
    /// Loads `main.js`. Setting `cancel` stops any running code at its next
    /// bytecode check, which is how an endless loop is ended.
    pub async fn start(
        source: &str,
        cancel: Arc<AtomicBool>,
        natives: Arc<dyn Natives>,
    ) -> Result<Self, JsError> {
        let runtime = AsyncRuntime::new()?;
        runtime.set_memory_limit(MEMORY_LIMIT).await;
        let flag = cancel.clone();
        runtime
            .set_interrupt_handler(Some(Box::new(move || flag.load(Ordering::SeqCst))))
            .await;
        let context = AsyncContext::full(&runtime).await?;
        let driver = tokio::spawn(runtime.drive());
        let source = source.to_owned();
        context
            .async_with(async move |ctx| -> Result<(), JsError> {
                rquickjs_extra::timers::init(&ctx)?;
                rquickjs_extra::url::init(&ctx)?;
                let native = Object::new(ctx.clone())?;
                natives.install(&ctx, &native)?;
                ctx.globals().set("__dangoNative", native)?;
                ctx.eval::<(), _>(PRELUDE).catch(&ctx)?;
                let (module, evaluated) = Module::declare(ctx.clone(), "main.js", source)
                    .catch(&ctx)?
                    .eval()
                    .catch(&ctx)?;
                evaluated.into_future::<()>().await.catch(&ctx)?;
                ctx.globals().set(MAIN, module.namespace()?)?;
                Ok(())
            })
            .await
            .map_err(|error| error.classify(&cancel))?;
        Ok(Self {
            context,
            cancel,
            driver,
            _runtime: runtime,
        })
    }

    /// Calls an export with an optional JSON argument and returns its result,
    /// awaited if it is a promise, as JSON. `None` means it returned nothing.
    pub async fn call(
        &self,
        target: Target<'_>,
        argument: Option<&str>,
    ) -> Result<Option<String>, JsError> {
        let target = match target {
            Target::Export(name) => (true, name.to_owned()),
            Target::Global(name) => (false, name.to_owned()),
        };
        let argument = argument.map(str::to_owned);
        self.context
            .async_with(async move |ctx| call(&ctx, target, argument).await)
            .await
            .map_err(|error| error.classify(&self.cancel))
    }
}

async fn call<'js>(
    ctx: &Ctx<'js>,
    (is_export, name): (bool, String),
    argument: Option<String>,
) -> Result<Option<String>, JsError> {
    let holder: Object = if is_export {
        ctx.globals().get(MAIN)?
    } else {
        ctx.globals()
    };
    let value: Value = holder.get(name.as_str())?;
    let Some(function) = value.as_function() else {
        return Err(JsError::MissingExport(name));
    };
    let arguments = match argument {
        Some(json) => vec![ctx.json_parse(json).catch(ctx)?],
        None => Vec::new(),
    };
    let returned: Value = function.call((Rest(arguments),)).catch(ctx)?;
    let result = match returned.as_promise() {
        Some(promise) => promise.clone().into_future::<Value>().await.catch(ctx)?,
        None => returned,
    };
    Ok(ctx
        .json_stringify(result)?
        .map(|json| json.to_string())
        .transpose()?)
}

impl Drop for Session {
    fn drop(&mut self) {
        self.driver.abort();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct NoNatives;

    impl Natives for NoNatives {
        fn install<'js>(self: Arc<Self>, _: &Ctx<'js>, _: &Object<'js>) -> rquickjs::Result<()> {
            Ok(())
        }
    }

    fn no_natives() -> Arc<dyn Natives> {
        Arc::new(NoNatives)
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn dango_fail_is_a_failure_not_an_exception() {
        let session = Session::start(
            "export function run() { dango.fail('VPN is not installed'); }",
            Arc::default(),
            no_natives(),
        )
        .await
        .unwrap();

        assert_eq!(
            session.call(Target::Export("run"), None).await,
            Err(JsError::Failure("VPN is not installed".into()))
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn an_export_awaits_a_timer_and_returns_json() {
        let session = Session::start(
            r#"
            export async function greet(who) {
                await new Promise(resolve => setTimeout(resolve, 10));
                return { text: `hi ${who.name}` };
            }
            "#,
            Arc::default(),
            no_natives(),
        )
        .await
        .unwrap();

        let result = session
            .call(Target::Export("greet"), Some(r#"{"name":"nico"}"#))
            .await;

        assert_eq!(result, Ok(Some(r#"{"text":"hi nico"}"#.to_string())));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn returning_nothing_is_none() {
        let session = Session::start("export function quiet() {}", Arc::default(), no_natives())
            .await
            .unwrap();

        assert_eq!(session.call(Target::Export("quiet"), None).await, Ok(None));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn a_missing_export_is_named() {
        let session = Session::start("export const x = 1;", Arc::default(), no_natives())
            .await
            .unwrap();

        assert_eq!(
            session.call(Target::Export("run"), None).await,
            Err(JsError::MissingExport("run".into()))
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn a_thrown_error_keeps_its_message() {
        let session = Session::start(
            "export function boom() { throw new TypeError('bad'); }",
            Arc::default(),
            no_natives(),
        )
        .await
        .unwrap();

        let Err(JsError::Exception { message, stack }) =
            session.call(Target::Export("boom"), None).await
        else {
            panic!("expected an exception");
        };
        assert_eq!(message, "bad");
        assert!(stack.is_some());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn cancelling_stops_an_endless_loop() {
        let cancel = Arc::new(AtomicBool::new(false));
        let session = Session::start(
            "export function spin() { for (;;) {} }",
            cancel.clone(),
            no_natives(),
        )
        .await
        .unwrap();
        let flag = cancel.clone();
        tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
            flag.store(true, Ordering::SeqCst);
        });

        assert_eq!(
            session.call(Target::Export("spin"), None).await,
            Err(JsError::Interrupted)
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn allocating_past_the_limit_is_out_of_memory() {
        let session = Session::start(
            "export function hog() { const all = []; for (;;) all.push('x'.repeat(1 << 20)); }",
            Arc::default(),
            no_natives(),
        )
        .await
        .unwrap();

        assert_eq!(
            session.call(Target::Export("hog"), None).await,
            Err(JsError::OutOfMemory)
        );
    }
}
