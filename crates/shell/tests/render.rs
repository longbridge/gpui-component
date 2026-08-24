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
    let object = runtime.instantiate(&view_type).expect("instantiate");

    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);

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
    let object = runtime.instantiate(&view_type).expect("instantiate");

    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);

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
    let object = runtime.instantiate(&view_type).expect("instantiate");

    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);

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
    let object = runtime.instantiate(&view_type).expect("instantiate");

    let runtime_for_view = runtime.clone();
    let window = cx.add_window(|_, cx| {
        let _ = cx;
        ScriptView::new(runtime_for_view, object)
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
        .join("../../examples/js_checklist")
        .canonicalize()
        .expect("example directory");

    let view_type = runtime.load_app(&directory).expect("load example");
    let object = runtime.instantiate(&view_type).expect("instantiate");

    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);

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
    let object = runtime.instantiate(&view_type).expect("instantiate");

    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);

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
