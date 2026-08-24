//! System capability APIs on the `gpui` module: `fs`, `store`, `clipboard` and
//! `log` (design doc §17).
//!
//! Nothing here is available by default (§19.2). The grant lives in a
//! thread-local [`Capabilities`], the host installs one before it loads an
//! application, and every entry point re-reads it at call time — so revoking a
//! capability takes effect on the next call rather than on the next restart.
//!
//! Two rules keep this file honest:
//!
//! - **One path resolver.** Every filesystem path goes through
//!   [`Capabilities::resolve`], never through `std::fs` directly. `gpui.fs` and
//!   the capability-gated `os.*` entry points that arrive later therefore share
//!   one policy, and there is no second place for a traversal bug to hide.
//! - **A denial names its manifest key.** The error a script sees is the
//!   instruction for fixing it: which `capabilities.*` key to declare. Wording
//!   follows [`CapabilityError`], and the cases that resolver does not cover
//!   (clipboard read versus write) are spelled out the same way here.
//!
//! # TODO(M3): these must return promises
//!
//! §17.1 requires the filesystem surface — and `store.flush` — to be
//! asynchronous, because synchronous IO on the render thread stalls the frame.
//! The scheduler that hands out promises is not in place yet, so for now the
//! calls block. Every body is deliberately a capability check plus one
//! `std::fs` call, so the move is mechanical: hand the closure body to
//! `gpui.spawn` and return its promise, changing nothing about the checks.

use std::{
    cell::RefCell,
    path::{Path, PathBuf},
};

use gpui::{App, ClipboardItem};
use rquickjs::{
    Array, Ctx, Error as JsError, Exception, FromJs, IntoJs, Object, Result as JsResult, Value,
    function::{Func, Rest},
};
use serde_json::Value as Json;

use crate::{
    capability::{Access, Capabilities, CapabilityError},
    scope,
};

thread_local! {
    /// The JS VM and GPUI's `App` are both main-thread only, so a thread-local
    /// is the whole story here: no lock, and no `Send` bound forced onto
    /// [`Capabilities`] for the sake of a runtime that never leaves its thread.
    static CAPABILITIES: RefCell<Capabilities> = RefCell::new(Capabilities::default());
    static STORE: RefCell<Option<Store>> = const { RefCell::new(None) };
}

/// Installs the grant the loaded application runs under.
///
/// The host calls this before loading an application. Loading a different
/// application means calling it again; the default is [`Capabilities::default`],
/// which allows nothing.
// `mod host` is private to the engine, so nothing in the crate can call the
// three host-facing entry points below yet; they become reachable when `engine`
// re-exports them for the host binary.
#[allow(dead_code)]
pub fn set_capabilities(capabilities: Capabilities) {
    CAPABILITIES.with_borrow_mut(|current| *current = capabilities);
}

/// The grant in force on this thread.
pub fn capabilities() -> Capabilities {
    CAPABILITIES.with_borrow(Clone::clone)
}

/// Points `gpui.store` at its backing file.
///
/// Each application gets its own file so one cannot read another's settings
/// (§17.3). Setting it drops whatever the previous application had cached, so a
/// reload cannot serve stale values from the file it no longer owns.
#[allow(dead_code)]
pub fn set_store_path(path: PathBuf) {
    STORE.with_borrow_mut(|store| *store = Some(Store::new(path)));
}

/// The context argument is the engine's; the sub-objects are built from the
/// module's own [`Object::ctx`] instead. `Ctx` and `Object` are invariant in
/// `'js`, so the two elided lifetimes in this signature are distinct to the
/// compiler and a value built from one cannot be set on the other — the same
/// constraint that forces conversions into `FromJs`/`IntoJs` elsewhere in the
/// engine. They are the same context at run time.
pub fn install(_ctx: &Ctx<'_>, module: &Object<'_>) -> JsResult<()> {
    let ctx = module.ctx();
    module.set("fs", fs_object(ctx)?)?;
    module.set("store", store_object(ctx)?)?;
    module.set("clipboard", clipboard_object(ctx)?)?;
    module.set("log", log_object(ctx)?)?;
    Ok(())
}

// -- Filesystem ------------------------------------------------------------

fn fs_object<'js>(ctx: &Ctx<'js>) -> JsResult<Object<'js>> {
    let fs = Object::new(ctx.clone())?;

    fs.set(
        "read_text",
        Func::from(|ctx: Ctx<'_>, path: String| -> JsResult<String> {
            let path = resolve(&ctx, &path, Access::Read)?;
            std::fs::read_to_string(&path).map_err(|error| io_error(&ctx, "read", &path, &error))
        }),
    )?;

    fs.set(
        "write_text",
        Func::from(
            |ctx: Ctx<'_>, path: String, contents: String| -> JsResult<()> {
                let path = resolve(&ctx, &path, Access::Write)?;
                std::fs::write(&path, contents)
                    .map_err(|error| io_error(&ctx, "write", &path, &error))
            },
        ),
    )?;

    fs.set(
        "read_dir",
        Func::from(|ctx: Ctx<'_>, path: String| -> JsResult<Vec<DirEntry>> {
            let path = resolve(&ctx, &path, Access::Read)?;
            read_dir(&path).map_err(|error| io_error(&ctx, "list", &path, &error))
        }),
    )?;

    // A denied path throws rather than answering `false`: "you may not look"
    // and "it is not there" are different facts, and collapsing them would let
    // a script probe outside its roots one boolean at a time.
    fs.set(
        "exists",
        Func::from(|ctx: Ctx<'_>, path: String| -> JsResult<bool> {
            Ok(resolve(&ctx, &path, Access::Read)?.exists())
        }),
    )?;

    // Directory removal is not recursive. Write access is granted per root, so
    // a recursive remove would turn one mistyped path into the loss of an
    // application's whole data directory; a script that means it can walk the
    // tree itself.
    fs.set(
        "remove",
        Func::from(|ctx: Ctx<'_>, path: String| -> JsResult<()> {
            let path = resolve(&ctx, &path, Access::Write)?;
            let removed = if path.is_dir() {
                std::fs::remove_dir(&path)
            } else {
                std::fs::remove_file(&path)
            };
            removed.map_err(|error| io_error(&ctx, "remove", &path, &error))
        }),
    )?;

    fs.set(
        "create_dir_all",
        Func::from(|ctx: Ctx<'_>, path: String| -> JsResult<()> {
            let path = resolve(&ctx, &path, Access::Write)?;
            std::fs::create_dir_all(&path).map_err(|error| io_error(&ctx, "create", &path, &error))
        }),
    )?;

    Ok(fs)
}

/// One entry of `fs.read_dir`, as the plain object a script sees.
///
/// `is_dir` rides along because there is no `stat` in this surface, and without
/// it every caller would have to guess from the name.
struct DirEntry {
    name: String,
    is_dir: bool,
}

impl<'js> IntoJs<'js> for DirEntry {
    fn into_js(self, ctx: &Ctx<'js>) -> JsResult<Value<'js>> {
        let object = Object::new(ctx.clone())?;
        object.set("name", self.name)?;
        object.set("is_dir", self.is_dir)?;
        Ok(object.into_value())
    }
}

/// Sorted by name, so a script that renders a listing does not have to sort it
/// and does not inherit the filesystem's arbitrary order.
fn read_dir(path: &Path) -> std::io::Result<Vec<DirEntry>> {
    let mut entries = Vec::new();
    for entry in std::fs::read_dir(path)? {
        let entry = entry?;
        entries.push(DirEntry {
            name: entry.file_name().to_string_lossy().into_owned(),
            is_dir: entry.file_type().map(|kind| kind.is_dir()).unwrap_or(false),
        });
    }
    entries.sort_by(|left, right| left.name.cmp(&right.name));
    Ok(entries)
}

/// The single path gate. Nothing in this module opens a path it did not get
/// back from here.
fn resolve(ctx: &Ctx<'_>, path: &str, access: Access) -> JsResult<PathBuf> {
    capabilities()
        .resolve(Path::new(path), access)
        .map_err(|error| Exception::throw_type(ctx, &denial(&error)))
}

/// Turns a [`CapabilityError`] into a message that always ends in the manifest
/// key to add. `OutsideRoots` on its own says only that the path is out of
/// bounds, which leaves an author guessing at the fix.
fn denial(error: &CapabilityError) -> String {
    match error {
        CapabilityError::OutsideRoots { access, .. } => format!(
            "{error}; add its directory to capabilities.fs.{} in the manifest",
            access_key(*access)
        ),
        other => other.to_string(),
    }
}

fn access_key(access: Access) -> &'static str {
    match access {
        Access::Read => "read",
        Access::Write => "write",
    }
}

fn io_error(ctx: &Ctx<'_>, action: &str, path: &Path, error: &std::io::Error) -> JsError {
    Exception::throw_message(
        ctx,
        &format!("cannot {action} `{}`: {error}", path.display()),
    )
}

// -- Store -----------------------------------------------------------------

/// The per-application settings file: a flat JSON object on disk.
///
/// Values are cached in memory because `get` is called from `render`, where a
/// file read per frame would be absurd. Mutations write through immediately —
/// see [`Store::persist`] for why `flush` is still part of the API.
struct Store {
    path: PathBuf,
    values: Option<serde_json::Map<String, Json>>,
}

impl Store {
    fn new(path: PathBuf) -> Self {
        Self { path, values: None }
    }

    /// Loads on first use. A missing file is an empty store — a first run is
    /// not an error. A malformed file is an error, because silently discarding
    /// a user's settings is worse than refusing to start.
    fn values(&mut self) -> Result<&mut serde_json::Map<String, Json>, String> {
        if self.values.is_none() {
            self.values = Some(match std::fs::read(&self.path) {
                Ok(bytes) => serde_json::from_slice(&bytes).map_err(|error| {
                    format!(
                        "`{}` is not a valid store file: {error}",
                        self.path.display()
                    )
                })?,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                    serde_json::Map::new()
                }
                Err(error) => {
                    return Err(format!("cannot read `{}`: {error}", self.path.display()));
                }
            });
        }
        Ok(self.values.as_mut().expect("just populated"))
    }

    /// Writes to a temporary file and renames it over the target, so a crash
    /// mid-write leaves the previous settings intact rather than a truncated
    /// file.
    ///
    /// Every mutation persists immediately. The store holds small configuration
    /// data, and losing a setting because a script forgot to call `flush` is a
    /// worse failure than one extra rename. `flush` stays in the API as the
    /// durability barrier for M3, where the write becomes a promise the script
    /// can await.
    fn persist(&mut self) -> Result<(), String> {
        let Some(values) = &self.values else {
            return Ok(());
        };
        let body = serde_json::to_vec_pretty(values)
            .map_err(|error| format!("cannot encode the store: {error}"))?;

        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|error| format!("cannot create `{}`: {error}", parent.display()))?;
        }

        let mut temporary = self.path.clone().into_os_string();
        temporary.push(".tmp");
        let temporary = PathBuf::from(temporary);

        std::fs::write(&temporary, body)
            .map_err(|error| format!("cannot write `{}`: {error}", temporary.display()))?;
        std::fs::rename(&temporary, &self.path)
            .map_err(|error| format!("cannot write `{}`: {error}", self.path.display()))
    }
}

fn store_object<'js>(ctx: &Ctx<'js>) -> JsResult<Object<'js>> {
    let store = Object::new(ctx.clone())?;

    store.set(
        "get",
        Func::from(|ctx: Ctx<'_>, key: String| -> JsResult<JsonValue> {
            with_store(&ctx, |store| {
                let values = store.values().map_err(|error| fail(&ctx, &error))?;
                Ok(JsonValue(values.get(&key).cloned().unwrap_or(Json::Null)))
            })
        }),
    )?;

    store.set(
        "set",
        Func::from(
            |ctx: Ctx<'_>, key: String, value: JsonValue| -> JsResult<()> {
                with_store(&ctx, |store| {
                    store
                        .values()
                        .map_err(|error| fail(&ctx, &error))?
                        .insert(key, value.0);
                    store.persist().map_err(|error| fail(&ctx, &error))
                })
            },
        ),
    )?;

    store.set(
        "remove",
        Func::from(|ctx: Ctx<'_>, key: String| -> JsResult<()> {
            with_store(&ctx, |store| {
                store
                    .values()
                    .map_err(|error| fail(&ctx, &error))?
                    .remove(&key);
                store.persist().map_err(|error| fail(&ctx, &error))
            })
        }),
    )?;

    store.set(
        "keys",
        Func::from(|ctx: Ctx<'_>| -> JsResult<Vec<String>> {
            with_store(&ctx, |store| {
                let values = store.values().map_err(|error| fail(&ctx, &error))?;
                Ok(values.keys().cloned().collect())
            })
        }),
    )?;

    store.set(
        "flush",
        Func::from(|ctx: Ctx<'_>| -> JsResult<()> {
            with_store(&ctx, |store| {
                store.persist().map_err(|error| fail(&ctx, &error))
            })
        }),
    )?;

    Ok(store)
}

/// Gates every store entry point on the capability and on the host having said
/// where the file lives.
fn with_store<R>(ctx: &Ctx<'_>, body: impl FnOnce(&mut Store) -> JsResult<R>) -> JsResult<R> {
    if !capabilities().has_store() {
        return Err(Exception::throw_type(
            ctx,
            &CapabilityError::StoreDenied.to_string(),
        ));
    }

    STORE.with_borrow_mut(|store| match store {
        Some(store) => body(store),
        // Not a manifest problem, so it does not name a capability key: the
        // embedder skipped `set_store_path`.
        None => Err(Exception::throw_message(
            ctx,
            "gpui.store has no backing file; the host must call set_store_path \
             before the application runs",
        )),
    })
}

fn fail(ctx: &Ctx<'_>, message: &str) -> JsError {
    Exception::throw_message(ctx, message)
}

/// A JS value carried as JSON.
///
/// `rquickjs` ships serde integration behind a feature this crate does not
/// enable, so both directions are written out by hand below. The wrapper exists
/// for a second reason as well: a JS closure cannot unify the `'js` of a `Ctx`
/// parameter with the `'js` of a `Value` parameter or return, so the conversion
/// has to happen inside `FromJs`/`IntoJs`, where there is only one lifetime.
struct JsonValue(Json);

impl<'js> FromJs<'js> for JsonValue {
    fn from_js(ctx: &Ctx<'js>, value: Value<'js>) -> JsResult<Self> {
        to_json(ctx, &value, 0).map(JsonValue)
    }
}

impl<'js> IntoJs<'js> for JsonValue {
    fn into_js(self, ctx: &Ctx<'js>) -> JsResult<Value<'js>> {
        from_json(ctx, &self.0)
    }
}

/// A cycle in a JS object graph would recurse forever, so depth is capped.
/// Real configuration data is nowhere near this deep.
const MAX_JSON_DEPTH: usize = 64;

/// Arrays and plain objects only, matching what the store can persist.
/// Functions and `undefined` properties are dropped exactly as
/// `JSON.stringify` drops them, so a script's mental model transfers.
fn to_json(ctx: &Ctx<'_>, value: &Value<'_>, depth: usize) -> JsResult<Json> {
    if depth > MAX_JSON_DEPTH {
        return Err(Exception::throw_type(
            ctx,
            "value nests more than 64 levels deep; the store holds plain data and \
             cannot hold a reference cycle",
        ));
    }

    if value.is_null() || value.is_undefined() {
        return Ok(Json::Null);
    }
    if let Some(flag) = value.as_bool() {
        return Ok(Json::Bool(flag));
    }
    if let Some(number) = value.as_int() {
        return Ok(Json::Number(number.into()));
    }
    if let Some(number) = value.as_number() {
        return serde_json::Number::from_f64(number)
            .map(Json::Number)
            .ok_or_else(|| {
                Exception::throw_type(
                    ctx,
                    "NaN and Infinity have no JSON form and cannot be stored",
                )
            });
    }
    if let Some(text) = value.as_string() {
        return Ok(Json::String(text.to_string()?));
    }
    if let Some(array) = value.as_array() {
        let mut items = Vec::with_capacity(array.len());
        for entry in array.iter::<Value>() {
            items.push(to_json(ctx, &entry?, depth + 1)?);
        }
        return Ok(Json::Array(items));
    }
    if value.is_function() {
        return Err(Exception::throw_type(
            ctx,
            "a function cannot be stored; the store holds data, not behaviour",
        ));
    }
    if let Some(object) = value.as_object() {
        let mut map = serde_json::Map::new();
        for property in object.props::<String, Value>() {
            let (key, entry) = property?;
            if entry.is_undefined() || entry.is_function() {
                continue;
            }
            map.insert(key, to_json(ctx, &entry, depth + 1)?);
        }
        return Ok(Json::Object(map));
    }

    Err(Exception::throw_type(
        ctx,
        "unsupported value; the store holds null, booleans, numbers, strings, \
         arrays and plain objects",
    ))
}

fn from_json<'js>(ctx: &Ctx<'js>, value: &Json) -> JsResult<Value<'js>> {
    Ok(match value {
        Json::Null => Value::new_null(ctx.clone()),
        Json::Bool(flag) => Value::new_bool(ctx.clone(), *flag),
        // Every JS number is a double, so the integer/float split on the JSON
        // side does not survive the trip and does not need to.
        Json::Number(number) => match number.as_f64() {
            Some(number) => Value::new_float(ctx.clone(), number),
            None => Value::new_null(ctx.clone()),
        },
        Json::String(text) => rquickjs::String::from_str(ctx.clone(), text)?.into_value(),
        Json::Array(items) => {
            let array = Array::new(ctx.clone())?;
            for (index, item) in items.iter().enumerate() {
                array.set(index, from_json(ctx, item)?)?;
            }
            array.into_value()
        }
        Json::Object(map) => {
            let object = Object::new(ctx.clone())?;
            for (key, item) in map {
                object.set(key.as_str(), from_json(ctx, item)?)?;
            }
            object.into_value()
        }
    })
}

// -- Clipboard -------------------------------------------------------------

fn clipboard_object<'js>(ctx: &Ctx<'js>) -> JsResult<Object<'js>> {
    let clipboard = Object::new(ctx.clone())?;

    clipboard.set(
        "read_text",
        Func::from(|ctx: Ctx<'_>| -> JsResult<Option<String>> {
            if !capabilities().is_clipboard_readable() {
                return Err(Exception::throw_type(&ctx, CLIPBOARD_READ_DENIED));
            }
            with_app(&ctx, "clipboard.read_text()", |app| {
                app.read_from_clipboard().and_then(|item| item.text())
            })
        }),
    )?;

    clipboard.set(
        "write_text",
        Func::from(|ctx: Ctx<'_>, text: String| -> JsResult<()> {
            if !capabilities().is_clipboard_writable() {
                return Err(Exception::throw_type(&ctx, CLIPBOARD_WRITE_DENIED));
            }
            with_app(&ctx, "clipboard.write_text(text)", move |app| {
                app.write_to_clipboard(ClipboardItem::new_string(text))
            })
        }),
    )?;

    Ok(clipboard)
}

/// Read and write are separate grants, so the denial names the half that was
/// missing. Wording follows [`CapabilityError`].
const CLIPBOARD_READ_DENIED: &str =
    "reading the clipboard is not granted; declare capabilities.clipboard.read in the manifest";
const CLIPBOARD_WRITE_DENIED: &str =
    "writing the clipboard is not granted; declare capabilities.clipboard.write in the manifest";

/// Runs `body` against GPUI's `App`.
///
/// The `App` only exists for the duration of a host call (see [`scope`]), so a
/// clipboard call made from, say, a module's top level has nothing to talk to.
/// That reports as an ordinary script error rather than a panic — a misplaced
/// call is an application bug, not a host bug (§5.8).
fn with_app<R>(ctx: &Ctx<'_>, what: &str, body: impl FnOnce(&mut App) -> R) -> JsResult<R> {
    scope::with_current_app(body).ok_or_else(|| {
        Exception::throw_type(
            ctx,
            &format!(
                "{what} needs a live host call; call it from render, an event handler or a task"
            ),
        )
    })
}

// -- Log -------------------------------------------------------------------

fn log_object<'js>(ctx: &Ctx<'js>) -> JsResult<Object<'js>> {
    let log = Object::new(ctx.clone())?;

    // No capability: a script that can run can already say something. Denying
    // logging would only cost the author their diagnostics.
    log.set(
        "debug",
        Func::from(|message: Printable, rest: Rest<Printable>| {
            tracing::debug!(target: SCRIPT_TARGET, "{}", line(message, rest));
        }),
    )?;
    log.set(
        "info",
        Func::from(|message: Printable, rest: Rest<Printable>| {
            tracing::info!(target: SCRIPT_TARGET, "{}", line(message, rest));
        }),
    )?;
    log.set(
        "warn",
        Func::from(|message: Printable, rest: Rest<Printable>| {
            tracing::warn!(target: SCRIPT_TARGET, "{}", line(message, rest));
        }),
    )?;
    log.set(
        "error",
        Func::from(|message: Printable, rest: Rest<Printable>| {
            tracing::error!(target: SCRIPT_TARGET, "{}", line(message, rest));
        }),
    )?;

    Ok(log)
}

/// Script output is separable from host output in a log filter.
const SCRIPT_TARGET: &str = "gpui_shell::script";

/// Extra arguments are appended space-separated after the message, the way
/// `console.log` behaves — JS authors write `log.info("loaded", count)` without
/// thinking about it.
fn line(message: Printable, rest: Rest<Printable>) -> String {
    let mut line = message.0;
    for argument in rest.0 {
        line.push(' ');
        line.push_str(&argument.0);
    }
    line
}

/// Any JS value, rendered for a log line.
///
/// A wrapper rather than `String` because a script will pass numbers, objects
/// and `undefined` to a logger, and refusing those with a type error would make
/// the logger the least usable thing in the API.
struct Printable(String);

impl<'js> FromJs<'js> for Printable {
    fn from_js(ctx: &Ctx<'js>, value: Value<'js>) -> JsResult<Self> {
        if let Some(text) = value.as_string() {
            return Ok(Self(text.to_string()?));
        }
        if value.is_undefined() {
            return Ok(Self("undefined".into()));
        }
        if value.is_function() {
            return Ok(Self("[function]".into()));
        }
        // Structured values print as JSON, which is what an author reading a
        // log wants to see; anything the conversion refuses prints as a
        // placeholder rather than aborting the call it was describing.
        Ok(match to_json(ctx, &value, 0) {
            Ok(json) => Self(json.to_string()),
            Err(_) => {
                ctx.catch();
                Self("[unprintable]".into())
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use rquickjs::{Context as JsContext, Runtime as JsRuntime};

    use super::*;

    /// A directory of our own under the system temporary directory. `tempfile`
    /// is not a dependency of this crate and one test module is not a reason to
    /// add one.
    struct TempDir(PathBuf);

    impl TempDir {
        fn new() -> Self {
            static COUNTER: AtomicUsize = AtomicUsize::new(0);
            let path = std::env::temp_dir().join(format!(
                "gpui-shell-host-{}-{}",
                std::process::id(),
                COUNTER.fetch_add(1, Ordering::Relaxed)
            ));
            std::fs::create_dir_all(&path).expect("creating the test directory");
            Self(path)
        }

        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    /// Builds a real VM with the host surface bound to `gpui`, the way the
    /// engine binds it. Each `#[test]` runs on its own thread, so the
    /// thread-local grant starts empty every time.
    fn with_host<R>(body: impl FnOnce(&Ctx<'_>) -> R) -> R {
        let runtime = JsRuntime::new().expect("starting the JS runtime");
        let context = JsContext::full(&runtime).expect("creating the JS context");
        context.with(|ctx| {
            let module = Object::new(ctx.clone()).expect("creating the module object");
            install(&ctx, &module).expect("installing the host surface");
            ctx.globals().set("gpui", module).expect("binding `gpui`");
            body(&ctx)
        })
    }

    /// The message of a thrown exception, which is the part these tests are
    /// about: a denial has to say what to declare.
    fn error_of(ctx: &Ctx<'_>, source: &str) -> String {
        match ctx.eval::<Value, _>(source) {
            Ok(_) => panic!("`{source}` was expected to throw"),
            Err(JsError::Exception) => {
                let thrown = ctx.catch();
                thrown
                    .as_exception()
                    .and_then(|exception| exception.message())
                    .unwrap_or_else(|| format!("{thrown:?}"))
            }
            Err(error) => panic!("`{source}` failed without an exception: {error}"),
        }
    }

    #[test]
    fn a_read_outside_the_granted_root_names_the_manifest_key() {
        let directory = TempDir::new();
        set_capabilities(Capabilities::new().with_read_roots([directory.path().to_path_buf()]));

        let message = with_host(|ctx| error_of(ctx, "gpui.fs.read_text('../../etc/passwd')"));
        assert!(
            message.contains("outside every granted read root"),
            "unexpected message: {message}"
        );
        assert!(
            message.contains("capabilities.fs.read"),
            "the denial must name the manifest key: {message}"
        );
    }

    #[test]
    fn a_read_without_a_grant_reports_the_missing_capability() {
        let message = with_host(|ctx| error_of(ctx, "gpui.fs.read_text('items.json')"));
        assert!(
            message.contains("is not granted") && message.contains("capabilities.fs.read"),
            "unexpected message: {message}"
        );
    }

    #[test]
    fn a_granted_read_and_write_round_trip() {
        let directory = TempDir::new();
        set_capabilities(
            Capabilities::new()
                .with_read_roots([directory.path().to_path_buf()])
                .with_write_roots([directory.path().to_path_buf()]),
        );

        with_host(|ctx| {
            ctx.eval::<(), _>("gpui.fs.write_text('notes.txt', 'hello')")
                .expect("writing inside the granted root");
            let text: String = ctx
                .eval("gpui.fs.read_text('notes.txt')")
                .expect("reading it back");
            assert_eq!(text, "hello");

            let names: Vec<String> = ctx
                .eval("gpui.fs.read_dir('.').map((entry) => entry.name)")
                .expect("listing the granted root");
            assert_eq!(names, vec!["notes.txt".to_string()]);

            let exists: bool = ctx.eval("gpui.fs.exists('notes.txt')").expect("exists");
            assert!(exists);
        });
    }

    #[test]
    fn the_store_is_denied_without_the_capability() {
        let directory = TempDir::new();
        set_store_path(directory.path().join("store.json"));

        let message = with_host(|ctx| error_of(ctx, "gpui.store.get('theme')"));
        assert!(
            message.contains("capabilities.store"),
            "unexpected message: {message}"
        );
    }

    #[test]
    fn the_store_round_trips_a_nested_object() {
        let directory = TempDir::new();
        let file = directory.path().join("store.json");
        set_capabilities(Capabilities::new().store(true));
        set_store_path(file.clone());

        with_host(|ctx| {
            ctx.eval::<(), _>(
                "gpui.store.set('window', { title: 'Notes', size: [640, 480], \
                 open: true, nested: { depth: 2 } })",
            )
            .expect("storing a nested object");

            let title: String = ctx
                .eval("gpui.store.get('window').title")
                .expect("reading it back");
            assert_eq!(title, "Notes");

            let width: f64 = ctx
                .eval("gpui.store.get('window').size[0]")
                .expect("reading a nested array element");
            assert_eq!(width, 640.0);

            let depth: f64 = ctx
                .eval("gpui.store.get('window').nested.depth")
                .expect("reading a nested object field");
            assert_eq!(depth, 2.0);

            let keys: Vec<String> = ctx.eval("gpui.store.keys()").expect("listing the keys");
            assert_eq!(keys, vec!["window".to_string()]);
        });

        // It reached disk, atomically, without anyone calling flush.
        let written = std::fs::read_to_string(&file).expect("the store file exists");
        assert!(written.contains("\"title\": \"Notes\""), "{written}");
        assert!(!directory.path().join("store.json.tmp").exists());
    }

    #[test]
    fn the_store_forgets_a_removed_key() {
        let directory = TempDir::new();
        set_capabilities(Capabilities::new().store(true));
        set_store_path(directory.path().join("store.json"));

        with_host(|ctx| {
            ctx.eval::<(), _>("gpui.store.set('theme', 'dark'); gpui.store.remove('theme')")
                .expect("setting then removing");
            let keys: Vec<String> = ctx.eval("gpui.store.keys()").expect("listing the keys");
            assert!(keys.is_empty(), "unexpected keys: {keys:?}");
        });
    }

    #[test]
    fn the_clipboard_fails_cleanly_outside_a_host_call() {
        set_capabilities(
            Capabilities::new()
                .clipboard_read(true)
                .clipboard_write(true),
        );

        let message = with_host(|ctx| error_of(ctx, "gpui.clipboard.read_text()"));
        assert!(
            message.contains("needs a live host call"),
            "unexpected message: {message}"
        );
    }

    #[test]
    fn the_clipboard_denial_names_the_half_that_is_missing() {
        let message = with_host(|ctx| error_of(ctx, "gpui.clipboard.write_text('x')"));
        assert!(
            message.contains("capabilities.clipboard.write"),
            "unexpected message: {message}"
        );
    }

    #[test]
    fn logging_needs_no_capability_and_takes_extra_arguments() {
        with_host(|ctx| {
            ctx.eval::<(), _>("gpui.log.info('loaded', 3, { ok: true })")
                .expect("logging is always available");
        });
    }
}
