//! The render-frequency invariant, in tests.
//!
//! GPUI repaints for reasons the script never hears about, and the whole point
//! of a snapshot is that none of them enter the VM. That claim is only worth
//! anything if it is checked, so these tests count script renders directly:
//!
//! ```text
//! script activity  ──▶ script render
//! GPUI activity    ──▶ (nothing)
//! ```
//!
//! They run against whichever engine is enabled, because the invariant belongs
//! to the runtime rather than to QuickJS.

use std::ops::Deref;

use gpui::{AppContext as _, Entity, IntoElement as _, TestAppContext, VisualTestContext};
use gpui_shell::{
    RenderSnapshot, ScriptView, ShellRuntime,
    spec::{CallbackId, SpecOp},
};

const TOGGLE: &str = r#"
import { View, v_flex, text, Checkbox } from "gpui";

export default class Toggle extends View {
  init() {
    this.count = 0;
  }

  render() {
    return v_flex()
      .child(text(`count: ${this.count}`))
      .child(
        Checkbox.new("toggle").on_change((checked, cx) => {
          this.count += 1;
          cx.notify();
        }),
      );
  }
}
"#;

const ENTRY: &str = "toggle.js";

/// A script whose `render` throws every other call, so a failed build can be
/// observed next to a successful one.
const FLAKY: &str = r#"
import { View, v_flex, text } from "gpui";

export default class Flaky extends View {
  init() {
    this.fail = false;
  }

  render() {
    if (this.fail) {
      throw new Error("render failed on purpose");
    }
    return v_flex().child(text("good"));
  }
}
"#;

#[gpui::test]
fn repeated_gpui_renders_do_not_re_enter_the_script(cx: &mut TestAppContext) {
    let (runtime, mut context, view) = script_view(cx, TOGGLE);

    render_once(&mut context, &view);
    assert_eq!(
        runtime.metrics().read().script_renders(),
        1,
        "the first render must build"
    );

    for _ in 0..64 {
        render_once(&mut context, &view);
    }

    assert_eq!(
        runtime.metrics().read().script_renders(),
        1,
        "a clean view was materialized 65 times and must have entered the VM once"
    );
}

#[gpui::test]
fn a_script_notify_causes_exactly_one_rebuild(cx: &mut TestAppContext) {
    let (runtime, mut context, view) = script_view(cx, TOGGLE);

    render_once(&mut context, &view);
    assert_eq!(runtime.metrics().read().script_renders(), 1);

    render_once(&mut context, &view);
    assert_eq!(
        runtime.metrics().read().script_renders(),
        1,
        "a clean frame must not rebuild"
    );

    let callback = click_target(&mut context, &view);
    context.update(|window, cx| runtime.dispatch_change(callback, true, window, cx));

    render_once(&mut context, &view);
    assert_eq!(
        runtime.metrics().read().script_renders(),
        2,
        "the notify from the handler must rebuild the snapshot once"
    );

    render_once(&mut context, &view);
    assert_eq!(
        runtime.metrics().read().script_renders(),
        2,
        "and the frame after that must be clean again"
    );
}

#[gpui::test]
fn notifying_three_times_rebuilds_once(cx: &mut TestAppContext) {
    let (runtime, mut context, view) = script_view(cx, TOGGLE);

    render_once(&mut context, &view);
    let callback = click_target(&mut context, &view);

    // Three separate events before the next frame. GPUI already coalesces the
    // repaint; the runtime must not add a second scheduler that turns each one
    // into its own script render.
    for _ in 0..3 {
        context.update(|window, cx| runtime.dispatch_change(callback, true, window, cx));
    }

    render_once(&mut context, &view);

    assert_eq!(
        runtime.metrics().read().script_renders(),
        2,
        "three notifies before one frame must rebuild one snapshot"
    );
    assert!(
        snapshot_text(&mut context, &view).contains("count: 3"),
        "all three events must still have reached the script"
    );
}

#[gpui::test]
fn a_bare_notify_repaints_without_running_the_script(cx: &mut TestAppContext) {
    let (runtime, mut context, view) = script_view(cx, TOGGLE);

    render_once(&mut context, &view);
    assert_eq!(runtime.metrics().read().script_renders(), 1);

    // What a host does when something changed that the script cannot see — a
    // hover, an animation, a parent laying out again.
    view.update(&mut context, |_, cx| cx.notify());
    render_once(&mut context, &view);
    assert_eq!(
        runtime.metrics().read().script_renders(),
        1,
        "a bare notify must not re-run the script"
    );

    // What a host does when it changed state the script reads.
    view.update(&mut context, |view, cx| view.refresh(cx));
    render_once(&mut context, &view);
    assert_eq!(
        runtime.metrics().read().script_renders(),
        2,
        "refresh must re-run the script"
    );
}

#[gpui::test]
fn a_handler_survives_the_frames_that_follow_its_render(cx: &mut TestAppContext) {
    let (runtime, mut context, view) = script_view(cx, TOGGLE);

    render_once(&mut context, &view);
    let callback = click_target(&mut context, &view);

    // The frame the handler was registered in is long gone. Its snapshot is
    // not, and that is what has to keep the handler callable.
    for _ in 0..32 {
        render_once(&mut context, &view);
    }

    context.update(|window, cx| runtime.dispatch_change(callback, true, window, cx));
    render_once(&mut context, &view);

    assert!(
        snapshot_text(&mut context, &view).contains("count: 1"),
        "the handler from the live snapshot was dropped by later frames"
    );
}

#[gpui::test]
fn a_failed_render_still_draws_the_interface_under_it(cx: &mut TestAppContext) {
    let (runtime, mut context, view) = script_view(cx, FLAKY);

    render_once(&mut context, &view);
    set_flag(&mut context, &view, &runtime);
    view.update(&mut context, |view, _| view.invalidate());

    // The banner composes over a materialized snapshot rather than replacing
    // it, and both have to survive a real layout and paint pass together.
    render_once(&mut context, &view);
    render_once(&mut context, &view);

    assert!(
        snapshot_text(&mut context, &view).contains("good"),
        "the kept snapshot must still be the one being drawn"
    );
}

#[gpui::test]
fn a_failed_render_keeps_the_previous_snapshot(cx: &mut TestAppContext) {
    let (runtime, mut context, view) = script_view(cx, FLAKY);

    render_once(&mut context, &view);
    assert!(snapshot_text(&mut context, &view).contains("good"));

    set_flag(&mut context, &view, &runtime);
    view.update(&mut context, |view, _| view.invalidate());
    render_once(&mut context, &view);

    assert!(
        snapshot_text(&mut context, &view).contains("good"),
        "a script that threw must not take the last valid description with it"
    );
}

#[gpui::test]
fn a_failed_render_is_not_retried_every_frame(cx: &mut TestAppContext) {
    let (runtime, mut context, view) = script_view(cx, FLAKY);

    render_once(&mut context, &view);
    set_flag(&mut context, &view, &runtime);
    view.update(&mut context, |view, _| view.invalidate());
    render_once(&mut context, &view);

    let after_failure = runtime.metrics().read().script_renders();
    for _ in 0..16 {
        render_once(&mut context, &view);
    }

    assert_eq!(
        runtime.metrics().read().script_renders(),
        after_failure,
        "a broken render is as frame-coupled as a working one if failure re-triggers the build"
    );
}

#[gpui::test]
fn one_view_rendering_does_not_invalidate_another(cx: &mut TestAppContext) {
    let (runtime, mut context, first) = script_view(cx, TOGGLE);
    let second = another_view(&mut context, &runtime, TOGGLE);

    render_once(&mut context, &first);
    render_once(&mut context, &second);
    let callback = click_target(&mut context, &first);

    // Rebuilding the second view must not retire the first view's handlers:
    // both share one runtime, and a global render generation would have made
    // the second view's render invalidate the first view's buttons.
    second.update(&mut context, |view, _| view.invalidate());
    render_once(&mut context, &second);

    context.update(|window, cx| runtime.dispatch_change(callback, true, window, cx));
    render_once(&mut context, &first);

    assert!(
        snapshot_text(&mut context, &first).contains("count: 1"),
        "the other view's render retired this view's handler"
    );
}

#[gpui::test]
fn a_palette_change_rebuilds_the_snapshot(cx: &mut TestAppContext) {
    let (runtime, mut context, view) = script_view(cx, TOGGLE);

    render_once(&mut context, &view);
    assert_eq!(runtime.metrics().read().script_renders(), 1);

    // Tokens resolve to concrete colors while the script builds, so they are
    // baked into the snapshot. Repainting cannot pick up a new palette; only a
    // rebuild can.
    context.update(|_, cx| {
        gpui_shell::theme::set_mode(gpui_shell::theme::ThemeMode::Dark, cx);
    });
    render_once(&mut context, &view);

    assert_eq!(
        runtime.metrics().read().script_renders(),
        2,
        "a palette change must reach script views"
    );
}

/// One GPUI frame containing this view.
///
/// A real layout and paint pass rather than a direct call to `Render::render`:
/// the failure surface uses window-keyed state, and an element that only works
/// outside a paint would not be much of a test.
fn render_once(context: &mut VisualTestContext, view: &Entity<ScriptView>) {
    let view = view.clone();
    context.draw(
        gpui::Point::default(),
        gpui::size(gpui::px(400.), gpui::px(300.)),
        move |_, _| view.into_any_element(),
    );
}

/// The first `on_change` handler in the view's published snapshot.
fn click_target(context: &mut VisualTestContext, view: &Entity<ScriptView>) -> CallbackId {
    context.update(|_, cx| {
        view.read(cx)
            .snapshot()
            .and_then(first_change_callback)
            .expect("the view should have published a snapshot with a handler")
    })
}

fn first_change_callback(snapshot: &RenderSnapshot) -> Option<CallbackId> {
    (0..snapshot.len() as u32)
        .filter_map(|id| snapshot.arena().node(id))
        .flat_map(|node| node.ops())
        .find_map(|op| match op {
            SpecOp::Callback("on_change", id) => Some(*id),
            _ => None,
        })
}

/// Reads the published description without entering the VM — which is also what
/// makes this safe to call between assertions about the render count.
fn snapshot_text(context: &mut VisualTestContext, view: &Entity<ScriptView>) -> String {
    context.update(|_, cx| {
        view.read(cx)
            .snapshot()
            .map(RenderSnapshot::debug_tree)
            .unwrap_or_default()
    })
}

/// Flips `FLAKY`'s `fail` field by rendering a replacement instance.
///
/// The script has no host-reachable setter, so the flag is set the way a script
/// would set it: through a fresh object whose `init` starts it true.
fn set_flag(
    context: &mut VisualTestContext,
    view: &Entity<ScriptView>,
    runtime: &std::rc::Rc<ShellRuntime>,
) {
    let source = FLAKY.replace("this.fail = false", "this.fail = true");
    let view_type = runtime.load_source("flaky-failing", &source).expect("load");
    let object = context
        .update(|window, cx| runtime.instantiate(&view_type, window, cx))
        .expect("instantiate");
    view.update(context, |view, _| view.replace_object(object));
}

fn another_view(
    context: &mut VisualTestContext,
    runtime: &std::rc::Rc<ShellRuntime>,
    source: &str,
) -> Entity<ScriptView> {
    let view_type = runtime.load_source("second", source).expect("load");
    context.update(|window, cx| {
        let object = runtime
            .instantiate(&view_type, window, cx)
            .expect("instantiate");
        cx.new(|_| ScriptView::new(runtime.clone(), object))
    })
}

/// A window with a script view in it, plus the runtime that owns it.
fn script_view(
    cx: &mut TestAppContext,
    source: &str,
) -> (
    std::rc::Rc<ShellRuntime>,
    VisualTestContext,
    Entity<ScriptView>,
) {
    cx.update(|cx| gpui_shell::init(cx));

    let runtime = ShellRuntime::new().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));

    let view_type = runtime.load_source(ENTRY, source).expect("load");

    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    let view = context.update(|window, cx| {
        let object = runtime
            .instantiate(&view_type, window, cx)
            .expect("instantiate");
        cx.new(|_| ScriptView::new(runtime.clone(), object))
    });

    (runtime, context, view)
}

struct Empty;

impl gpui::Render for Empty {
    fn render(
        &mut self,
        _: &mut gpui::Window,
        _: &mut gpui::Context<Self>,
    ) -> impl gpui::IntoElement {
        gpui::div()
    }
}
