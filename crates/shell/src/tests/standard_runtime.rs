use std::ops::Deref;

use gpui::{IntoElement as _, TestAppContext, VisualTestContext};

use crate::ShellRuntime;

const PURE_MODULES: &str = r#"
import { View, v_flex, text } from "gpui";
import { Buffer } from "buffer";
import path from "path";
import { URL } from "url";
import { deflateSync, inflateSync } from "zlib";
import { createHash } from "crypto";

export default class Probe extends View {
  render() {
    const input = Buffer.from("shell", "utf8");
    const compressed = deflateSync(input);
    const inflated = inflateSync(compressed).toString("utf8");
    const digest = createHash("sha256").update(input).digest("hex");
    const url = new URL("https://example.com/a?b=1");
    return v_flex().child(text([
      input.toString("hex"),
      path.join("a", "b"),
      url.hostname,
      inflated,
      digest,
    ].join("|")));
  }
}
"#;

#[gpui::test]
fn llrt_pure_modules_execute_inside_the_shell_runtime(cx: &mut TestAppContext) {
    cx.update(crate::init);
    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));
    let view_type = runtime
        .load_source("standard-runtime.js", PURE_MODULES)
        .expect("load LLRT imports");
    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    let view = context
        .update(|window, cx| runtime.instantiate_view(&view_type, window, cx))
        .expect("instantiate probe");

    let view_to_draw = view.clone();
    context.draw(
        gpui::Point::default(),
        gpui::size(gpui::px(400.), gpui::px(300.)),
        move |_, _| view_to_draw.into_any_element(),
    );

    let rendered = context.update(|_, cx| {
        view.read(cx)
            .snapshot()
            .map(crate::RenderSnapshot::debug_tree)
            .unwrap_or_default()
    });
    let joined_path = format!("a{}b", std::path::MAIN_SEPARATOR);
    let expected = format!(
        "7368656c6c|{joined_path}|example.com|shell|\
         ce635c4eabff5e4f56dba8fb1e39ca235530aa2b6b18533eef1af3862016c577"
    );
    assert!(
        rendered.contains(&expected),
        "unexpected Standard Runtime result: {rendered}"
    );
}

#[test]
fn node_prefixed_modules_are_not_part_of_the_shell_contract() {
    let runtime = ShellRuntime::new_isolated().expect("runtime");
    let error = runtime
        .load_source(
            "node-prefix.js",
            r#"import { Buffer } from "node:buffer"; export default Buffer;"#,
        )
        .expect_err("the shell must not advertise Node.js module names");
    assert!(error.to_string().contains("node:buffer"), "{error:#}");
}

#[gpui::test]
fn safe_host_standard_modules_replace_the_old_gpui_exports(cx: &mut TestAppContext) {
    let source = r#"
import { View, v_flex, text } from "gpui";
import console from "console";
import os from "os";
import process from "process";

export default class Probe extends View {
  render() {
    console.log("standard runtime", os.platform());
    return v_flex().child(text([
      typeof process.run,
      process.cwd(),
      Object.keys(process.env).length,
      os.homedir(),
      os.tmpdir(),
    ].join("|")));
  }
}

"#;
    cx.update(crate::init);
    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));
    let view_type = runtime
        .load_source("host-standard.js", source)
        .expect("load");
    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    let view = context
        .update(|window, cx| runtime.instantiate_view(&view_type, window, cx))
        .expect("instantiate");
    let view_to_draw = view.clone();
    context.draw(
        gpui::Point::default(),
        gpui::size(gpui::px(400.), gpui::px(300.)),
        move |_, _| view_to_draw.into_any_element(),
    );
    let rendered = context.update(|_, cx| {
        view.read(cx)
            .snapshot()
            .map(crate::RenderSnapshot::debug_tree)
            .unwrap_or_default()
    });
    assert!(rendered.contains("function|.|0|.|."), "{rendered}");

    let error = runtime
        .load_source(
            "old-host-exports.js",
            r#"import { fs, process } from "gpui"; export default fs || process;"#,
        )
        .expect_err("the old gpui.fs/process exports must be removed");
    assert!(
        error.to_string().to_ascii_lowercase().contains("not find"),
        "{error:#}"
    );
}

#[gpui::test]
fn synchronous_unawaited_host_calls_hit_the_runtime_task_limit(cx: &mut TestAppContext) {
    let source = r#"
import { View, v_flex, text, sleep } from "gpui";

export default class Probe extends View {
  init() {
    this.state = "unlimited";
    for (let index = 0; index < 2000; index += 1) {
      try {
        sleep(60000);
      } catch (error) {
        this.state = `limited:${error.message}`;
        break;
      }
    }
  }
  render() { return v_flex().child(text(this.state)); }
}
"#;
    cx.update(crate::init);
    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));
    let view_type = runtime
        .load_source("host-task-limit.js", source)
        .expect("load");
    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    let view = context
        .update(|window, cx| runtime.instantiate_view(&view_type, window, cx))
        .expect("instantiate");
    let view_to_draw = view.clone();
    context.draw(
        gpui::Point::default(),
        gpui::size(gpui::px(400.), gpui::px(300.)),
        move |_, _| view_to_draw.into_any_element(),
    );
    let rendered = context.update(|_, cx| {
        view.read(cx)
            .snapshot()
            .map(crate::RenderSnapshot::debug_tree)
            .unwrap_or_default()
    });
    assert!(
        rendered.contains("limited:") && rendered.contains("outstanding host task limit"),
        "synchronous calls must be stopped by a per-runtime hard limit: {rendered}"
    );
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
