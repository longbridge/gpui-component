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

use crate::{
    RenderSnapshot, ScriptView, ShellRuntime,
    spec::{CallbackId, SpecOp},
};
use gpui::{
    AppContext as _, Entity, IntoElement as _, ParentElement as _, SharedString, Styled as _,
    TestAppContext, VisualTestContext,
};

const TOGGLE: &str = r#"
import { div, View } from "gpui";
import { v_flex, Checkbox } from "gpui-base";

export default class Toggle extends View {
  init() {
    this.count = 0;
  }

  render(cx) {
    return v_flex()
      .child(`count: ${this.count}`)
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

const FPS_MONITOR: &str = r#"
import { View, div } from "gpui";
import { fps_monitor } from "gpui-fps";

export default class Monitor extends View {
  render() {
    return div().relative().size_full().child(fps_monitor());
  }
}
"#;

const PATH: &str = r##"
import { div, View, PathBuilder, Background } from "gpui";

export default class NativePath extends View {
  render() {
    const path = PathBuilder.fill()
      .move_to(0, "100%")
      .line_to("50%", 0)
      .line_to("100%", "100%")
      .close()
      .build();
    return window.paint_path(path, Background.solid(`#16a34a`))
      .w(200)
      .h(80);
  }
}
"##;

#[gpui::test]
fn path_builder_freezes_commands_in_the_render_snapshot(cx: &mut TestAppContext) {
    let (_runtime, mut context, view) = script_view(cx, PATH);

    render_once(&mut context, &view);

    // Not `unwrap()`: a missing snapshot means the render threw, and the
    // panic that reports the absence says nothing about the cause. This one
    // has been seen to fire rarely under load, so when it does it has to
    // arrive with the script's own error attached.
    let tree = context.update(|_, cx| {
        let view = view.read(cx);
        match view.snapshot() {
            Some(snapshot) => snapshot.debug_tree(),
            None => panic!(
                "the render produced no snapshot; the build failed with: {}",
                view.build_error().unwrap_or("no error was recorded either")
            ),
        }
    });
    assert!(tree.contains("path fill"), "{tree}");
    assert!(tree.contains("move_to"), "{tree}");
    assert!(tree.contains("50%"), "{tree}");
    assert!(tree.contains("close"), "{tree}");
}

#[gpui::test]
fn path_dash_rejects_values_that_round_to_zero_pixels(cx: &mut TestAppContext) {
    let source = r##"
import { div, View, PathBuilder } from "gpui";
export default class TinyDash extends View {
  render() {
    const path = PathBuilder.stroke(1)
      .move_to(0, 0)
      .line_to(100, 0)
      .dash_array([Number.MIN_VALUE])
      .build();
    return window.paint_path(path, "#000");
  }
}
"##;
    let (_runtime, mut context, view) = script_view(cx, source);

    render_once(&mut context, &view);

    let error = context.update(|_, cx| view.read(cx).build_error().map(str::to_owned));
    assert!(
        error.is_some_and(|error| error.contains("positive finite pixel numbers")),
        "the unsafe dash must be rejected before native path construction"
    );
}

/// A script whose `render` throws every other call, so a failed build can be
/// observed next to a successful one.
const FLAKY: &str = r#"
import { div, View } from "gpui";
import { v_flex } from "gpui-base";

export default class Flaky extends View {
  init() {
    this.fail = false;
  }

  render(cx) {
    if (this.fail) {
      throw new Error("render failed on purpose");
    }
    return v_flex().child("good");
  }
}
"#;

const ASYNC_FAILURE: &str = r#"
import { div, View } from "gpui";
import { v_flex } from "gpui-base";

export default class AsyncFailure extends View {
  // `init` hands out an async context, and an async context is the flavour a
  // view may keep — a bare `.then` is resumed by the drain, well after the
  // render that queued it returned.
  init(_props, cx) {
    this.cx = cx;
  }
  render() {
    Promise.resolve().then(() => this.cx.notify());
    throw new Error("render failed after queueing work");
  }
}
"#;

const ALWAYS_FAILS: &str = r#"
import { div, View } from "gpui";
export default class AlwaysFails extends View {
  render() { throw new Error("first render failed on purpose"); }
}
"#;

const INPUT_SUBSCRIPTION: &str = r#"
import { div, View } from "gpui";
import { v_flex, InputState } from "gpui-base";

export default class InputSubscription extends View {
  init(_props, cx) {
    this.count = 0;
    this.field = InputState.new({});
    this.field.on("submit", (_event, cx) => {
      this.count += 1;
      cx.notify();
    });
  }

  render() {
    return v_flex().child(`submits: ${this.count}`);
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
fn shell_root_reuses_a_clean_views_materialized_subtree(cx: &mut TestAppContext) {
    let (runtime, mut context, view) = script_view(cx, TOGGLE);
    context.update(|window, cx| {
        window.replace_root(cx, |window, cx| {
            crate::root::ShellRoot::new(view.clone().into(), window, cx)
        })
    });

    context.update(|window, cx| window.draw(cx).clear(cx));
    let first = runtime.metrics().read();
    let started = std::time::Instant::now();
    for _ in 0..64 {
        context.update(|window, cx| window.draw(cx).clear(cx));
    }
    let elapsed = started.elapsed();

    let clean = runtime.metrics().read().since(&first);
    eprintln!(
        "clean_frames=64 elapsed={elapsed:?} materializations={} materialize_time={:?}",
        clean.materializations(),
        clean.materialize_time(),
    );
    assert_eq!(clean.script_renders(), 0);
    assert_eq!(
        clean.materializations(),
        0,
        "64 clean window frames must reuse the subtree produced by the first materialization"
    );

    view.update(&mut context, |view, cx| view.refresh(cx));
    context.update(|window, cx| window.draw(cx).clear(cx));

    let refreshed = runtime.metrics().read().since(&first);
    assert_eq!(refreshed.script_renders(), 1);
    assert_eq!(
        refreshed.materializations(),
        1,
        "refreshing the script view must invalidate and replace the cached subtree once"
    );
}

#[gpui::test]
fn shell_fps_monitor_does_not_drive_the_window_unless_requested(cx: &mut TestAppContext) {
    let (_runtime, mut context, view) = script_view(cx, FPS_MONITOR);
    context.update(|window, cx| {
        window.replace_root(cx, |window, cx| {
            crate::root::ShellRoot::new(view.clone().into(), window, cx)
        })
    });

    context.update(|window, cx| window.draw(cx).clear(cx));
    let requested = context.update(|window, cx| window.simulate_next_frame(cx));

    assert_eq!(
        requested, 0,
        "the diagnostic HUD must observe application frames rather than create a redraw loop"
    );
}

#[gpui::test]
fn shell_fps_monitor_can_explicitly_drive_a_sustained_frame_test(cx: &mut TestAppContext) {
    let source = FPS_MONITOR.replace("fps_monitor()", "fps_monitor().continuous(true)");
    let (_runtime, mut context, view) = script_view(cx, &source);
    context.update(|window, cx| {
        window.replace_root(cx, |window, cx| {
            crate::root::ShellRoot::new(view.clone().into(), window, cx)
        })
    });

    context.update(|window, cx| window.draw(cx).clear(cx));
    let requested = context.update(|window, cx| window.simulate_next_frame(cx));

    assert!(
        requested > 0,
        "continuous(true) must remain an explicit sustained-frame diagnostic mode"
    );
}

#[gpui::test]
fn a_changed_motion_target_requests_native_frames_without_reentering_js(cx: &mut TestAppContext) {
    let source = r#"
import { View, div } from "gpui";
import { Checkbox } from "gpui-base";

export default class Panel extends View {
  init() { this.expanded = false; }
  render(cx) {
    return div()
      .id("panel")
      .w(this.expanded ? 320 : 64)
      .transition("width", { duration: 180 })
      .child(
        Checkbox.new("expand").on_change((expanded, cx) => {
          this.expanded = expanded;
          cx.notify();
        }),
      );
  }
}
"#;
    let (runtime, mut context, view) = script_view(cx, source);
    render_once(&mut context, &view);
    let callback = click_target(&mut context, &view);
    context.update(|window, cx| runtime.dispatch_change(callback, true, window, cx));
    render_once(&mut context, &view);

    let before_frames = runtime.metrics().read();
    let mut native_frames = 0;
    for _ in 0..120 {
        context
            .executor()
            .advance_clock(std::time::Duration::from_millis(2));
        native_frames += context.update(|window, cx| window.simulate_next_frame(cx));
        render_once(&mut context, &view);
    }
    assert!(
        native_frames > 1,
        "retargeting width must schedule native animation frames"
    );
    let after_frames = runtime.metrics().read();
    assert_eq!(
        after_frames.script_renders(),
        2,
        "120 native animation frames must not enter QuickJS"
    );
    assert!(
        after_frames.materializations() >= before_frames.materializations() + 120,
        "animation frames must repeatedly materialize the retained snapshot"
    );
}

#[gpui::test]
fn a_changed_spring_target_requests_native_frames_without_reentering_js(cx: &mut TestAppContext) {
    let source = r#"
import { View, div } from "gpui";
import { Checkbox } from "gpui-base";

export default class Indicator extends View {
  init() { this.selected = false; }
  render(cx) {
    return div()
      .id("indicator")
      .left(this.selected ? 240 : 0)
      .spring("left", { response: 250, damping: 0.85 })
      .child(
        Checkbox.new("select").on_change((selected, cx) => {
          this.selected = selected;
          cx.notify();
        }),
      );
  }
}
"#;
    let (runtime, mut context, view) = script_view(cx, source);
    render_once(&mut context, &view);
    let callback = click_target(&mut context, &view);
    context.update(|window, cx| runtime.dispatch_change(callback, true, window, cx));
    render_once(&mut context, &view);

    let pending = context.update(|window, cx| window.simulate_next_frame(cx));
    assert_eq!(
        pending, 1,
        "retargeting left must schedule a native spring frame"
    );
    assert_eq!(runtime.metrics().read().script_renders(), 2);
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
fn notify_from_an_input_subscription_rebuilds_its_own_view(cx: &mut TestAppContext) {
    cx.update(crate::init);
    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));
    let view_type = runtime
        .load_source("input-subscription.js", INPUT_SUBSCRIPTION)
        .expect("load");
    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    let view = context
        .update(|window, cx| runtime.instantiate_view(&view_type, window, cx))
        .expect("instantiate under the final ScriptView owner");
    render_once(&mut context, &view);

    let input = runtime
        .entities()
        .first_input()
        .expect("the script created an input state");
    context.update(|_, cx| {
        input.update(cx, |_, cx| {
            cx.emit(gpui_base::input::InputEvent::PressEnter {
                secondary: false,
                shift: false,
            });
        });
    });
    context.run_until_parked();
    assert!(
        context.update(|_, cx| view.read(cx).is_dirty()),
        "the subscription's cx.notify() did not invalidate its owner"
    );
    render_once(&mut context, &view);

    assert!(
        snapshot_text(&mut context, &view).contains("submits: 1"),
        "cx.notify() from a retained input subscription must invalidate its owner"
    );
    assert_eq!(runtime.metrics().read().script_renders(), 2);
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
fn a_cloned_snapshot_retires_its_callback_generation_only_after_the_last_clone(
    cx: &mut TestAppContext,
) {
    let (runtime, mut context, view) = script_view(cx, TOGGLE);
    render_once(&mut context, &view);
    let callback = click_target(&mut context, &view);
    let retained = context.update(|_, cx| view.read(cx).snapshot().unwrap().clone());

    for _ in 0..2 {
        view.update(&mut context, |view, cx| view.refresh(cx));
        render_once(&mut context, &view);
    }
    assert!(runtime.live_callback_ids().contains(&callback));
    drop(retained);
    assert!(!runtime.live_callback_ids().contains(&callback));
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
fn a_failed_first_render_is_not_retried_every_frame(cx: &mut TestAppContext) {
    let (runtime, mut context, view) = script_view(cx, ALWAYS_FAILS);

    render_once(&mut context, &view);
    let after_failure = runtime.metrics().read().script_renders();
    assert_eq!(after_failure, 1);
    for _ in 0..16 {
        render_once(&mut context, &view);
    }

    assert_eq!(runtime.metrics().read().script_renders(), after_failure);
    assert!(!view.read_with(&context, |view, _| view.is_dirty()));

    view.update(&mut context, |view, _| view.invalidate());
    render_once(&mut context, &view);
    assert_eq!(runtime.metrics().read().script_renders(), after_failure + 1);
}

#[gpui::test]
fn a_failed_render_continuation_keeps_its_original_view(cx: &mut TestAppContext) {
    let (runtime, mut context, failed) = script_view(cx, ASYNC_FAILURE);
    let other = another_view(&mut context, &runtime, TOGGLE);
    render_once(&mut context, &other);

    render_once(&mut context, &failed);
    context.run_until_parked();

    assert!(failed.read_with(&context, |view, _| view.is_dirty()));
    assert!(
        !other.read_with(&context, |view, _| view.is_dirty()),
        "the failed view's continuation invalidated another view"
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
        gpui_base::Theme::global_mut(cx).tokens.colors.background = gpui::black();
    });
    render_once(&mut context, &view);

    assert_eq!(
        runtime.metrics().read().script_renders(),
        2,
        "a palette change must reach script views"
    );
}

#[gpui::test]
fn an_appearance_only_change_rebuilds_the_snapshot(cx: &mut TestAppContext) {
    let (runtime, mut context, view) = script_view(cx, TOGGLE);

    render_once(&mut context, &view);
    assert_eq!(runtime.metrics().read().script_renders(), 1);

    context.update(|_, cx| {
        gpui_base::Theme::global_mut(cx).appearance = gpui_base::ThemeAppearance::Dark;
    });
    render_once(&mut context, &view);

    assert_eq!(
        runtime.metrics().read().script_renders(),
        2,
        "appearance is part of cx.theme(), so changing it must invalidate script views"
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

/// A window root that mounts the script view uncached, so a cached subtree
/// inside it is the one level of cache gpui reuses.
struct Host(Entity<ScriptView>);

impl gpui::Render for Host {
    fn render(
        &mut self,
        _: &mut gpui::Window,
        _: &mut gpui::Context<Self>,
    ) -> impl gpui::IntoElement {
        gpui::div().size_full().child(self.0.clone())
    }
}

/// Makes `view` the window's content under an uncached `Host` root.
fn mount(context: &mut VisualTestContext, view: &Entity<ScriptView>) {
    let view = view.clone();
    context.update(|window, cx| window.replace_root(cx, |_, _| Host(view)));
}

/// One real window frame: `Window::draw`, with gpui's cached-view
/// bookkeeping, which `render_once` (an element drawn by hand) skips.
fn draw_frame(context: &mut VisualTestContext) {
    context.update(|window, cx| window.draw(cx).clear(cx));
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
    cx.update(|cx| crate::init(cx));

    let runtime = ShellRuntime::new_isolated().expect("runtime");
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

const TWO_CACHED_PANELS: &str = r#"
import { View, div } from "gpui";
import { v_flex, h_flex, Checkbox } from "gpui-base";

export default class Panels extends View {
  init() {
    this.expanded = false;
    this.label = "left";
  }

  render(cx) {
    return h_flex()
      .size_full()
      .child(
        v_flex()
          .id("left")
          .flex_1()
          .min_h(0)
          .cached()
          .child(this.label)
          .child(
            div()
              .id("bar")
              .w(this.expanded ? 200 : 40)
              .h(8)
              .transition("width", { duration: 180 }),
          )
          .child(
            Checkbox.new("expand").on_change((expanded, cx) => {
              this.expanded = expanded;
              this.label = expanded ? "left, expanded" : "left";
              cx.notify();
            }),
          ),
      )
      .child(v_flex().id("right").flex_1().min_h(0).cached().child("right"));
  }
}
"#;

#[gpui::test]
fn a_cached_element_without_an_id_is_drawn_plain(cx: &mut TestAppContext) {
    let source = r#"
import { View, div } from "gpui";

export default class Plain extends View {
  render() {
    return div().size_full().cached().child("no id here");
  }
}
"#;
    let (runtime, mut context, view) = script_view(cx, source);
    mount(&mut context, &view);

    let metrics = runtime.metrics().read();
    assert_eq!(
        metrics.subtree_mounts(),
        0,
        "an element without an id must not be mounted as a cached subtree"
    );
    assert!(
        snapshot_text(&mut context, &view).contains("no id here"),
        "the element still has to be described and drawn"
    );

    // Named once for the description rather than once for every frame that
    // draws it: this is the list the warning is read off.
    context.update(|_, cx| {
        assert_eq!(
            view.read(cx)
                .snapshot()
                .expect("published")
                .cached_nodes()
                .id_less(),
            ["div"],
            "the report names the component that asked for a cache it cannot have"
        );
    });
    let before = runtime.metrics().read();
    draw_frame(&mut context);
    draw_frame(&mut context);
    assert_eq!(
        runtime.metrics().read().since(&before).script_renders(),
        0,
        "the frames that redraw it do not rebuild the description, so nothing recomputes the list"
    );
}

#[gpui::test]
fn a_cached_element_is_mounted_once_per_view_render(cx: &mut TestAppContext) {
    let (runtime, mut context, view) = script_view(cx, TWO_CACHED_PANELS);
    mount(&mut context, &view);

    let metrics = runtime.metrics().read();
    assert_eq!(
        metrics.subtree_mounts(),
        2,
        "two cached() elements, two mounts"
    );
    assert_eq!(
        metrics.subtree_rebuilds(),
        2,
        "a subtree gpui has never drawn renders on its first frame"
    );
    assert_eq!(metrics.materializations(), 1);
}

#[gpui::test]
fn a_script_rebuild_reaches_every_cached_subtree_in_the_same_frame(cx: &mut TestAppContext) {
    let (runtime, mut context, view) = script_view(cx, TWO_CACHED_PANELS);
    mount(&mut context, &view);
    let callback = click_target(&mut context, &view);
    let before = runtime.metrics().read();

    // `dispatch_change` runs the handler, whose `cx.notify()` goes through
    // `ScriptView::refresh` outside any draw — the path a real event takes.
    context.update(|window, cx| runtime.dispatch_change(callback, true, window, cx));
    draw_frame(&mut context);

    let after = runtime.metrics().read().since(&before);
    assert_eq!(after.script_renders(), 1);
    assert_eq!(
        after.subtree_rebuilds(),
        2,
        "both cached subtrees must draw the new description in the frame that rebuilt it"
    );
    let caches = runtime.subtree_caches().entities(view.entity_id());
    assert_eq!(caches.len(), 2);
    let described = context.update(|_, cx| {
        caches
            .iter()
            .map(|cache| {
                let cache = cache.read(cx);
                view.read(cx)
                    .snapshot()
                    .expect("published")
                    .arena()
                    .debug_tree(cache.root())
            })
            .collect::<Vec<_>>()
    });
    assert!(
        described.iter().any(|tree| tree.contains("left, expanded")),
        "the left subtree must point at the rebuilt description: {described:?}"
    );

    // The frame after: nothing changed, nothing rebuilds.
    let settled = runtime.metrics().read();
    draw_frame(&mut context);
    let quiet = runtime.metrics().read().since(&settled);
    assert_eq!(quiet.subtree_rebuilds(), 0);
    assert_eq!(quiet.subtree_mounts(), 2);
}

#[gpui::test]
fn a_cached_subtree_that_leaves_the_description_is_dropped(cx: &mut TestAppContext) {
    let source = r#"
import { View, div } from "gpui";
import { v_flex, Checkbox } from "gpui-base";

export default class Optional extends View {
  init() { this.shown = true; }
  render(cx) {
    return v_flex()
      .size_full()
      .child(
        Checkbox.new("toggle").on_change((shown, cx) => {
          this.shown = shown;
          cx.notify();
        }),
      )
      .when(this.shown, (el) =>
        el.child(div().id("optional").size_full().cached().child("optional")),
      );
  }
}
"#;
    let (runtime, mut context, view) = script_view(cx, source);
    mount(&mut context, &view);
    assert_eq!(runtime.subtree_caches().entities(view.entity_id()).len(), 1);

    let callback = click_target(&mut context, &view);
    context.update(|window, cx| runtime.dispatch_change(callback, false, window, cx));
    draw_frame(&mut context);

    assert!(
        runtime
            .subtree_caches()
            .entities(view.entity_id())
            .is_empty(),
        "an id the new description no longer marks must release its entity"
    );
}

#[gpui::test]
fn dropping_a_view_drops_its_cached_subtrees(cx: &mut TestAppContext) {
    let (runtime, mut context, view) = script_view(cx, TWO_CACHED_PANELS);
    // Not `mount`: mounting under `Host` gives the window root its own
    // permanent clone of `view`, so the entity this test drops would never
    // reach a zero refcount. `render_once` draws through a temporary closure
    // that does not outlive the call, which is what lets `drop(view)` below
    // be the entity's last strong reference.
    render_once(&mut context, &view);
    let view_id = view.entity_id();
    assert_eq!(runtime.subtree_caches().entities(view_id).len(), 2);

    drop(view);
    // Released entities are dropped when the app next flushes its effects,
    // which any update does.
    context.update(|_, _| ());

    assert!(
        runtime.subtree_caches().entities(view_id).is_empty(),
        "a dropped view must not leave its caches in the runtime"
    );
}

#[gpui::test]
fn a_rebuild_from_inside_the_draw_reaches_cached_subtrees_a_frame_later(cx: &mut TestAppContext) {
    // `replace_object` is the hot-reload path: it marks the view dirty without
    // notifying, so the rebuild happens inside the next draw, where a notify
    // cannot reach the frame being drawn. The subtrees follow one frame later.
    let (runtime, mut context, view) = script_view(cx, TWO_CACHED_PANELS);
    mount(&mut context, &view);

    let reloaded =
        TWO_CACHED_PANELS.replace("this.label = \"left\";", "this.label = \"reloaded\";");
    let view_type = runtime
        .load_source("panels-reloaded", &reloaded)
        .expect("load");
    let object = context
        .update(|window, cx| runtime.instantiate(&view_type, window, cx))
        .expect("instantiate");
    view.update(&mut context, |view, _| view.replace_object(object));

    let before = runtime.metrics().read();
    draw_frame(&mut context);
    let rebuilt = runtime.metrics().read().since(&before);
    assert_eq!(
        rebuilt.script_renders(),
        1,
        "the reload rebuilt inside this draw"
    );
    assert_eq!(
        rebuilt.subtree_rebuilds(),
        0,
        "a notify from inside the draw lands in the next frame, not this one"
    );

    // Not `draw_frame`: the point of the fallback is that it *asks* for the
    // frame it needs. `on_next_frame` is the only demand a clean window
    // answers, so running the scheduled callbacks — which notify the caches
    // outside any draw, and so dirty the window and draw it at the end of
    // this update — is what the next frame really is.
    let scheduled = context.update(|window, cx| window.simulate_next_frame(cx));
    assert!(
        scheduled >= 1,
        "the fallback must schedule the frame it needs, not merely notify into one"
    );
    let following = runtime.metrics().read().since(&before);
    assert_eq!(
        following.subtree_rebuilds(),
        2,
        "the frame after shows the reloaded description in every cached subtree"
    );
}

#[gpui::test]
fn an_animation_in_a_cached_subtree_rebuilds_only_that_subtree(cx: &mut TestAppContext) {
    let (runtime, mut context, view) = script_view(cx, TWO_CACHED_PANELS);
    mount(&mut context, &view);
    draw_frame(&mut context);
    let callback = click_target(&mut context, &view);
    context.update(|window, cx| runtime.dispatch_change(callback, true, window, cx));
    draw_frame(&mut context);

    let before = runtime.metrics().read();
    let mut native_frames = 0;
    let frames = 30;
    for _ in 0..frames {
        context
            .executor()
            .advance_clock(std::time::Duration::from_millis(2));
        // Not followed by `draw_frame`: in test builds, `App::flush_effects`
        // draws every dirty window once its pending effects drain (see
        // `flush_effects` in gpui's `app.rs`), and the callback this runs
        // dirties the window via `cx.notify`. That auto-draw *is* the frame
        // this loop is driving; an explicit `draw_frame` here would be a
        // second, redundant real draw of the same animation frame and would
        // double every per-draw count below.
        native_frames += context.update(|window, cx| window.simulate_next_frame(cx));
    }
    let during = runtime.metrics().read().since(&before);

    assert!(
        native_frames > 1,
        "the width transition must request native frames"
    );
    assert_eq!(
        during.script_renders(),
        0,
        "animation frames never enter the VM"
    );
    assert_eq!(
        during.subtree_mounts(),
        2 * frames,
        "the view's own materialization mounts both subtrees every frame"
    );
    assert_eq!(
        during.subtree_rebuilds(),
        frames,
        "only the subtree the transition lives in rebuilds; the other is reused"
    );
}

#[gpui::test]
fn the_root_view_is_cached_only_while_it_has_no_cached_subtrees(cx: &mut TestAppContext) {
    // Without markers: #2908 holds, a clean frame materializes nothing.
    let (runtime, mut context, view) = script_view(cx, TOGGLE);
    context.update(|window, cx| {
        window.replace_root(cx, |window, cx| {
            crate::root::ShellRoot::new(view.clone().into(), window, cx)
        })
    });
    context.update(|window, cx| window.draw(cx).clear(cx));
    let first = runtime.metrics().read();
    context.update(|window, cx| window.draw(cx).clear(cx));
    let clean = runtime.metrics().read().since(&first);
    assert_eq!(
        clean.materializations(),
        0,
        "a markerless view keeps the root cache"
    );

    // With markers: the view materializes its skeleton on a clean frame and
    // every subtree is reused.
    let (runtime, mut context, view) = script_view(cx, TWO_CACHED_PANELS);
    context.update(|window, cx| {
        window.replace_root(cx, |window, cx| {
            crate::root::ShellRoot::new(view.clone().into(), window, cx)
        })
    });
    context.update(|window, cx| window.draw(cx).clear(cx));
    let first = runtime.metrics().read();
    context.update(|window, cx| window.draw(cx).clear(cx));
    let clean = runtime.metrics().read().since(&first);
    assert_eq!(clean.script_renders(), 0);
    assert_eq!(
        clean.materializations(),
        1,
        "a view with cached subtrees is mounted uncached and materializes its skeleton"
    );
    assert_eq!(clean.subtree_mounts(), 2);
    assert_eq!(
        clean.subtree_rebuilds(),
        0,
        "both subtrees are reused on a clean frame"
    );
}

#[gpui::test]
fn a_cached_subtree_inside_an_element_argument_is_a_subtree_of_this_view(cx: &mut TestAppContext) {
    // A registered component can take a whole element as a constructor
    // argument. Such a node is claimed off the tree — no parent lists it as a
    // child — but it is materialized with the same snapshot, so a `.cached()`
    // inside it is mounted as one of this view's subtrees like any other. The
    // walk that lists the keys has to reach it, or the view believes it caches
    // nothing (and keeps the root cache, defeating every subtree) while
    // `retain` drops the entity after every rebuild.
    use crate::{
        ArgumentDescriptor, ArgumentSchema, COMPONENT_REGISTRY_API_VERSION, ComponentArgument,
        ComponentDescriptor, ComponentMaterializer, ComponentPayload, ComponentRegistry,
        ConstructorDescriptor, MaterializeRequest,
    };
    use gpui::{AnyElement, IntoElement as _, div};
    use std::sync::Arc;

    struct WrapMaterializer;
    impl ComponentMaterializer for WrapMaterializer {
        fn materialize(&self, mut request: MaterializeRequest<'_>) -> anyhow::Result<AnyElement> {
            let argument = request
                .payload()
                .downcast_ref::<ComponentArgument>()
                .cloned()
                .ok_or_else(|| anyhow::anyhow!("Wrap payload"))?;
            let child = request.resolve_element(&argument)?;
            Ok(div().size_full().child(child).into_any_element())
        }
    }

    cx.update(crate::init);
    let mut registry = ComponentRegistry::new(
        COMPONENT_REGISTRY_API_VERSION,
        crate::DEFAULT_COMPONENT_MODULE,
    )
    .unwrap();
    registry
        .register(
            ComponentDescriptor::new("Wrap", Arc::new(WrapMaterializer)).with_constructors(vec![
                ConstructorDescriptor::new(
                    "Wrap",
                    vec![ArgumentDescriptor::new("child", ArgumentSchema::Element)],
                    |arguments| Ok(ComponentPayload::new(arguments[0].clone())),
                ),
            ]),
        )
        .unwrap();
    let runtime =
        ShellRuntime::new_isolated_with_components(registry.freeze().unwrap()).expect("runtime");
    cx.update(|cx| runtime.set_global(cx));

    let source = r#"
import { View, div } from "gpui";
import { v_flex, Checkbox } from "gpui-base";
import { Wrap } from "gpui-component";

export default class Wrapped extends View {
  init() { this.label = "before"; }
  render(cx) {
    return v_flex()
      .size_full()
      .child(
        Checkbox.new("bump").on_change((_, cx) => {
          this.label = "after";
          cx.notify();
        }),
      )
      .child(
        new Wrap(
          div().id("argument").size_full().cached().child(this.label),
        ),
      );
  }
}
"#;
    let view_type = runtime.load_source(ENTRY, source).expect("load");
    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    let view = context.update(|window, cx| {
        let object = runtime
            .instantiate(&view_type, window, cx)
            .expect("instantiate");
        cx.new(|_| ScriptView::new(runtime.clone(), object))
    });
    mount(&mut context, &view);

    context.update(|_, cx| {
        let view = view.read(cx);
        let snapshot = view.snapshot().expect("published");
        assert!(
            snapshot.cached_nodes().holds(&"argument".into()),
            "a .cached() held as an element argument is still one of this view's subtrees"
        );
        assert!(
            view.caches_subtrees(),
            "so the root must mount this view uncached"
        );
    });

    let entities = runtime.subtree_caches().entities(view.entity_id());
    assert_eq!(entities.len(), 1, "the argument's subtree has an entity");
    let before = entities[0].entity_id();

    let callback = click_target(&mut context, &view);
    context.update(|window, cx| runtime.dispatch_change(callback, true, window, cx));
    draw_frame(&mut context);

    let entities = runtime.subtree_caches().entities(view.entity_id());
    assert_eq!(entities.len(), 1);
    assert_eq!(
        entities[0].entity_id(),
        before,
        "an id the description still marks keeps its entity, and with it every \
         transition, scroll offset and hover state inside the subtree"
    );
}

/// The ids this description marks `.cached()` more than once.
fn duplicate_ids(snapshot: &RenderSnapshot) -> Vec<&str> {
    snapshot
        .cached_nodes()
        .duplicates()
        .iter()
        .map(SharedString::as_ref)
        .collect()
}

const DUPLICATE_SIBLINGS: &str = r#"
import { View } from "gpui";
import { v_flex, h_flex } from "gpui-base";

export default class Twins extends View {
  render() {
    return h_flex()
      .size_full()
      .child(v_flex().id("panel").flex_1().min_h(0).cached().child("first"))
      .child(v_flex().id("panel").flex_1().min_h(0).cached().child("second"));
  }
}
"#;

const DUPLICATE_NESTED: &str = r#"
import { View } from "gpui";
import { v_flex } from "gpui-base";

export default class Nested extends View {
  render() {
    return v_flex()
      .size_full()
      .child(
        v_flex()
          .id("panel")
          .size_full()
          .cached()
          .child("outer")
          .child(v_flex().id("panel").size_full().cached().child("inner")),
      );
  }
}
"#;

#[gpui::test]
fn two_cached_elements_sharing_an_id_mount_one_subtree(cx: &mut TestAppContext) {
    // One id, one entity. Mounting both would give them one `SubtreeCache`
    // between them — last `describe` wins, so both would draw the second
    // subtree — and one `ElementId::View`, so they would overwrite each
    // other's prepaint and paint ranges. The second is drawn plain instead.
    let (runtime, mut context, view) = script_view(cx, DUPLICATE_SIBLINGS);
    mount(&mut context, &view);

    context.update(|_, cx| {
        let view = view.read(cx);
        let snapshot = view.snapshot().expect("published");
        assert_eq!(
            duplicate_ids(snapshot),
            ["panel"],
            "the repeated id is reported, once for the snapshot"
        );
    });
    assert_eq!(
        runtime.subtree_caches().entities(view.entity_id()).len(),
        1,
        "one id is one entity, however many elements claim it"
    );
    assert_eq!(runtime.metrics().read().subtree_mounts(), 1);
    let text = snapshot_text(&mut context, &view);
    assert!(text.contains("first") && text.contains("second"), "{text}");
}

#[gpui::test]
fn a_cached_element_nested_under_its_own_id_is_drawn_plain(cx: &mut TestAppContext) {
    // The shape that used to panic: the inner element resolves to the entity
    // gpui has already leased to render the outer one, and `describe` would
    // borrow it a second time.
    let (runtime, mut context, view) = script_view(cx, DUPLICATE_NESTED);
    mount(&mut context, &view);
    draw_frame(&mut context);

    context.update(|_, cx| {
        assert_eq!(
            duplicate_ids(view.read(cx).snapshot().expect("published")),
            ["panel"]
        );
    });
    assert_eq!(
        runtime.subtree_caches().entities(view.entity_id()).len(),
        1,
        "the outer element keeps the id; the inner one is drawn plain"
    );
    let text = snapshot_text(&mut context, &view);
    assert!(text.contains("outer") && text.contains("inner"), "{text}");
}

#[gpui::test]
fn cached_false_marks_no_subtree(cx: &mut TestAppContext) {
    // `.cached(false)` is what a script writes to turn the call off for one
    // branch. The pass that lists the keys has to read the flag exactly as
    // `apply_behavior` does, or the view would keep an entity — and mount its
    // root uncached — for a subtree nothing ever mounts.
    let source = r#"
import { View } from "gpui";
import { v_flex } from "gpui-base";

export default class Off extends View {
  render() {
    return v_flex().id("panel").size_full().cached(false).child("plain");
  }
}
"#;
    let (runtime, mut context, view) = script_view(cx, source);
    mount(&mut context, &view);

    context.update(|_, cx| {
        let view = view.read(cx);
        assert!(
            view.snapshot()
                .expect("published")
                .cached_nodes()
                .is_empty(),
            "cached(false) marks nothing"
        );
        assert!(
            !view.caches_subtrees(),
            "so the root keeps its own cache, as it does for a view with no markers"
        );
    });
    assert_eq!(runtime.metrics().read().subtree_mounts(), 0);
    assert!(
        runtime
            .subtree_caches()
            .entities(view.entity_id())
            .is_empty()
    );
}
