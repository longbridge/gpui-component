//! Script-level contracts for small host APIs.
//!
//! These deliberately render a real `ScriptView`: success means the API was
//! installed in the `gpui` module, ran inside a live host scope, and produced
//! state JavaScript could publish to a snapshot.

use std::ops::Deref;

use gpui::{Entity, IntoElement as _, TestAppContext, VisualTestContext};

use crate::{Capabilities, HostModules, HostValue, ScriptView, ShellRuntime};

const CLIPBOARD_PROBE: &str = r#"
import { div, View } from "gpui";
import { v_flex } from "gpui-base";

export default class Probe extends View {
  init(_props, cx) {
    cx.write_to_clipboard("written by JavaScript");
    this.value = cx.read_from_clipboard();
  }
  render() { return v_flex().child(this.value); }
}
"#;

const CANCELLED_TIMER_PROBE: &str = r#"
import { div, View } from "gpui";
import { v_flex } from "gpui-base";

export default class Probe extends View {
  init(_props, cx) {
    this.fired = false;
    this.timer = cx.timer.after(0, () => { this.fired = true; });
    this.timer.cancel();
  }
  render() {
    return v_flex().child(`${this.timer.is_done()}|${this.fired}`);
  }
}
"#;

const HOST_MODULE_PROBE: &str = r#"
import { View } from "gpui";
import { v_flex } from "gpui-base";
import { increment } from "calculator";

export default class Probe extends View {
  init() { this.answer = increment(41); }
  render(cx) { return v_flex().child(`answer:${this.answer}`); }
}
"#;

#[gpui::test]
fn javascript_round_trips_text_through_the_host_clipboard(cx: &mut TestAppContext) {
    let (runtime, mut context, view) = script_view(
        cx,
        Capabilities::new()
            .clipboard_read(true)
            .clipboard_write(true),
        "clipboard.js",
        CLIPBOARD_PROBE,
    );
    let _keep_runtime_alive = runtime;

    draw(&mut context, &view);
    let rendered = snapshot_text(&mut context, &view);
    assert!(
        rendered.contains("written by JavaScript"),
        "JavaScript did not read back the clipboard value it wrote: {rendered}"
    );
}

#[gpui::test]
fn cancelling_a_javascript_timer_prevents_its_callback(cx: &mut TestAppContext) {
    let (runtime, mut context, view) = script_view(
        cx,
        Capabilities::new(),
        "cancelled-cx.timer.js",
        CANCELLED_TIMER_PROBE,
    );
    let _keep_runtime_alive = runtime;

    context.run_until_parked();
    draw(&mut context, &view);
    let rendered = snapshot_text(&mut context, &view);
    assert!(
        rendered.contains("true|false"),
        "the cancelled timer either stayed live or fired: {rendered}"
    );
}

#[gpui::test]
fn javascript_imports_a_host_registered_module(cx: &mut TestAppContext) {
    cx.update(crate::init);
    let mut modules = HostModules::new();
    modules.register("calculator", |module| {
        module.function("increment", |arguments| {
            Ok(HostValue::from(arguments.number(0)? + 1.))
        });
    });
    crate::export_modules(modules).expect("the module names are free");
    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));
    let view_type = runtime
        .load_source("host_module.js", HOST_MODULE_PROBE)
        .expect("load script");
    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    let view = context
        .update(|window, cx| runtime.instantiate_view(&view_type, window, cx))
        .expect("instantiate script view");

    draw(&mut context, &view);
    let rendered = snapshot_text(&mut context, &view);
    assert!(
        rendered.contains("answer:42"),
        "the native argument/result bridge did not round-trip: {rendered}"
    );
}

fn script_view(
    cx: &mut TestAppContext,
    capabilities: Capabilities,
    name: &str,
    source: &str,
) -> (
    std::rc::Rc<ShellRuntime>,
    VisualTestContext,
    Entity<ScriptView>,
) {
    cx.update(crate::init);
    crate::set_capabilities(capabilities);
    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));
    let view_type = runtime.load_source(name, source).expect("load script");
    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    let view = context
        .update(|window, cx| runtime.instantiate_view(&view_type, window, cx))
        .expect("instantiate script view");
    (runtime, context, view)
}

fn draw(context: &mut VisualTestContext, view: &Entity<ScriptView>) {
    let view = view.clone();
    context.draw(
        gpui::Point::default(),
        gpui::size(gpui::px(400.), gpui::px(300.)),
        move |_, _| view.into_any_element(),
    );
}

fn snapshot_text(context: &mut VisualTestContext, view: &Entity<ScriptView>) -> String {
    context.update(|_, cx| {
        view.read(cx)
            .snapshot()
            .map(crate::RenderSnapshot::debug_tree)
            .unwrap_or_default()
    })
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
