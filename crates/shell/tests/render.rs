//! End-to-end tests for the script render protocol.
//!
//! These exercise the whole path — VM, method dispatch, spec arena, event
//! callbacks — without painting a frame, because the element description is
//! plain data. They run against whichever engine is enabled, which is what
//! keeps the fallback engine honest.

use gpui::{TestAppContext, VisualTestContext};
use gpui_shell::{ScriptView, ShellRuntime};

#[cfg(feature = "quickjs")]
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

#[cfg(not(feature = "quickjs"))]
const COUNTER: &str = r#"
local gpui = require("gpui")
local Counter = gpui.view("Counter")

function Counter:init()
  self.count = 0
end

function Counter:render(cx)
  return gpui.v_flex()
    :size_full():items_center():gap_2():p(16):bg("background")
    :child(gpui.text("Count: " .. self.count):text_color("foreground"))
    :child(
      gpui.Button.new("increment")
        :px(12):py(6):rounded(6):bg("primary")
        :on_click(function(event, cx)
          self.count = self.count + 1
          cx:notify()
        end)
        :child(gpui.text("Increment"):text_color("primary_foreground"))
    )
end

return Counter
"#;

/// The entry name only affects diagnostics, but each engine has its own
/// convention and the tests should read the way real code does.
#[cfg(feature = "quickjs")]
const ENTRY: &str = "counter.js";
#[cfg(not(feature = "quickjs"))]
const ENTRY: &str = "counter.lua";

#[gpui::test]
fn a_script_view_produces_an_element_description(cx: &mut TestAppContext) {
    cx.update(|cx| gpui_shell::init(cx));

    let runtime = ShellRuntime::new().expect("runtime");
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
fn an_element_cannot_be_added_to_two_parents(cx: &mut TestAppContext) {
    cx.update(|cx| gpui_shell::init(cx));

    let runtime = ShellRuntime::new().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));

    #[cfg(feature = "quickjs")]
    let source = r#"
import { View, v_flex, text } from "gpui";

export default class Broken extends View {
  render() {
    const shared = text("reused");
    return v_flex().child(shared).child(shared);
  }
}
"#;
    #[cfg(not(feature = "quickjs"))]
    let source = r#"
local gpui = require("gpui")
local Broken = gpui.view("Broken")

function Broken:render(cx)
  local shared = gpui.text("reused")
  return gpui.v_flex():child(shared):child(shared)
end

return Broken
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
    cx.update(|cx| gpui_shell::init(cx));

    let runtime = ShellRuntime::new().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));

    #[cfg(feature = "quickjs")]
    let source = r#"
import { View, div } from "gpui";

export default class Typo extends View {
  render() {
    return div().items_centre();
  }
}
"#;
    #[cfg(not(feature = "quickjs"))]
    let source = r#"
local gpui = require("gpui")
local Typo = gpui.view("Typo")

function Typo:render(cx)
  return gpui.div():items_centre()
end

return Typo
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
    cx.update(|cx| gpui_shell::init(cx));

    let runtime = ShellRuntime::new().expect("runtime");
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

#[cfg(feature = "quickjs")]
#[gpui::test]
fn the_bundled_example_application_loads_and_renders(cx: &mut TestAppContext) {
    cx.update(|cx| gpui_shell::init(cx));

    let runtime = ShellRuntime::new().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));

    // The example is the contract with users: if it stops rendering, the
    // quickstart in the README is wrong.
    let directory = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples/js_todolist")
        .canonicalize()
        .expect("example directory");

    let view_type = runtime.load_app(&directory).expect("load example");

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

#[cfg(feature = "quickjs")]
#[gpui::test]
fn state_styles_reuse_the_ordinary_style_methods(cx: &mut TestAppContext) {
    cx.update(|cx| gpui_shell::init(cx));

    let runtime = ShellRuntime::new().expect("runtime");
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
fn theme_tokens_resolve_outside_a_call_scope(cx: &mut TestAppContext) {
    cx.update(|cx| gpui_shell::init(cx));

    // Materialization happens after the call scope closes, so a palette that
    // could only be read through the scope resolved every color to `None` and
    // painted an unstyled black window. This is that regression.
    assert!(
        gpui_shell::theme::token_color("background").is_some(),
        "semantic tokens must resolve without an open call scope"
    );
    assert!(gpui_shell::theme::token_color("primary").is_some());
    assert!(gpui_shell::theme::token_color("not_a_token").is_none());
}

/// The todo list exists to exercise the whole runtime at once: retained input
/// state, controlled checkboxes, a dialog, a toast, capability-gated storage,
/// and a filter that must survive every mutation. If a subsystem regresses,
/// this is the test that notices.
#[cfg(feature = "quickjs")]
#[gpui::test]
fn the_todolist_example_exercises_the_runtime(cx: &mut TestAppContext) {
    cx.update(|cx| gpui_shell::init(cx));

    let runtime = ShellRuntime::new().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));

    let directory = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples/js_todolist")
        .canonicalize()
        .expect("example directory");

    let view_type = runtime.load_app(&directory).expect("load example");

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

#[cfg(feature = "quickjs")]
#[gpui::test]
fn an_unknown_input_event_names_the_valid_ones(cx: &mut TestAppContext) {
    cx.update(|cx| gpui_shell::init(cx));

    let runtime = ShellRuntime::new().expect("runtime");
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
