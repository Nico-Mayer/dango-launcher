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
            CaughtError::Exception(exception) => Self::Exception {
                message: exception.message().unwrap_or_default(),
                stack: exception.stack(),
            },
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
    pub async fn start(source: &str, cancel: Arc<AtomicBool>) -> Result<Self, JsError> {
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
        export: &str,
        argument: Option<&str>,
    ) -> Result<Option<String>, JsError> {
        let export = export.to_owned();
        let argument = argument.map(str::to_owned);
        self.context
            .async_with(async move |ctx| call(&ctx, &export, argument).await)
            .await
            .map_err(|error| error.classify(&self.cancel))
    }
}

async fn call<'js>(
    ctx: &Ctx<'js>,
    export: &str,
    argument: Option<String>,
) -> Result<Option<String>, JsError> {
    let main: Object = ctx.globals().get(MAIN)?;
    let value: Value = main.get(export)?;
    let Some(function) = value.as_function() else {
        return Err(JsError::MissingExport(export.to_owned()));
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
        )
        .await
        .unwrap();

        let result = session.call("greet", Some(r#"{"name":"nico"}"#)).await;

        assert_eq!(result, Ok(Some(r#"{"text":"hi nico"}"#.to_string())));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn returning_nothing_is_none() {
        let session = Session::start("export function quiet() {}", Arc::default())
            .await
            .unwrap();

        assert_eq!(session.call("quiet", None).await, Ok(None));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn a_missing_export_is_named() {
        let session = Session::start("export const x = 1;", Arc::default())
            .await
            .unwrap();

        assert_eq!(
            session.call("run", None).await,
            Err(JsError::MissingExport("run".into()))
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn a_thrown_error_keeps_its_message() {
        let session = Session::start(
            "export function boom() { throw new TypeError('bad'); }",
            Arc::default(),
        )
        .await
        .unwrap();

        let Err(JsError::Exception { message, stack }) = session.call("boom", None).await else {
            panic!("expected an exception");
        };
        assert_eq!(message, "bad");
        assert!(stack.is_some());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn cancelling_stops_an_endless_loop() {
        let cancel = Arc::new(AtomicBool::new(false));
        let session = Session::start("export function spin() { for (;;) {} }", cancel.clone())
            .await
            .unwrap();
        let flag = cancel.clone();
        tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
            flag.store(true, Ordering::SeqCst);
        });

        assert_eq!(session.call("spin", None).await, Err(JsError::Interrupted));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn allocating_past_the_limit_is_out_of_memory() {
        let session = Session::start(
            "export function hog() { const all = []; for (;;) all.push('x'.repeat(1 << 20)); }",
            Arc::default(),
        )
        .await
        .unwrap();

        assert_eq!(session.call("hog", None).await, Err(JsError::OutOfMemory));
    }
}
