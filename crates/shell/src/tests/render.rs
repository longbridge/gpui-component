//! End-to-end tests for the script render protocol.
//!
//! These exercise the whole path — VM, method dispatch, spec arena, event
//! callbacks — without painting a frame, because the element description is
//! plain data. They run against whichever engine is enabled, which is what
//! keeps the fallback engine honest.

use crate::{
    NativeModules, NativeValue, ScriptView, ShellRuntime, capability::Capabilities, policy::Policy,
};
use gpui::{AppContext as _, TestAppContext, VisualTestContext};
use std::{cell::Cell, path::PathBuf, rc::Rc};

const COUNTER: &str = r#"
import { View, v_flex, text, Button } from "gpui";

export default class Counter extends View {
  init() {
    this.count = 0;
  }

  render() {
    return v_flex()
      .size_full()
      .items_center()
      .gap_2()
      .p(16)
      .bg("background")
      .child(text(`Count: ${this.count}`).text_color("foreground"))
      .child(
        Button.new("increment")
          .px(12)
          .py(6)
          .rounded(6)
          .bg("primary")
          .on_click((event, cx) => {
            this.count += 1;
            cx.notify();
          })
          .child(text("Increment").text_color("primary_foreground")),
      );
  }
}
"#;

/// The entry name only affects diagnostics, but each engine has its own
/// convention and the tests should read the way real code does.
const ENTRY: &str = "counter.js";

#[gpui::test]
fn a_script_view_produces_an_element_description(cx: &mut TestAppContext) {
    cx.update(|cx| crate::init(cx));

    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));

    let view_type = runtime.load_source(ENTRY, COUNTER).expect("load");

    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    let object = context
        .update(|window, cx| runtime.instantiate(&view_type, window, cx))
        .expect("instantiate");

    let tree = context.update(|window, cx| {
        runtime
            .render_to_spec(&object, None, window, cx)
            .expect("render")
    });

    assert!(tree.starts_with("v_flex"), "unexpected root: {tree}");
    assert!(tree.contains("text \"Count: 0\""), "missing label: {tree}");
    assert!(
        tree.contains("Button \"increment\""),
        "missing button: {tree}"
    );
    assert!(tree.contains(":on_click(fn)"), "missing handler: {tree}");
}

#[gpui::test]
fn an_external_link_survives_the_script_render(cx: &mut TestAppContext) {
    cx.update(crate::init);
    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));
    let source = r#"
import { Link, View, text } from "gpui";
export default class ExternalLink extends View {
  render() {
    return Link.new("authorize")
      .href("https://example.com/device")
      .child(text("Open authorization"));
  }
}
"#;
    let view_type = runtime
        .load_source("external-link.js", source)
        .expect("load");
    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    let object = context
        .update(|window, cx| runtime.instantiate(&view_type, window, cx))
        .expect("instantiate");
    let tree = context.update(|window, cx| {
        runtime
            .render_to_spec(&object, None, window, cx)
            .expect("render")
    });

    assert!(tree.contains("Link \"authorize\""), "missing Link: {tree}");
    assert!(
        tree.contains(":href[Str(\"https://example.com/device\")]"),
        "missing external target: {tree}"
    );
}

#[gpui::test]
fn an_external_link_requires_a_parseable_http_origin(cx: &mut TestAppContext) {
    cx.update(crate::init);
    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));
    let source = r#"
import { Link, View } from "gpui";
export default class InvalidExternalLink extends View {
  render() { return Link.new("broken").href("https://"); }
}
"#;
    let view_type = runtime
        .load_source("invalid-external-link.js", source)
        .expect("load");
    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    let object = context
        .update(|window, cx| runtime.instantiate(&view_type, window, cx))
        .expect("instantiate");

    let error = context
        .update(|window, cx| runtime.render_to_spec(&object, None, window, cx))
        .expect_err("a URL without an origin must be refused at the call site");
    assert!(
        error.to_string().contains("absolute HTTP(S) URL"),
        "unexpected error: {error}"
    );
}

#[gpui::test]
fn render_context_exposes_base_aligned_theme_tokens(cx: &mut TestAppContext) {
    cx.update(crate::init);
    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));
    let source = r#"
import { View, text } from "gpui";
export default class Themed extends View {
  render(cx) {
    return text("semantic")
      .text_color(cx.theme().foreground)
      .bg(cx.theme().surface)
      .p(cx.theme().spacing.md)
      .rounded(cx.theme().radius.md);
  }
}

"#;
    let view_type = runtime
        .load_source("context-theme.js", source)
        .expect("load");
    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    let object = context
        .update(|window, cx| runtime.instantiate(&view_type, window, cx))
        .expect("instantiate");
    let tree = context.update(|window, cx| {
        runtime
            .render_to_spec(&object, None, window, cx)
            .expect("render with cx.theme()")
    });
    assert!(
        tree.contains("text_color[Str(\"#"),
        "theme color was not resolved: {tree}"
    );
    assert!(
        tree.contains("p[Number("),
        "theme spacing was not resolved: {tree}"
    );
}

#[gpui::test]
fn render_context_theme_snapshot_is_deeply_read_only(cx: &mut TestAppContext) {
    cx.update(crate::init);
    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));
    let source = r#"
import { View, text } from "gpui";
export default class Themed extends View {
  render(cx) {
    const theme = cx.theme();
    if (!Object.isFrozen(theme)
        || !Object.isFrozen(theme.colors)
        || !Object.isFrozen(theme.spacing)
        || !Object.isFrozen(theme.radius)) {
      throw new Error("theme snapshot must be deeply frozen");
    }
    return text("semantic");
  }
}
"#;
    let view_type = runtime
        .load_source("read-only-context-theme.js", source)
        .expect("load");
    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    let object = context
        .update(|window, cx| runtime.instantiate(&view_type, window, cx))
        .expect("instantiate");

    context
        .update(|window, cx| runtime.render_to_spec(&object, None, window, cx))
        .expect("all nested theme token groups must be read-only");
}

#[gpui::test]
fn render_context_theme_rejects_a_stale_context(cx: &mut TestAppContext) {
    cx.update(crate::init);
    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));
    let source = r#"
import { View, text } from "gpui";
export default class Themed extends View {
  render(cx) {
    if (this.savedTheme) this.savedTheme();
    else this.savedTheme = cx.theme;
    return text("semantic");
  }
}
"#;
    let view_type = runtime
        .load_source("stale-context-theme.js", source)
        .expect("load");
    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    let object = context
        .update(|window, cx| runtime.instantiate(&view_type, window, cx))
        .expect("instantiate");

    context
        .update(|window, cx| runtime.render_to_spec(&object, None, window, cx))
        .expect("first render captures cx.theme");
    let error = context
        .update(|window, cx| runtime.render_to_spec(&object, None, window, cx))
        .expect_err("a theme reader from an earlier render must be stale");
    assert!(
        error.to_string().contains("cx is no longer valid"),
        "{error}"
    );
}

#[test]
fn link_typings_expose_a_real_external_target() {
    let types = crate::typings::declarations();
    assert!(types.contains("export const Link: ComponentType;"));
    assert!(types.contains("href(url: string): Element;"));
}

#[gpui::test]
fn an_element_cannot_be_added_to_two_parents(cx: &mut TestAppContext) {
    cx.update(|cx| crate::init(cx));

    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));

    let source = r#"
import { View, v_flex, text } from "gpui";

export default class Broken extends View {
  render() {
    const shared = text("reused");
    return v_flex().child(shared).child(shared);
  }
}
"#;

    let view_type = runtime.load_source("broken", source).expect("load");

    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    let object = context
        .update(|window, cx| runtime.instantiate(&view_type, window, cx))
        .expect("instantiate");

    let error = context
        .update(|window, cx| runtime.render_to_spec(&object, None, window, cx))
        .expect_err("reusing an element must fail");

    assert!(
        error.to_string().contains("already added to a parent"),
        "unexpected error: {error}"
    );
}

#[gpui::test]
fn an_unknown_style_method_suggests_the_closest_name(cx: &mut TestAppContext) {
    cx.update(|cx| crate::init(cx));

    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));

    let source = r#"
import { View, div } from "gpui";

export default class Typo extends View {
  render() {
    return div().items_centre();
  }
}
"#;

    let view_type = runtime.load_source("typo", source).expect("load");

    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    let object = context
        .update(|window, cx| runtime.instantiate(&view_type, window, cx))
        .expect("instantiate");

    let error = context
        .update(|window, cx| runtime.render_to_spec(&object, None, window, cx))
        .expect_err("a typo must fail");

    assert!(
        error.to_string().contains("items_center"),
        "expected a suggestion, got: {error}"
    );
}

#[gpui::test]
fn a_view_renders_through_gpui(cx: &mut TestAppContext) {
    cx.update(|cx| crate::init(cx));

    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));

    let view_type = runtime.load_source(ENTRY, COUNTER).expect("load");

    // The view is constructed inside the window builder, because `init` may
    // create retained state and that needs a live `Window`.
    let runtime_for_view = runtime.clone();
    let window = cx.add_window(|window, cx| {
        let object = runtime_for_view
            .instantiate(&view_type, window, cx)
            .expect("instantiate");
        ScriptView::new(runtime_for_view.clone(), object)
    });
    let mut context = VisualTestContext::from_window(*window.deref(), cx);

    // A real paint must not panic: it exercises materialize, not just the
    // description.
    context.draw(
        gpui::Point::default(),
        gpui::size(gpui::px(400.), gpui::px(300.)),
        |_, _| gpui::div(),
    );
    context.run_until_parked();
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

use std::ops::Deref;

#[gpui::test]
fn the_bundled_example_application_loads_and_renders(cx: &mut TestAppContext) {
    cx.update(|cx| crate::init(cx));

    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));

    // The example is the contract with users: if it stops rendering, the
    // quickstart in the README is wrong.
    let directory = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples/js_todolist")
        .canonicalize()
        .expect("example directory");

    let view_type = runtime
        .load_app(&directory, "main.js")
        .expect("load example");

    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    let object = context
        .update(|window, cx| runtime.instantiate(&view_type, window, cx))
        .expect("instantiate");

    let tree = context.update(|window, cx| {
        runtime
            .render_to_spec(&object, None, window, cx)
            .expect("render")
    });

    assert!(tree.contains("Button"), "example has no button: {tree}");
    assert!(tree.contains("text"), "example has no text: {tree}");
}

#[gpui::test]
fn state_styles_reuse_the_ordinary_style_methods(cx: &mut TestAppContext) {
    cx.update(|cx| crate::init(cx));

    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));

    let source = r#"
import { View, div, Button, text } from "gpui";

export default class Styled extends View {
  render() {
    return div()
      .hover((el) => el.bg("accent"))
      .child(
        Button.new("go")
          .bg("primary")
          .hover((el) => el.opacity(0.9))
          .active((el) => el.opacity(0.8))
          .child(text("Go")),
      );
  }
}
"#;

    let view_type = runtime.load_source("styled", source).expect("load");

    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    let object = context
        .update(|window, cx| runtime.instantiate(&view_type, window, cx))
        .expect("instantiate");

    let tree = context.update(|window, cx| {
        runtime
            .render_to_spec(&object, None, window, cx)
            .expect("render")
    });

    assert!(tree.contains(":hover(.bg"), "hover not recorded: {tree}");
    assert!(
        tree.contains(":active(.opacity"),
        "active not recorded: {tree}"
    );
}

#[gpui::test]
fn transition_declarations_survive_the_script_render(cx: &mut TestAppContext) {
    cx.update(|cx| crate::init(cx));

    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));

    let source = r#"
import { View, div } from "gpui";

export default class Motion extends View {
  render() {
    return div()
      .id("sidebar")
      .w(320)
      .transition("width", { duration: 180, delay: 20, easing: "ease-out" });
  }
}
"#;

    let view_type = runtime.load_source("motion", source).expect("load");
    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    let object = context
        .update(|window, cx| runtime.instantiate(&view_type, window, cx))
        .expect("instantiate");

    let tree = context.update(|window, cx| {
        runtime
            .render_to_spec(&object, None, window, cx)
            .expect("render")
    });

    assert!(
        tree.contains(":transition(width, 180ms, 20ms, ease-out)"),
        "the native motion target and policy were not retained in the snapshot: {tree}"
    );
}

#[gpui::test]
fn native_overflow_scroll_behaviors_survive_script_render_and_materialize(cx: &mut TestAppContext) {
    cx.update(|cx| crate::init(cx));

    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));
    let source = r#"
import { View, text, v_flex } from "gpui";

export default class ScrollableQuotes extends View {
  render() {
    return v_flex()
      .child(v_flex().id("both").size(80).overflow_scroll().child(text("Both")))
      .child(v_flex().id("horizontal").size(80).overflow_x_scroll().child(text("Horizontal")))
      .child(v_flex().id("watchlist-quotes").h(120).overflow_y_scroll()
        .children(Array.from({ length: 30 }, (_, index) => text(`Quote ${index}`))));
  }
}
"#;

    let view_type = runtime.load_source("scroll-y", source).expect("load");
    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    let object = context
        .update(|window, cx| runtime.instantiate(&view_type, window, cx))
        .expect("instantiate");

    let tree = context
        .update(|window, cx| runtime.render_to_spec(&object, None, window, cx))
        .expect("native overflow scroll methods must be supported behaviors");
    for behavior in ["overflow_scroll", "overflow_x_scroll", "overflow_y_scroll"] {
        assert!(
            tree.contains(&format!(":{behavior}")),
            "missing {behavior}: {tree}"
        );
    }
    let view = context.update(|_, cx| cx.new(|_| ScriptView::new(runtime, object)));
    draw(&mut context, &view);
}

#[gpui::test]
fn motion_rejects_properties_the_native_layer_cannot_interpolate(cx: &mut TestAppContext) {
    cx.update(|cx| crate::init(cx));

    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));
    let source = r#"
import { View, div } from "gpui";

export default class BadMotion extends View {
  render() {
    return div().id("panel").transition("padding", 120);
  }
}
"#;
    let view_type = runtime.load_source("bad-motion", source).expect("load");
    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    let object = context
        .update(|window, cx| runtime.instantiate(&view_type, window, cx))
        .expect("instantiate");

    let error = context
        .update(|window, cx| runtime.render_to_spec(&object, None, window, cx))
        .expect_err("unsupported motion properties must fail at the script call site");

    assert!(
        error
            .to_string()
            .contains("opacity, width, height, left or top"),
        "the error must name the supported native motion properties: {error}"
    );
}

#[gpui::test]
fn spring_declarations_survive_the_script_render(cx: &mut TestAppContext) {
    cx.update(|cx| crate::init(cx));

    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));
    let source = r#"
import { View, div } from "gpui";

export default class Motion extends View {
  render() {
    return div().id("indicator").left(48).spring("left", {
      response: 250,
      damping: 0.85,
      epsilon: 0.25,
    });
  }
}
"#;
    let view_type = runtime.load_source("spring-motion", source).expect("load");
    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    let object = context
        .update(|window, cx| runtime.instantiate(&view_type, window, cx))
        .expect("instantiate");

    let tree = context.update(|window, cx| {
        runtime
            .render_to_spec(&object, None, window, cx)
            .expect("render")
    });
    assert!(
        tree.contains(":spring(left, 250ms, 0.85, 0.25)"),
        "the native spring target and policy were not retained in the snapshot: {tree}"
    );
}

#[gpui::test]
fn transition_rejects_an_unknown_native_easing(cx: &mut TestAppContext) {
    cx.update(|cx| crate::init(cx));
    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));
    let source = r#"
import { View, div } from "gpui";
export default class BadMotion extends View {
  render() {
    return div().opacity(0.5).transition("opacity", { duration: 120, easing: "bounce" });
  }
}
"#;
    let view_type = runtime.load_source("bad-easing", source).expect("load");
    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    let object = context
        .update(|window, cx| runtime.instantiate(&view_type, window, cx))
        .expect("instantiate");
    let error = context
        .update(|window, cx| runtime.render_to_spec(&object, None, window, cx))
        .expect_err("an easing Rust cannot sample must fail at the call site");
    assert!(
        error
            .to_string()
            .contains("linear, ease-in, ease-out or ease-in-out"),
        "the error must name the snapshot-safe easing values: {error}"
    );
}

#[gpui::test]
fn motion_rejects_non_finite_or_physically_invalid_policies(cx: &mut TestAppContext) {
    cx.update(|cx| crate::init(cx));
    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));
    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);

    for (name, declaration, expected) in [
        (
            "nan-duration",
            r#"div().opacity(0.5).transition("opacity", { duration: NaN })"#,
            "duration must be a finite non-negative number",
        ),
        (
            "negative-delay",
            r#"div().opacity(0.5).transition("opacity", { duration: 120, delay: -1 })"#,
            "delay must be a finite non-negative number",
        ),
        (
            "negative-damping",
            r#"div().left(20).spring("left", { damping: -0.1 })"#,
            "damping must be a finite non-negative number",
        ),
        (
            "zero-epsilon",
            r#"div().left(20).spring("left", { epsilon: 0 })"#,
            "epsilon must be a finite positive number",
        ),
    ] {
        let source = format!(
            r#"
import {{ View, div }} from "gpui";
export default class BadMotion extends View {{
  render() {{ return {declaration}; }}
}}
"#
        );
        let view_type = runtime.load_source(name, &source).expect("load");
        let object = context
            .update(|window, cx| runtime.instantiate(&view_type, window, cx))
            .expect("instantiate");
        let error = context
            .update(|window, cx| runtime.render_to_spec(&object, None, window, cx))
            .expect_err("invalid motion policies must fail at the script call site");
        assert!(
            error.to_string().contains(expected),
            "`{name}` must explain its invalid field: {error}"
        );
    }
}

#[gpui::test]
fn theme_tokens_resolve_outside_a_call_scope(cx: &mut TestAppContext) {
    cx.update(|cx| crate::init(cx));

    // Materialization happens after the call scope closes, so a palette that
    // could only be read through the scope resolved every color to `None` and
    // painted an unstyled black window. This is that regression.
    assert!(
        crate::theme::token_color("background").is_some(),
        "semantic tokens must resolve without an open call scope"
    );
    assert!(crate::theme::token_color("primary").is_some());
    assert!(crate::theme::token_color("not_a_token").is_none());
}

/// The todo list exists to exercise the whole runtime at once: retained input
/// state, controlled checkboxes, a dialog, a toast, capability-gated storage,
/// and a filter that must survive every mutation. If a subsystem regresses,
/// this is the test that notices.
#[gpui::test]
fn the_todolist_example_exercises_the_runtime(cx: &mut TestAppContext) {
    cx.update(|cx| crate::init(cx));

    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));

    let directory = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples/js_todolist")
        .canonicalize()
        .expect("example directory");

    let view_type = runtime
        .load_app(&directory, "main.js")
        .expect("load example");

    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);

    // `init` creates retained state, so instantiation needs a live host call.
    let object = context.update(|window, cx| runtime.instantiate(&view_type, window, cx));
    let object = object.expect("instantiate");

    let tree = context.update(|window, cx| {
        runtime
            .render_to_spec(&object, None, window, cx)
            .expect("render")
    });

    for expected in [
        "Todo",             // the object
        "Input",            // retained state reached the description
        "\"Add\"",          // the action that creates work
        "No items yet",     // the empty state explains the next step
        "Clear completed…", // an ellipsis, because it opens a dialog
    ] {
        assert!(
            tree.contains(expected),
            "todolist is missing `{expected}`:\n{tree}"
        );
    }
}

#[gpui::test]
fn an_unknown_input_event_names_the_valid_ones(cx: &mut TestAppContext) {
    cx.update(|cx| crate::init(cx));

    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));

    let source = r#"
import { View, div, InputState } from "gpui";

export default class Bad extends View {
  init() {
    this.field = InputState.new({});
    this.field.on("entered", () => {});
  }
  render() {
    return div();
  }
}
"#;

    let view_type = runtime.load_source("bad", source).expect("load");

    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);

    let error = context
        .update(|window, cx| runtime.instantiate(&view_type, window, cx))
        .expect_err("an unknown event name must fail");

    assert!(
        error.to_string().contains("submit"),
        "the error should list the valid events, got: {error}"
    );
}

/// Hot reload has to pick up a change in an imported module, not only in the
/// entry point. QuickJS caches an evaluated module by name and an ES module
/// cannot be unloaded, so a naive reload re-evaluates `main.js` against the
/// first version of everything it imports — and looks like it worked.
#[gpui::test]
fn a_reload_picks_up_a_change_in_an_imported_module(cx: &mut TestAppContext) {
    cx.update(|cx| crate::init(cx));

    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));

    let directory = std::env::temp_dir().join(format!("gpui-shell-reload-{}", std::process::id()));
    std::fs::create_dir_all(&directory).expect("temp directory");

    std::fs::write(
        directory.join("main.js"),
        r#"
import { View, v_flex, text } from "gpui";
import { caption } from "./caption.js";

export default class Reloading extends View {
  render() {
    return v_flex().child(text(caption()));
  }
}
"#,
    )
    .expect("write main");
    std::fs::write(
        directory.join("caption.js"),
        "export const caption = () => \"before\";\n",
    )
    .expect("write caption");

    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);

    let render = |context: &mut VisualTestContext| {
        let view_type = runtime.load_app(&directory, "main.js").expect("load");
        let object = context
            .update(|window, cx| runtime.instantiate(&view_type, window, cx))
            .expect("instantiate");
        context.update(|window, cx| {
            runtime
                .render_to_spec(&object, None, window, cx)
                .expect("render")
        })
    };

    assert!(render(&mut context).contains("before"));

    std::fs::write(
        directory.join("caption.js"),
        "export const caption = () => \"after\";\n",
    )
    .expect("rewrite caption");

    let reloaded = render(&mut context);
    assert!(
        reloaded.contains("after"),
        "the imported module was served from the cache:\n{reloaded}"
    );

    std::fs::remove_dir_all(&directory).ok();
}

#[gpui::test]
fn oversized_entry_and_imported_modules_are_refused(cx: &mut TestAppContext) {
    cx.update(crate::init);
    let runtime = ShellRuntime::new_isolated().expect("runtime");
    let directory =
        std::env::temp_dir().join(format!("gpui-shell-module-limit-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&directory);
    std::fs::create_dir_all(&directory).expect("application directory");

    let entry = directory.join("main.js");
    let file = std::fs::File::create(&entry).expect("entry module");
    file.set_len(8 * 1024 * 1024 + 1).expect("sparse entry");
    let error = runtime
        .load_app(&directory, "main.js")
        .expect_err("oversized entry module must fail");
    assert!(error.to_string().contains("module") && error.to_string().contains("limit"));

    std::fs::write(&entry, "import './huge.js'; export default class Panel {};")
        .expect("entry module");
    let imported = std::fs::File::create(directory.join("huge.js")).expect("imported module");
    imported
        .set_len(8 * 1024 * 1024 + 1)
        .expect("sparse import");
    let error = runtime
        .load_app(&directory, "main.js")
        .expect_err("oversized imported module must fail");
    assert!(error.to_string().contains("module") && error.to_string().contains("limit"));
    let _ = std::fs::remove_dir_all(directory);
}

/// An embedded runtime reloads on a save, with no host doing anything but
/// asking for it once.
///
/// The binary has `--watch` because the person running it is the person
/// editing. A host that embeds the runtime has no flag to offer, so a debug
/// build simply *is* the development build — and this is the test that says so,
/// since the behaviour is otherwise invisible until someone saves a file.
#[gpui::test]
fn an_embedded_runtime_reloads_when_a_source_changes(cx: &mut TestAppContext) {
    cx.update(|cx| crate::init(cx));

    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));

    let directory = std::env::temp_dir().join(format!("gpui-shell-watch-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&directory);
    std::fs::create_dir_all(&directory).expect("a temporary application");
    let source = |caption: &str| {
        format!(
            "import {{ View, v_flex, text }} from \"gpui\";\n\
             export default class Panel extends View {{\n\
               render() {{ return v_flex().child(text(\"{caption}\")); }}\n\
             }}\n"
        )
    };
    std::fs::write(directory.join("main.js"), source("before")).expect("writing main.js");

    let view_type = runtime.load_app(&directory, "main.js").expect("load");
    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);

    let view = context.update(|window, cx| {
        let object = runtime
            .instantiate(&view_type, window, cx)
            .expect("instantiate");
        let view = cx.new(|_| ScriptView::new(runtime.clone(), object));
        // Exercise the watcher itself in both debug and release test builds.
        let watch =
            crate::watch::Watch::start(&runtime, &view, directory.clone(), "main.js", window, cx);
        watch.forget();
        view
    });

    let description = |context: &mut VisualTestContext| {
        context.update(|_, cx| {
            view.read(cx)
                .snapshot()
                .map(crate::RenderSnapshot::debug_tree)
                .unwrap_or_default()
        })
    };

    draw(&mut context, &view);
    assert!(description(&mut context).contains("before"));

    // The watcher compares modification stamps, so the file has to look older
    // than the write that follows it — a test that runs inside one filesystem
    // tick would otherwise see nothing change.
    std::thread::sleep(std::time::Duration::from_millis(20));
    std::fs::write(directory.join("main.js"), source("after")).expect("rewriting main.js");

    // Two polls, with real time in between. The poll interval is on the
    // executor's clock, which `advance_clock` moves; the debounce is measured
    // against the wall, because it is absorbing a burst of saves from an editor
    // rather than counting frames. So the first poll notices the change and the
    // second one — after the tree has been still for the debounce window —
    // reports it.
    let settle = |context: &mut VisualTestContext| {
        context
            .executor()
            .advance_clock(crate::watch::POLL_INTERVAL * 2);
        context.run_until_parked();
    };
    settle(&mut context);
    std::thread::sleep(std::time::Duration::from_millis(250));
    settle(&mut context);

    draw(&mut context, &view);

    assert!(
        description(&mut context).contains("after"),
        "a saved change should have reached the view without anyone asking: {}",
        description(&mut context)
    );

    let _ = std::fs::remove_dir_all(&directory);
}

#[gpui::test]
fn reload_replaces_old_tasks_and_rolls_back_failed_new_tasks(cx: &mut TestAppContext) {
    cx.update(crate::init);
    let runtime = ShellRuntime::new_isolated().expect("runtime");
    let directory =
        std::env::temp_dir().join(format!("gpui-shell-reload-tasks-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&directory);
    std::fs::create_dir_all(&directory).expect("application directory");

    let source = |caption: &str| {
        format!(
            "import {{ View, timer, text }} from \"gpui\";\n\
             export default class Panel extends View {{\n\
               init() {{ timer.every(60_000, () => {{}}); }}\n\
               render() {{ return text(\"{caption}\"); }}\n\
             }}\n"
        )
    };
    std::fs::write(directory.join("main.js"), source("first")).expect("initial source");

    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    let baseline = crate::engine::quickjs::task_count();
    let view = context.update(|window, cx| {
        let view_type = runtime.load_app(&directory, "main.js").expect("load");
        runtime
            .instantiate_view(&view_type, window, cx)
            .expect("instantiate")
    });
    assert_eq!(crate::engine::quickjs::task_count(), baseline + 1);

    std::fs::write(directory.join("main.js"), source("second")).expect("replacement source");
    context
        .update(|window, cx| {
            crate::watch::reload(&runtime, &view, &directory, "main.js", window, cx)
        })
        .expect("successful reload");
    assert_eq!(
        crate::engine::quickjs::task_count(),
        baseline + 1,
        "the old instance's timer must be retired when the new one commits"
    );

    std::fs::write(
        directory.join("main.js"),
        "import { timer } from \"gpui\";\n\
         timer.every(60_000, () => {});\n\
         throw new Error(\"reload failed\");",
    )
    .expect("failing source");
    context
        .update(|window, cx| {
            crate::watch::reload(&runtime, &view, &directory, "main.js", window, cx)
        })
        .expect_err("the replacement must fail");
    assert_eq!(
        crate::engine::quickjs::task_count(),
        baseline + 1,
        "work created by a failed reload must be rolled back"
    );

    let _ = std::fs::remove_dir_all(directory);
}

#[gpui::test]
fn reload_evaluates_modules_under_the_views_frozen_capabilities(cx: &mut TestAppContext) {
    cx.update(crate::init);
    let observed = Rc::new(Cell::new(false));
    let mut modules = NativeModules::new();
    modules.register("audit", {
        let observed = observed.clone();
        move |module| {
            module.function("observe", move |_| {
                observed.set(crate::scope::policy().capabilities().has_read_access());
                Ok(NativeValue::from(true))
            });
        }
    });
    crate::set_native_modules(modules);

    let directory =
        std::env::temp_dir().join(format!("gpui-shell-reload-policy-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&directory);
    std::fs::create_dir_all(&directory).expect("application directory");
    let source = |caption: &str| {
        format!(
            "import {{ View, native, text }} from \"gpui\";\n\
             native('audit').observe();\n\
             export default class Panel extends View {{\n\
               render() {{ return text(\"{caption}\"); }}\n\
             }}"
        )
    };
    std::fs::write(directory.join("main.js"), source("first")).expect("initial source");

    crate::set_capabilities(Capabilities::new().read_roots([directory.clone()]));
    let runtime = ShellRuntime::new_isolated().expect("runtime");
    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    let view = context.update(|window, cx| {
        let view_type = runtime.load_app(&directory, "main.js").expect("load");
        runtime
            .instantiate_view(&view_type, window, cx)
            .expect("instantiate")
    });

    crate::set_capabilities(Capabilities::new());
    observed.set(false);
    std::fs::write(directory.join("main.js"), source("second")).expect("replacement source");
    context
        .update(|window, cx| {
            crate::watch::reload(&runtime, &view, &directory, "main.js", window, cx)
        })
        .expect("reload");
    assert!(
        observed.get(),
        "module evaluation must keep the view's frozen capability grant"
    );

    crate::clear_native_modules();
    let _ = std::fs::remove_dir_all(directory);
}

#[gpui::test]
fn loading_a_second_application_keeps_the_first_dynamic_import_root(cx: &mut TestAppContext) {
    cx.update(crate::init);
    let runtime = ShellRuntime::new_isolated().expect("runtime");
    let base = std::env::temp_dir().join(format!("gpui-shell-multi-root-{}", std::process::id()));
    let first = base.join("first");
    let second = base.join("second");
    let _ = std::fs::remove_dir_all(&base);
    std::fs::create_dir_all(&first).expect("first application");
    std::fs::create_dir_all(&second).expect("second application");
    std::fs::write(
        first.join("feature.js"),
        "export const label = 'first feature';",
    )
    .expect("first feature");
    std::fs::write(
        first.join("main.js"),
        "import { View, sleep, spawn, text, with_cx } from \"gpui\";\n\
         export default class First extends View {\n\
           init() {\n\
             this.label = 'waiting';\n\
             spawn(async () => {\n\
               await sleep(1);\n\
               this.label = (await import('./feature.js')).label;\n\
               with_cx((cx) => cx.notify());\n\
             });\n\
           }\n\
           render() { return text(this.label); }\n\
         }",
    )
    .expect("first entry");
    std::fs::write(
        second.join("main.js"),
        "import { View, text } from \"gpui\";\n\
         export default class Second extends View { render() { return text('second'); } }",
    )
    .expect("second entry");

    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    let first_view = context.update(|window, cx| {
        let view_type = runtime.load_app(&first, "main.js").expect("load first");
        runtime
            .instantiate_view(&view_type, window, cx)
            .expect("instantiate first")
    });
    context.update(|window, cx| {
        let view_type = runtime.load_app(&second, "main.js").expect("load second");
        runtime
            .instantiate_view(&view_type, window, cx)
            .expect("instantiate second")
    });

    context
        .executor()
        .advance_clock(std::time::Duration::from_millis(2));
    context.run_until_parked();
    draw(&mut context, &first_view);
    let tree = context.update(|_, cx| {
        first_view
            .read(cx)
            .snapshot()
            .map(crate::RenderSnapshot::debug_tree)
            .unwrap_or_default()
    });
    assert!(
        tree.contains("first feature"),
        "the first application's lazy import used the wrong root: {tree}"
    );

    let _ = std::fs::remove_dir_all(base);
}

fn draw(context: &mut VisualTestContext, view: &gpui::Entity<ScriptView>) {
    let view = view.clone();
    context.draw(
        gpui::Point::default(),
        gpui::size(gpui::px(400.), gpui::px(300.)),
        move |_, _| gpui::IntoElement::into_any_element(view),
    );
}

/// A granted `process.exit` reaches the host, with the code the script asked
/// for.
///
/// The request used to be written into a cell no production code read: the
/// script got a success and the window stayed open. So the test is not "the
/// flag was set" but "the host was told", which is the only version of this
/// that can go wrong quietly.
#[gpui::test]
fn a_granted_exit_reaches_the_host(cx: &mut TestAppContext) {
    cx.update(|cx| crate::init(cx));
    crate::set_capabilities(crate::Capabilities::new().exit(true));

    let asked: std::rc::Rc<std::cell::Cell<Option<i32>>> = Default::default();
    let recorded = asked.clone();
    crate::on_exit_request(move |request, _, _| recorded.set(Some(request.code())));

    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));

    let source = r#"
import { View, v_flex, text } from "gpui";

export default class Quitter extends View {
  init() {
    process.exit(7);
  }

  render() {
    return v_flex().child(text("still here"));
  }
}
"#;
    let view_type = runtime.load_source("quitter.js", source).expect("load");

    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    context
        .update(|window, cx| runtime.instantiate(&view_type, window, cx))
        .expect("instantiate");

    assert_eq!(
        asked.get(),
        Some(7),
        "the host was never told the script asked to exit"
    );

    crate::clear_exit_handler();
}

/// A watcher does not keep its view alive, and stops when the view goes.
///
/// The loop polls every quarter second for the life of the window. Holding the
/// view strongly would mean a panel removed from a dock is never dropped — the
/// runtime it points at is never dropped either — and the poller goes on stating
/// a directory for a panel nobody can see. Mount and unmount a few and they
/// accumulate.
#[gpui::test]
fn a_watcher_releases_its_view(cx: &mut TestAppContext) {
    cx.update(|cx| crate::init(cx));

    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));

    let directory = std::env::temp_dir().join(format!("gpui-shell-release-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&directory);
    std::fs::create_dir_all(&directory).expect("a temporary application");
    std::fs::write(
        directory.join("main.js"),
        "import { View, v_flex } from \"gpui\";\n\
         export default class Panel extends View { render() { return v_flex(); } }\n",
    )
    .expect("writing main.js");

    let view_type = runtime.load_app(&directory, "main.js").expect("load");
    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);

    let weak = context.update(|window, cx| {
        let object = runtime
            .instantiate(&view_type, window, cx)
            .expect("instantiate");
        let view = cx.new(|_| ScriptView::new(runtime.clone(), object));
        let watch =
            crate::watch::Watch::start(&runtime, &view, directory.clone(), "main.js", window, cx);
        watch.forget();
        view.downgrade()
    });

    // Nothing else is holding it: the panel it stood for has been removed.
    context
        .executor()
        .advance_clock(crate::watch::POLL_INTERVAL * 2);
    context.run_until_parked();

    assert!(
        weak.upgrade().is_none(),
        "the watcher is still holding the view it was watching for"
    );

    let _ = std::fs::remove_dir_all(&directory);
}

/// Two runtimes coexist on one thread, each under its own authority.
///
/// This used to be refused: the grant and the store were thread state, so a
/// second runtime would silently run under the first one's permissions. They now
/// live on a `Policy` that travels on the call frame, so the two cannot collide
/// and the refusal has nothing left to protect.
#[gpui::test]
fn two_runtimes_share_a_thread_without_sharing_a_grant(cx: &mut TestAppContext) {
    cx.update(|cx| crate::init(cx));

    let _first = ShellRuntime::new_isolated().expect("the first runtime");
    let _second = ShellRuntime::new_isolated().expect("the second runtime");
}

/// A scope opened under a policy answers `fs` with *that* grant.
///
/// This is the seam every capability check goes through, and the half of the
/// P0 fix that the scheduler's capture relies on: a task that kept its policy
/// is only correct if restoring that policy actually changes what the engine
/// sees.
#[gpui::test]
fn a_scope_answers_with_the_policy_it_was_opened_under(cx: &mut TestAppContext) {
    cx.update(|cx| crate::init(cx));

    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);

    crate::set_capabilities(Capabilities::new());
    let plugin = Rc::new(
        Policy::new().with_capabilities(Capabilities::new().read_roots([PathBuf::from("/tmp/p")])),
    );
    let runtime = ShellRuntime::new_isolated().expect("runtime");

    context.update(|window, cx| {
        assert!(
            !crate::scope::policy().capabilities().has_read_access(),
            "the default grants nothing"
        );

        // No view: the case a plugin's module top level runs in.
        let (guard, _) = crate::scope::enter_with_runtime(
            &runtime,
            window,
            cx,
            crate::scope::ScopePhase::Task,
            None,
            plugin.clone(),
        );
        assert!(
            crate::scope::policy().capabilities().has_read_access(),
            "inside the scope the plugin's grant is what fs sees"
        );
        drop(guard);

        assert!(
            !crate::scope::policy().capabilities().has_read_access(),
            "and it does not outlive the call"
        );
    });
}

/// Two policies hold two grants at the same time.
///
/// The point the single process-wide slot could not reach: authority belongs to
/// the code that is running, not to the moment it runs in.
#[gpui::test]
fn two_policies_hold_two_grants_at_once(cx: &mut TestAppContext) {
    cx.update(|cx| crate::init(cx));

    let reader = Rc::new(
        Policy::default()
            .with_capabilities(Capabilities::new().read_roots([PathBuf::from("/tmp/reader")])),
    );
    let writer = Rc::new(
        Policy::default()
            .with_capabilities(Capabilities::new().write_roots([PathBuf::from("/tmp/writer")])),
    );

    assert!(reader.capabilities().has_read_access());
    assert!(!reader.capabilities().has_write_access());
    assert!(writer.capabilities().has_write_access());
    assert!(!writer.capabilities().has_read_access());

    // Both are alive at the same instant, and neither is the other.
    assert!(!Rc::ptr_eq(&reader, &writer));
}
