//! The M0 viability gate.
//!
//! The design doc (§20.3) says the whole approach stands or falls on one
//! number: how long it takes script code to describe a realistic interface.
//! That is why the engine sits behind a seam — this benchmark is what decides
//! which engine the seam should point at.
//!
//! Run it with output:
//!
//! ```text
//! cargo test -p gpui-shell --release --test benchmark -- --nocapture
//! cargo test -p gpui-shell --release --no-default-features --features luajit \
//!     --test benchmark -- --nocapture
//! ```

use std::{ops::Deref, time::Instant};

use gpui::{TestAppContext, VisualTestContext};
use gpui_shell::ShellRuntime;

/// Rows and columns chosen to land near the doc's "typical panel" figure:
/// 40 rows x 5 cells plus wrappers is ~250 nodes, each carrying 8-12 ops.
const ROWS: usize = 40;
const COLUMNS: usize = 5;
const ITERATIONS: usize = 50;

#[cfg(feature = "quickjs")]
const SOURCE: &str = r#"
import { View, div, v_flex, h_flex, text, Button } from "gpui";

export default class Grid extends View {
  init() {
    this.rows = 40;
    this.columns = 5;
  }

  cell(row, column) {
    return div()
      .flex()
      .items_center()
      .justify_center()
      .w(90)
      .h(24)
      .px(6)
      .rounded(4)
      .bg("surface")
      .text_color("foreground")
      .text_sm()
      .child(text(`${row}:${column}`));
  }

  row(row) {
    const cells = [];
    for (let column = 0; column < this.columns; column += 1) {
      cells.push(this.cell(row, column));
    }
    return h_flex().gap(6).py(2).children(cells);
  }

  render() {
    const rows = [];
    for (let row = 0; row < this.rows; row += 1) {
      rows.push(this.row(row));
    }
    return v_flex()
      .size_full()
      .p(12)
      .gap(4)
      .bg("background")
      .children(rows)
      .child(Button.new("refresh").px(10).py(4).rounded(6).bg("primary").child(text("Refresh")));
  }
}
"#;

#[cfg(not(feature = "quickjs"))]
const SOURCE: &str = r#"
local gpui = require("gpui")
local Grid = gpui.view("Grid")

function Grid:init()
  self.rows = 40
  self.columns = 5
end

function Grid:cell(row, column)
  return gpui.div()
    :flex():items_center():justify_center()
    :w(90):h(24):px(6):rounded(4)
    :bg("surface"):text_color("foreground"):text_sm()
    :child(gpui.text(row .. ":" .. column))
end

function Grid:row(row)
  local cells = {}
  for column = 0, self.columns - 1 do
    cells[#cells + 1] = self:cell(row, column)
  end
  return gpui.h_flex():gap(6):py(2):children(cells)
end

function Grid:render(cx)
  local rows = {}
  for row = 0, self.rows - 1 do
    rows[#rows + 1] = self:row(row)
  end
  return gpui.v_flex()
    :size_full():p(12):gap(4):bg("background")
    :children(rows)
    :child(gpui.Button.new("refresh"):px(10):py(4):rounded(6):bg("primary"):child(gpui.text("Refresh")))
end

return Grid
"#;

const ENGINE: &str = if cfg!(feature = "quickjs") {
    "quickjs"
} else if cfg!(feature = "luajit") {
    "luajit"
} else {
    "lua54"
};

#[gpui::test]
fn describing_a_panel_stays_inside_the_frame_budget(cx: &mut TestAppContext) {
    cx.update(|cx| gpui_shell::init(cx));

    let runtime = ShellRuntime::new().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));

    let view_type = runtime.load_source("grid", SOURCE).expect("load");
    let object = runtime.instantiate(&view_type).expect("instantiate");

    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);

    // Warm up: the style table and the module both initialize lazily.
    let nodes = context.update(|window, cx| {
        let tree = runtime
            .render_to_spec(&object, None, window, cx)
            .expect("render");
        tree.lines().count()
    });

    let started = Instant::now();
    context.update(|window, cx| {
        for _ in 0..ITERATIONS {
            runtime
                .render_to_spec(&object, None, window, cx)
                .expect("render");
        }
    });
    let elapsed = started.elapsed();

    let per_render = elapsed / ITERATIONS as u32;
    let ops = nodes * 10; // roughly ten recorded calls per node
    println!(
        "\n[{ENGINE}] {nodes} nodes ({ROWS}x{COLUMNS}) — {:.3} ms per render, \
         ~{} ns per recorded op ({ITERATIONS} iterations)\n",
        per_render.as_secs_f64() * 1000.0,
        per_render.as_nanos() as usize / ops.max(1),
    );

    // A smoke bound, not the real gate: the doc's 1.5 ms budget is for a
    // release build, and this assertion has to hold in debug too.
    assert!(
        per_render.as_millis() < 200,
        "[{ENGINE}] describing {nodes} nodes took {per_render:?}, which is far outside any budget"
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
