//! The JavaScript-facing process adapter.
//!
//! Unit tests in `crate::process` exercise the pipe, timeout and kill mechanics.
//! These tests cross the public script boundary: a real module calls
//! `process.run`, awaits its promise, and publishes what JavaScript observed in
//! a render snapshot.

use std::ops::Deref;

use gpui::{Entity, IntoElement as _, TestAppContext, VisualTestContext};

#[cfg(unix)]
use crate::ExecuteGrant;
use crate::{Capabilities, ScriptView, ShellRuntime};

#[cfg(unix)]
const OUTPUT_PROBE: &str = r#"
import { div, View } from "gpui-kit";
import { v_flex } from "gpui-base";
import process from "process";

export default class Probe extends View {
  init(_props, cx) {
    this.state = "pending";
    cx.spawn(async (cx) => {
      try {
        const output = await process.run("/bin/sh", [
          "-c",
          "printf out; printf err >&2; exit 7",
        ]);
        this.state = `${output.code}|${output.stdout}|${output.stderr}`;
      } catch (error) {
        this.state = `rejected:${error.message}`;
      }
      cx.notify();
    });
  }

  render() {
    return v_flex().child(this.state);
  }
}
"#;

#[cfg(unix)]
const FAILURE_PROBE: &str = r#"
import { div, View } from "gpui-kit";
import { v_flex } from "gpui-base";
import process from "process";

export default class Probe extends View {
  init(_props, cx) {
    this.state = "pending";
    cx.spawn(async (cx) => {
      try {
        await process.run("/gpui-shell-command-that-does-not-exist");
        this.state = "unexpectedly resolved";
      } catch (error) {
        this.state = `rejected:${error.message}`;
      }
      cx.notify();
    });
  }

  render() {
    return v_flex().child(this.state);
  }
}
"#;

#[cfg(unix)]
const OUTPUT_LIMIT_PROBE: &str = r#"
import { div, View } from "gpui-kit";
import { v_flex } from "gpui-base";
import process from "process";

export default class Probe extends View {
  init(_props, cx) {
    this.state = "pending";
    cx.spawn(async (cx) => {
      try {
        await process.run("/bin/sh", ["-c", "yes x | head -c 8388609"]);
        this.state = "unexpectedly resolved";
      } catch (error) {
        this.state = `rejected:${error.message}`;
      }
      cx.notify();
    });
  }

  render() {
    return v_flex().child(this.state);
  }
}
"#;

const DENIAL_PROBE: &str = r#"
import { div, View } from "gpui-kit";
import { v_flex } from "gpui-base";
import process from "process";

export default class Probe extends View {
  init(_props, cx) {
    try {
      process.run("gpui-shell-denied-command");
      this.state = "unexpectedly allowed";
    } catch (error) {
      this.state = [
        error.message,
        typeof process.kill,
        typeof process.setuid,
        typeof process.setgid,
        String(process.env.CARGO_MANIFEST_DIR),
      ].join("|");
      process.nextTick((suffix) => {
        this.state += suffix;
        cx.notify();
      }, "|tick");
    }
  }
  render(cx) { return v_flex().child(this.state); }
}
"#;

#[cfg(unix)]
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

#[cfg(unix)]
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

#[cfg(unix)]
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

#[gpui::test]
fn process_is_denied_and_ambient_authority_is_absent_on_every_platform(cx: &mut TestAppContext) {
    cx.update(crate::init);
    crate::set_capabilities(Capabilities::new());
    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));
    let view_type = runtime
        .load_source("process-denial.js", DENIAL_PROBE)
        .expect("load script");
    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    let view = context
        .update(|window, cx| runtime.instantiate_view(&view_type, window, cx))
        .expect("instantiate script view");
    draw(&mut context, &view);
    context.run_until_parked();
    draw(&mut context, &view);
    let rendered = snapshot_text(&mut context, &view);
    assert!(
        rendered.contains("capabilities.fs.execute")
            && rendered.contains("|undefined|undefined|undefined|undefined")
            && rendered.contains("|tick"),
        "{rendered}"
    );
}

#[gpui::test]
fn an_application_learns_where_it_is_and_only_the_environment_it_was_granted(
    cx: &mut TestAppContext,
) {
    let directory =
        std::env::temp_dir().join(format!("gpui-shell-process-paths-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&directory);
    std::fs::create_dir_all(directory.join("lib")).expect("application directory");
    std::fs::write(
        directory.join("lib").join("helper.js"),
        "export const helperUrl = import.meta.url;\n",
    )
    .expect("helper module");
    std::fs::write(
        directory.join("main.js"),
        r#"
import { View } from "gpui-kit";
import { v_flex } from "gpui-base";
import os from "os";
import process from "process";
import { helperUrl } from "./lib/helper.js";

export default class Probe extends View {
  render(cx) {
    return v_flex().child([
      import.meta.url,
      helperUrl,
      process.dataDir,
      Object.keys(process.env).join(","),
      os.homedir(),
    ].join("|"));
  }
}
"#,
    )
    .expect("entry module");

    cx.update(crate::init);
    // `HOME` is a default, `CARGO_PKG_NAME` is granted, and `CARGO_MANIFEST_DIR`
    // is set by cargo and neither, so it stays out.
    crate::set_capabilities(Capabilities::new().env_variables(["CARGO_PKG_NAME".to_owned()]));
    let data = directory.join("data");
    crate::policy::update_default(|policy| policy.with_data_dir(data.clone()));
    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));
    runtime.set_editor_files(false);
    let view_type = runtime.load_app(&directory, "main.js").expect("load");
    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    let view = context
        .update(|window, cx| runtime.instantiate_view(&view_type, window, cx))
        .expect("instantiate script view");
    draw(&mut context, &view);
    let rendered = snapshot_text(&mut context, &view);

    let home = crate::runtime::home_dir().expect("a test machine has a home directory");
    let fields: Vec<&str> = rendered
        .lines()
        .find(|line| line.contains("file://"))
        .and_then(|line| line.split_once("file://").map(|(_, rest)| rest))
        .map(|line| line.split('|').collect())
        .unwrap_or_default();
    assert_eq!(fields.len(), 5, "{rendered}");
    assert!(fields[0].ends_with("/main.js"), "{rendered}");
    assert!(
        fields[1].starts_with("file://") && fields[1].ends_with("/lib/helper.js"),
        "{rendered}"
    );
    assert_eq!(fields[2], data.to_string_lossy(), "{rendered}");
    let names: Vec<&str> = fields[3].split(',').collect();
    assert!(names.contains(&"HOME"), "{rendered}");
    assert!(names.contains(&"CARGO_PKG_NAME"), "{rendered}");
    assert!(!names.contains(&"CARGO_MANIFEST_DIR"), "{rendered}");
    assert!(
        fields[4].starts_with(&*home.to_string_lossy()),
        "{rendered}"
    );

    crate::policy::update_default(|policy| policy.with_capabilities(Capabilities::new()));
    let _ = std::fs::remove_dir_all(&directory);
}

#[cfg(unix)]
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

    let runtime = ShellRuntime::new_isolated().expect("runtime");
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
