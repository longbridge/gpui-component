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
    let runtime = ShellRuntime::new().expect("runtime");
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
    assert!(
        rendered.contains(
            "7368656c6c|a/b|example.com|shell|\
             ce635c4eabff5e4f56dba8fb1e39ca235530aa2b6b18533eef1af3862016c577"
        ),
        "unexpected Standard Runtime result: {rendered}"
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
