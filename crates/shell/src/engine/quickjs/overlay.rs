//! Dialogs, the sheet and toasts on the script-side `cx`.
//!
//! An overlay is a *host* capability, not something a script draws. A dialog is
//! not a floating `div`: it is a place in the window's stacking order, a focus
//! trap, an Escape target, and a promise about what a backdrop press means —
//! all of which [`ShellRoot`] decides for the window as a whole, because only
//! something that sees every overlay at once can order them. A script that drew
//! its own dialog would own none of that, and two scripts drawing two dialogs
//! would own even less. So the script says *what* to put in front of the user,
//! and the root says where it goes and how it leaves.
//!
//! What crosses the boundary is therefore small: a view class to instantiate, a
//! side to anchor to, a sentence to show. Everything else — layering,
//! dismissal, focus restoration, toast lifecycle — stays in [`crate::root`].
//!
//! # Why every entry point checks the phase first
//!
//! Opening or closing an overlay mutates the window, and the render pass is
//! reading it. GPUI's borrow model has no way to express "the script may notify
//! from here but not from there", so [`crate::scope`] carries the phase and each
//! entry point refuses `Render` and `Layout` (design doc §16.2). The check lives
//! here as well as in [`ShellRoot`] because the two refusals are different
//! things: the root logs and ignores, which is the right answer for host code
//! that got it wrong, while a script gets a thrown `TypeError` naming the phase
//! it called from — the same shape the style layer uses for an unknown method,
//! and the only shape an author can act on.
//!
//! # The script surface
//!
//! ```js
//! const depth = cx.open_dialog(ConfirmDialog, {
//!   escape_dismissable: false,
//!   backdrop_dismissable: false,
//!   props: { path },
//! });
//! cx.close_dialog();       // -> did anything close?
//! cx.close_all_dialogs();  // -> how many closed
//!
//! cx.open_sheet("right", FiltersPanel, { props: { filters } });
//! cx.close_sheet();        // -> did anything close?
//!
//! cx.toast({ title: "Saved", description: "3 files", level: "success",
//!            timeout: 4000, id: "save" });
//! cx.dismiss_toast("save");
//! cx.dismiss_all_toasts();
//! ```

use std::time::Duration;

use gpui::{AnyView, App, AppContext as _, Context, Window};
use rquickjs::{
    Constructor, Ctx, Exception, FromJs, Object, Persistent, Result as JsResult, Value,
    function::{Func, Opt},
};

use crate::{
    root::{DialogOptions, SheetSide, ShellRoot, ToastLevel, ToastRequest},
    scope::{self, ScopePhase},
    view::ScriptView,
};

use super::{ShellRuntime, ViewObject};

/// The names an error message uses, so a refusal reads like the call that
/// caused it rather than like the Rust function that answered it.
const OPEN_DIALOG: &str = "cx.open_dialog(view, options)";
const CLOSE_DIALOG: &str = "cx.close_dialog()";
const CLOSE_ALL_DIALOGS: &str = "cx.close_all_dialogs()";
const OPEN_SHEET: &str = "cx.open_sheet(side, view, options)";
const CLOSE_SHEET: &str = "cx.close_sheet()";
const TOAST: &str = "cx.toast(options)";
const DISMISS_TOAST: &str = "cx.dismiss_toast(id)";
const DISMISS_ALL_TOASTS: &str = "cx.dismiss_all_toasts()";

/// Adds the overlay methods to one script-side `cx`.
///
/// Called from `context_object`, once per host call, with that call's
/// generation. The generation is the whole of the safety story: a `cx` stashed
/// in a closure and used after its call returned reaches
/// [`scope::with_context`] with a stale generation and reports that, rather
/// than opening a dialog against a dead stack frame.
///
/// `ctx` is unused — every value installed here is built by `Object::set` from
/// the target object's own context, because `Ctx` and `Object` are invariant in
/// their lifetime and a value built from one cannot be set on the other. It
/// stays in the signature to match the other installers.
pub fn install(_ctx: &Ctx<'_>, context_object: &Object<'_>, generation: u64) -> JsResult<()> {
    // Returns the new depth of the dialog stack rather than a handle: the root
    // addresses dialogs by position, never by identity, so a handle would have
    // to promise "close *this* dialog", which is not an operation that exists.
    // The depth is what a script can actually use — to assert it opened one, or
    // to unwind to a known level.
    context_object.set(
        "open_dialog",
        Func::from(
            move |ctx: Ctx<'_>, class: ViewClass, options: Opt<DialogRequest>| -> JsResult<u32> {
                guard(&ctx, OPEN_DIALOG)?;
                let request = options.0.unwrap_or_default();
                let object = class.instantiate(&ctx, request.props.as_ref())?;

                with_root(&ctx, generation, OPEN_DIALOG, |root, window, cx| {
                    let view = mount(&ctx, object, cx)?;
                    root.open_dialog_with(view, request.options, window, cx);
                    Ok(root.dialog_count() as u32)
                })
            },
        ),
    )?;

    context_object.set(
        "close_dialog",
        Func::from(move |ctx: Ctx<'_>| -> JsResult<bool> {
            guard(&ctx, CLOSE_DIALOG)?;
            with_root(&ctx, generation, CLOSE_DIALOG, |root, window, cx| {
                Ok(root.close_dialog(window, cx))
            })
        }),
    )?;

    context_object.set(
        "close_all_dialogs",
        Func::from(move |ctx: Ctx<'_>| -> JsResult<u32> {
            guard(&ctx, CLOSE_ALL_DIALOGS)?;
            with_root(&ctx, generation, CLOSE_ALL_DIALOGS, |root, window, cx| {
                // Read before clearing: the root reports nothing, and "how many
                // did I close?" is the same question `close_dialog`'s `bool`
                // answers for one.
                let closed = root.dialog_count() as u32;
                root.close_all_dialogs(window, cx);
                Ok(closed)
            })
        }),
    )?;

    context_object.set(
        "open_sheet",
        Func::from(
            move |ctx: Ctx<'_>,
                  side: String,
                  class: ViewClass,
                  options: Opt<SheetRequest>|
                  -> JsResult<()> {
                guard(&ctx, OPEN_SHEET)?;
                let side = parse_side(&ctx, &side)?;
                let request = options.0.unwrap_or_default();
                let object = class.instantiate(&ctx, request.props.as_ref())?;

                with_root(&ctx, generation, OPEN_SHEET, |root, window, cx| {
                    let view = mount(&ctx, object, cx)?;
                    root.open_sheet(side, view, window, cx);
                    Ok(())
                })
            },
        ),
    )?;

    context_object.set(
        "close_sheet",
        Func::from(move |ctx: Ctx<'_>| -> JsResult<bool> {
            guard(&ctx, CLOSE_SHEET)?;
            with_root(&ctx, generation, CLOSE_SHEET, |root, window, cx| {
                Ok(root.close_sheet(window, cx))
            })
        }),
    )?;

    // A toast is data, not a view: no class, no instance, nothing for the
    // script to render. That is why it is the one overlay whose whole content
    // crosses the boundary as an options object.
    context_object.set(
        "toast",
        Func::from(move |ctx: Ctx<'_>, toast: ToastArgument| -> JsResult<()> {
            guard(&ctx, TOAST)?;
            with_root(&ctx, generation, TOAST, |root, window, cx| {
                root.push_toast(toast.0, window, cx);
                Ok(())
            })
        }),
    )?;

    context_object.set(
        "dismiss_toast",
        Func::from(move |ctx: Ctx<'_>, id: String| -> JsResult<bool> {
            guard(&ctx, DISMISS_TOAST)?;
            with_root(&ctx, generation, DISMISS_TOAST, |root, _, cx| {
                Ok(root.dismiss_toast(id, cx))
            })
        }),
    )?;

    context_object.set(
        "dismiss_all_toasts",
        Func::from(move |ctx: Ctx<'_>| -> JsResult<()> {
            guard(&ctx, DISMISS_ALL_TOASTS)?;
            with_root(&ctx, generation, DISMISS_ALL_TOASTS, |root, _, cx| {
                root.dismiss_all_toasts(cx);
                Ok(())
            })
        }),
    )
}

/// Refuses an overlay change that is not being made from an event or a task.
///
/// Outside any scope there is no window to reach either, so `none` is refused
/// with the same message: both cases mean the call has no live host frame.
fn guard(ctx: &Ctx<'_>, api: &str) -> JsResult<()> {
    let phase = scope::current_phase();
    if phase.is_some_and(ScopePhase::allows_notify) {
        return Ok(());
    }

    Err(Exception::throw_type(
        ctx,
        &format!(
            "{api} is not allowed during the `{}` phase; overlays may only be opened or \
             closed while handling an event or a task",
            phase.map(ScopePhase::as_str).unwrap_or("none")
        ),
    ))
}

/// Runs `body` against the overlay host of the window the call belongs to.
///
/// Two ways this fails, and they are different mistakes. A stale generation is
/// a script error — a `cx` used after its call returned — and
/// [`scope::StaleContext`] already explains it. A window whose root view is not
/// a [`ShellRoot`] is a *host* wiring error, so it says so rather than
/// pretending the overlay was opened.
fn with_root<R>(
    ctx: &Ctx<'_>,
    generation: u64,
    api: &str,
    body: impl FnOnce(&mut ShellRoot, &mut Window, &mut Context<ShellRoot>) -> JsResult<R>,
) -> JsResult<R> {
    let reached = scope::with_context(generation, |window, app| {
        ShellRoot::update(window, app, body)
    })
    .map_err(|error| Exception::throw_type(ctx, &error.to_string()))?;

    reached.unwrap_or_else(|| {
        Err(Exception::throw_type(
            ctx,
            &format!(
                "{api} needs a ShellRoot as the window's first view; this window was \
                 opened with another view"
            ),
        ))
    })
}

/// Wraps a freshly constructed script instance as a view the root can mount.
fn mount(ctx: &Ctx<'_>, object: ViewObject, cx: &mut App) -> JsResult<AnyView> {
    let Some(runtime) = ShellRuntime::global(cx) else {
        return Err(Exception::throw_type(
            ctx,
            "the shell runtime is not installed on this application",
        ));
    };
    Ok(cx.new(|_| ScriptView::new(runtime, object)).into())
}

/// A view class, kept alive across the argument conversion.
///
/// Its own type because a JS closure cannot unify the lifetime of a `Ctx<'js>`
/// parameter with that of a `Value<'js>` one — the two elided lifetimes are
/// independent as far as inference is concerned. Converting inside [`FromJs`],
/// where both are the same lifetime again, is the pattern `Arguments` in the
/// parent module exists for.
struct ViewClass(Persistent<Constructor<'static>>);

impl ViewClass {
    /// Constructs one instance, passing `props` to the class.
    ///
    /// Constructs directly rather than through the prelude's `__construct`,
    /// which takes no arguments: the base `View` constructor forwards whatever
    /// it is given to `init(props)`, so `new Class(props)` is the whole
    /// protocol. Any promise the constructor starts is drained by the entry
    /// point this call is nested inside, so there is nothing to pump here.
    fn instantiate(
        &self,
        ctx: &Ctx<'_>,
        props: Option<&Persistent<Value<'static>>>,
    ) -> JsResult<ViewObject> {
        let class = self.0.clone().restore(ctx)?;
        let instance: Object = match props {
            Some(props) => class.construct((props.clone().restore(ctx)?,))?,
            None => class.construct(())?,
        };
        Ok(Persistent::save(ctx, instance))
    }
}

impl<'js> FromJs<'js> for ViewClass {
    fn from_js(ctx: &Ctx<'js>, value: Value<'js>) -> JsResult<Self> {
        let Some(class) = value.into_constructor() else {
            return Err(Exception::throw_type(
                ctx,
                "expected a view class; open_dialog and open_sheet take the class itself, \
                 not an instance and not an element",
            ));
        };
        Ok(Self(Persistent::save(ctx, class)))
    }
}

/// `{ escape_dismissable, backdrop_dismissable, props }`.
#[derive(Default)]
struct DialogRequest {
    options: DialogOptions,
    props: Option<Persistent<Value<'static>>>,
}

const DIALOG_KEYS: &[&str] = &["escape_dismissable", "backdrop_dismissable", "props"];

impl<'js> FromJs<'js> for DialogRequest {
    fn from_js(ctx: &Ctx<'js>, value: Value<'js>) -> JsResult<Self> {
        let Some(object) = options_object(ctx, &value, OPEN_DIALOG)? else {
            return Ok(Self::default());
        };
        reject_unknown_keys(ctx, object, DIALOG_KEYS, OPEN_DIALOG)?;

        let mut options = DialogOptions::default();
        if let Some(dismissable) = object.get::<_, Option<bool>>("escape_dismissable")? {
            options = options.escape_dismissable(dismissable);
        }
        if let Some(dismissable) = object.get::<_, Option<bool>>("backdrop_dismissable")? {
            options = options.backdrop_dismissable(dismissable);
        }

        Ok(Self {
            options,
            props: props_of(object)?,
        })
    }
}

/// `{ props }`. A sheet has no dismissal options: there is only ever one, and
/// it is dismissed by Escape or by its overlay whenever no dialog is above it.
#[derive(Default)]
struct SheetRequest {
    props: Option<Persistent<Value<'static>>>,
}

const SHEET_KEYS: &[&str] = &["props"];

impl<'js> FromJs<'js> for SheetRequest {
    fn from_js(ctx: &Ctx<'js>, value: Value<'js>) -> JsResult<Self> {
        let Some(object) = options_object(ctx, &value, OPEN_SHEET)? else {
            return Ok(Self::default());
        };
        reject_unknown_keys(ctx, object, SHEET_KEYS, OPEN_SHEET)?;

        Ok(Self {
            props: props_of(object)?,
        })
    }
}

/// `{ title, description, level, timeout, id }`.
struct ToastArgument(ToastRequest);

const TOAST_KEYS: &[&str] = &["title", "description", "level", "timeout", "id"];

impl<'js> FromJs<'js> for ToastArgument {
    fn from_js(ctx: &Ctx<'js>, value: Value<'js>) -> JsResult<Self> {
        let Some(object) = value.as_object() else {
            return Err(Exception::throw_type(
                ctx,
                &format!("{TOAST} expects an object, such as {{ title: \"Saved\" }}"),
            ));
        };
        reject_unknown_keys(ctx, object, TOAST_KEYS, TOAST)?;

        let Some(title) = object.get::<_, Option<String>>("title")? else {
            return Err(Exception::throw_type(
                ctx,
                &format!("{TOAST} requires a `title`; it is the sentence the user reads"),
            ));
        };

        let mut toast = ToastRequest::new(title);
        if let Some(description) = object.get::<_, Option<String>>("description")? {
            toast = toast.with_description(description);
        }
        if let Some(level) = object.get::<_, Option<String>>("level")? {
            toast = toast.with_level(parse_level(ctx, &level)?);
        }
        if let Some(id) = object.get::<_, Option<String>>("id")? {
            toast = toast.with_id(id);
        }

        // An absent `timeout` keeps the default; an explicit `null` is the way
        // to ask for a toast that stays until it is dismissed, so the two
        // cannot be collapsed into one `Option`.
        let timeout: Value = object.get("timeout")?;
        if !timeout.is_undefined() {
            toast = toast.with_timeout(parse_timeout(ctx, &timeout)?);
        }

        Ok(Self(toast))
    }
}

/// The object form of an optional trailing options argument.
///
/// `None` means "not given": an omitted argument, or an explicit `null` or
/// `undefined`, all of which mean the defaults. Anything that is not an object
/// is a mistake worth naming.
fn options_object<'a, 'js>(
    ctx: &Ctx<'js>,
    value: &'a Value<'js>,
    api: &str,
) -> JsResult<Option<&'a Object<'js>>> {
    if value.is_undefined() || value.is_null() {
        return Ok(None);
    }
    match value.as_object() {
        Some(object) => Ok(Some(object)),
        None => Err(Exception::throw_type(
            ctx,
            &format!("{api} expects an options object"),
        )),
    }
}

/// Rejects a key the host does not read.
///
/// A silently ignored `escapeDismissable` is exactly the failure the style
/// layer's unknown-method diagnostic exists to prevent: the call looks like it
/// worked, and the dialog is dismissable anyway.
fn reject_unknown_keys(
    ctx: &Ctx<'_>,
    object: &Object<'_>,
    known: &[&str],
    api: &str,
) -> JsResult<()> {
    for key in object.keys::<String>() {
        let key = key?;
        if !known.contains(&key.as_str()) {
            return Err(Exception::throw_type(
                ctx,
                &format!(
                    "unknown option `{key}` for {api}; expected {}",
                    listed(known)
                ),
            ));
        }
    }
    Ok(())
}

fn props_of(object: &Object<'_>) -> JsResult<Option<Persistent<Value<'static>>>> {
    let props: Value = object.get("props")?;
    let given = !props.is_undefined() && !props.is_null();
    Ok(given.then(|| Persistent::save(object.ctx(), props)))
}

/// Every side a script may name. Also what an unknown one is told to use, so
/// the message cannot drift from the set.
const SHEET_SIDES: [SheetSide; 4] = [
    SheetSide::Left,
    SheetSide::Right,
    SheetSide::Top,
    SheetSide::Bottom,
];

const TOAST_LEVELS: [ToastLevel; 4] = [
    ToastLevel::Info,
    ToastLevel::Success,
    ToastLevel::Warning,
    ToastLevel::Error,
];

fn parse_side(ctx: &Ctx<'_>, name: &str) -> JsResult<SheetSide> {
    SheetSide::from_name(name).ok_or_else(|| {
        Exception::throw_type(
            ctx,
            &format!(
                "unknown sheet side `{name}`; expected {}",
                listed_by(&SHEET_SIDES, SheetSide::as_str)
            ),
        )
    })
}

fn parse_level(ctx: &Ctx<'_>, name: &str) -> JsResult<ToastLevel> {
    ToastLevel::from_name(name).ok_or_else(|| {
        Exception::throw_type(
            ctx,
            &format!(
                "unknown toast level `{name}`; expected {}",
                listed_by(&TOAST_LEVELS, ToastLevel::as_str)
            ),
        )
    })
}

/// `null` is sticky; a number is milliseconds.
fn parse_timeout(ctx: &Ctx<'_>, value: &Value<'_>) -> JsResult<Option<Duration>> {
    if value.is_null() {
        return Ok(None);
    }

    let refused = || {
        Exception::throw_type(
            ctx,
            &format!(
                "{TOAST} expects `timeout` to be a number of milliseconds, or null to keep \
                 the toast until it is dismissed"
            ),
        )
    };

    let ms = value.as_number().ok_or_else(refused)?;
    if !ms.is_finite() || ms < 0. {
        return Err(refused());
    }
    Ok(Some(Duration::from_millis(ms as u64)))
}

fn listed(names: &[&str]) -> String {
    match names.split_last() {
        Some((last, rest)) if !rest.is_empty() => format!("{} or {last}", rest.join(", ")),
        _ => names.concat(),
    }
}

fn listed_by<T: Copy>(values: &[T], name: fn(T) -> &'static str) -> String {
    listed(&values.iter().map(|value| name(*value)).collect::<Vec<_>>())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::quickjs::context_object;
    use gpui::{Entity, IntoElement, Render, TestAppContext, VisualTestContext, div};
    use std::rc::Rc;

    struct Empty;

    impl Render for Empty {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div()
        }
    }

    fn shell(
        cx: &mut TestAppContext,
    ) -> (Rc<ShellRuntime>, Entity<ShellRoot>, &mut VisualTestContext) {
        cx.update(crate::init);

        let runtime = ShellRuntime::new().expect("runtime");
        cx.update(|cx| runtime.set_global(cx));
        // The views these tests open hold `Persistent` script values, and a
        // `Persistent` released after its runtime has gone aborts the process.
        // Teardown order is not ours to choose, so the runtime outlives the
        // test deliberately — the same trade the scheduler's tests make.
        std::mem::forget(runtime.clone());

        let (root, cx) = cx.add_window_view(|window, cx| {
            let content = cx.new(|_| Empty).into();
            ShellRoot::new(content, window, cx)
        });
        (runtime, root, cx)
    }

    /// Evaluates `source` with a script-side `cx` belonging to a fresh scope,
    /// which is what an overlay call needs and what the phase check reads.
    fn eval<T>(
        runtime: &Rc<ShellRuntime>,
        cx: &mut VisualTestContext,
        phase: ScopePhase,
        source: &str,
    ) -> anyhow::Result<T>
    where
        T: for<'js> FromJs<'js> + 'static,
    {
        cx.update(|window, app| {
            let (_guard, generation) = scope::enter(window, app, phase, None);
            runtime.with_js(|ctx| {
                ctx.globals().set("cx", context_object(ctx, generation)?)?;
                ctx.eval::<T, _>(source)
            })
        })
    }

    #[gpui::test]
    fn a_script_opens_a_dialog_on_the_root(cx: &mut TestAppContext) {
        let (runtime, root, cx) = shell(cx);

        let depth: u32 = eval(
            &runtime,
            cx,
            ScopePhase::Event,
            "cx.open_dialog(class Confirm {})",
        )
        .expect("open_dialog");

        assert_eq!(depth, 1);
        assert_eq!(root.read_with(cx, |root, _| root.dialog_count()), 1);

        // A second dialog stacks rather than replacing, and closing reports
        // that it found something to close.
        let depth: u32 = eval(
            &runtime,
            cx,
            ScopePhase::Event,
            "cx.open_dialog(class Detail {}, { escape_dismissable: false })",
        )
        .expect("open_dialog");
        assert_eq!(depth, 2);

        let closed: bool =
            eval(&runtime, cx, ScopePhase::Event, "cx.close_dialog()").expect("close_dialog");
        assert!(closed);
        assert_eq!(root.read_with(cx, |root, _| root.dialog_count()), 1);
    }

    #[gpui::test]
    fn closing_a_dialog_reports_when_none_was_open(cx: &mut TestAppContext) {
        let (runtime, root, cx) = shell(cx);

        let closed: bool =
            eval(&runtime, cx, ScopePhase::Event, "cx.close_dialog()").expect("close_dialog");

        assert!(!closed);
        assert_eq!(root.read_with(cx, |root, _| root.dialog_count()), 0);
    }

    /// The second argument is the class's own, so a dialog can be opened with
    /// the row it is about.
    #[gpui::test]
    fn props_reach_the_view_class(cx: &mut TestAppContext) {
        let (runtime, _root, cx) = shell(cx);

        let name: String = eval(
            &runtime,
            cx,
            ScopePhase::Event,
            r#"
            cx.open_dialog(
              class Rename { constructor(props) { globalThis.__seen = props.name; } },
              { props: { name: "notes.md" } },
            );
            globalThis.__seen
            "#,
        )
        .expect("open_dialog");

        assert_eq!(name, "notes.md");
    }

    #[gpui::test]
    fn an_unknown_sheet_side_names_the_valid_ones(cx: &mut TestAppContext) {
        let (runtime, root, cx) = shell(cx);

        let error = eval::<()>(
            &runtime,
            cx,
            ScopePhase::Event,
            r#"cx.open_sheet("middle", class Filters {})"#,
        )
        .expect_err("an unknown side must be refused");

        let message = error.to_string();
        assert!(message.contains("middle"), "{message}");
        assert!(message.contains("left, right, top or bottom"), "{message}");
        assert!(root.read_with(cx, |root, _| root.sheet().is_none()));
    }

    #[gpui::test]
    fn an_unknown_toast_level_names_the_valid_ones(cx: &mut TestAppContext) {
        let (runtime, root, cx) = shell(cx);

        let error = eval::<()>(
            &runtime,
            cx,
            ScopePhase::Event,
            r#"cx.toast({ title: "Gone", level: "fatal" })"#,
        )
        .expect_err("an unknown level must be refused");

        let message = error.to_string();
        assert!(message.contains("fatal"), "{message}");
        assert!(
            message.contains("info, success, warning or error"),
            "{message}"
        );
        assert_eq!(root.read_with(cx, |root, _| root.toast_count()), 0);
    }

    #[gpui::test]
    fn a_toast_reaches_the_stack_and_can_be_dismissed_by_id(cx: &mut TestAppContext) {
        let (runtime, root, cx) = shell(cx);

        eval::<()>(
            &runtime,
            cx,
            ScopePhase::Event,
            r#"cx.toast({ title: "Saved", description: "3 files", level: "success",
                          timeout: 4000, id: "save" })"#,
        )
        .expect("toast");
        assert_eq!(root.read_with(cx, |root, _| root.toast_count()), 1);

        let dismissed: bool = eval(
            &runtime,
            cx,
            ScopePhase::Event,
            r#"cx.dismiss_toast("save")"#,
        )
        .expect("dismiss_toast");
        assert!(dismissed);
    }

    /// The render pass is reading the window an overlay would mutate, so the
    /// call is refused rather than deferred — and the message says which phase
    /// it came from, because that is the only clue the author has.
    #[gpui::test]
    fn overlays_are_refused_during_a_render(cx: &mut TestAppContext) {
        let (runtime, root, cx) = shell(cx);

        let error = eval::<u32>(
            &runtime,
            cx,
            ScopePhase::Render,
            "cx.open_dialog(class Confirm {})",
        )
        .expect_err("a render pass must not open a dialog");

        let message = error.to_string();
        assert!(message.contains("`render` phase"), "{message}");
        assert!(message.contains("event"), "{message}");
        assert_eq!(root.read_with(cx, |root, _| root.dialog_count()), 0);
    }

    /// A mistyped option is a silent no-op unless the host says otherwise.
    #[gpui::test]
    fn a_misspelled_option_is_refused(cx: &mut TestAppContext) {
        let (runtime, _root, cx) = shell(cx);

        let error = eval::<u32>(
            &runtime,
            cx,
            ScopePhase::Event,
            "cx.open_dialog(class Confirm {}, { escapeDismissable: false })",
        )
        .expect_err("an unknown option must be refused");

        let message = error.to_string();
        assert!(message.contains("escapeDismissable"), "{message}");
        assert!(message.contains("escape_dismissable"), "{message}");
    }
}
