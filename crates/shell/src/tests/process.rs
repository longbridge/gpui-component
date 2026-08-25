//! The JavaScript-facing process adapter.
//!
//! Unit tests in `crate::process` exercise the pipe, timeout and kill mechanics.
//! These tests cross the public script boundary: a real module calls
//! `process.run`, awaits its promise, and publishes what JavaScript observed in
//! a render snapshot.

#![cfg(unix)]

use std::ops::Deref;

use gpui::{Entity, IntoElement as _, TestAppContext, VisualTestContext};

use crate::{Capabilities, ExecuteGrant, ScriptView, ShellRuntime};

const OUTPUT_PROBE: &str = r#"
import { View, v_flex, text, spawn, process, with_cx } from "gpui";

export default class Probe extends View {
  init() {
    this.state = "pending";
    spawn(async (cx) => {
      try {
        const output = await process.run("/bin/sh", [
          "-c",
          "printf out; printf err >&2; exit 7",
        ]);
        this.state = `${output.code}|${output.stdout}|${output.stderr}`;
      } catch (error) {
        this.state = `rejected:${error.message}`;
      }
      with_cx((cx) => cx.notify());
    });
  }

  render() {
    return v_flex().child(text(this.state));
  }
}
"#;

const FAILURE_PROBE: &str = r#"
import { View, v_flex, text, spawn, process, with_cx } from "gpui";

export default class Probe extends View {
  init() {
    this.state = "pending";
    spawn(async (cx) => {
      try {
        await process.run("/gpui-shell-command-that-does-not-exist");
        this.state = "unexpectedly resolved";
      } catch (error) {
        this.state = `rejected:${error.message}`;
      }
      with_cx((cx) => cx.notify());
    });
  }

  render() {
    return v_flex().child(text(this.state));
  }
}
"#;

const OUTPUT_LIMIT_PROBE: &str = r#"
import { View, v_flex, text, spawn, process, with_cx } from "gpui";

export default class Probe extends View {
  init() {
    this.state = "pending";
    spawn(async (cx) => {
      try {
        await process.run("/bin/sh", ["-c", "yes x | head -c 8388609"]);
        this.state = "unexpectedly resolved";
      } catch (error) {
        this.state = `rejected:${error.message}`;
      }
      with_cx((cx) => cx.notify());
    });
  }

  render() {
    return v_flex().child(text(this.state));
  }
}
"#;

#[gpui::test]
fn process_promise_exposes_status_and_both_streams(cx: &mut TestAppContext) {
    let (_runtime, object, mut context) = probe(cx, "/bin/sh", "process-output.js", OUTPUT_PROBE);

    draw(&mut context, &object);
    assert!(snapshot_text(&mut context, &object).contains("pending"));
    context.run_until_parked();
    draw(&mut context, &object);

    let settled = snapshot_text(&mut context, &object);
    assert!(
        settled.contains("7|out|err"),
        "JavaScript did not observe the complete process result: {settled}"
    );
}

#[gpui::test]
fn process_start_failure_rejects_the_javascript_promise(cx: &mut TestAppContext) {
    let command = "/gpui-shell-command-that-does-not-exist";
    let (_runtime, object, mut context) = probe(cx, command, "process-failure.js", FAILURE_PROBE);

    context.run_until_parked();
    draw(&mut context, &object);

    let settled = snapshot_text(&mut context, &object);
    assert!(
        settled.contains("rejected:"),
        "promise did not reject: {settled}"
    );
    assert!(
        settled.contains(command) && settled.contains("failed"),
        "the rejection did not identify the failed command: {settled}"
    );
}

#[gpui::test]
fn process_output_limit_rejects_the_javascript_promise(cx: &mut TestAppContext) {
    let (_runtime, object, mut context) =
        probe(cx, "/bin/sh", "process-output-limit.js", OUTPUT_LIMIT_PROBE);

    context.run_until_parked();
    draw(&mut context, &object);

    let settled = snapshot_text(&mut context, &object);
    assert!(
        settled.contains("rejected:") && settled.contains("stdout") && settled.contains("exceeded"),
        "JavaScript did not observe the bounded-output rejection: {settled}"
    );
}

fn probe(
    cx: &mut TestAppContext,
    command: &str,
    name: &str,
    source: &str,
) -> (
    std::rc::Rc<ShellRuntime>,
    Entity<ScriptView>,
    VisualTestContext,
) {
    cx.update(crate::init);
    crate::set_capabilities(
        Capabilities::new().execute(ExecuteGrant::Allowed(vec![command.to_owned()])),
    );

    let runtime = ShellRuntime::new().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));
    let view_type = runtime.load_source(name, source).expect("load script");

    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    let object = context
        .update(|window, cx| runtime.instantiate_view(&view_type, window, cx))
        .expect("instantiate script view");
    (runtime, object, context)
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
