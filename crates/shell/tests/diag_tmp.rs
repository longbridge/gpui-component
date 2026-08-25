//! Temporary diagnostic: which frame enters the VM, and why.
use std::ops::Deref;

use gpui::{AppContext as _, Entity, IntoElement as _, TestAppContext, VisualTestContext};
use gpui_shell::{ScriptView, ShellRuntime};

const TEMPLATE: &str = r#"
import { View, div, v_flex, h_flex, text, Button } from "gpui";

export default class Grid extends View {
  init() {
    this.rows = __ROWS__;
    this.columns = 10;
  }

  cell(row, column) {
    return div().flex().items_center().justify_center().w(90).h(24).px(6)
      .rounded(4).bg("surface").text_color("foreground").text_sm()
      .child(text(`${row}:${column}`));
  }

  row(row) {
    const cells = [];
    for (let column = 0; column < this.columns; column += 1) cells.push(this.cell(row, column));
    return h_flex().gap(6).py(2).children(cells);
  }

  render() {
    const rows = [];
    for (let row = 0; row < this.rows; row += 1) rows.push(this.row(row));
    return v_flex().size_full().p(12).gap(4).bg("background").children(rows)
      .child(Button.new("refresh").px(10).py(4).rounded(6).bg("primary").child(text("Refresh")));
  }
}
"#;

#[gpui::test]
fn which_frame_enters_the_vm(cx: &mut TestAppContext) {
    for rows in [40usize, 400] {
        cx.update(|cx| gpui_shell::init(cx));
        let runtime = ShellRuntime::new().expect("runtime");
        cx.update(|cx| runtime.set_global(cx));
        let source = TEMPLATE.replace("__ROWS__", &rows.to_string());
        let view_type = runtime.load_source("grid", &source).expect("load");
        let window = cx.add_window(|_, _| Empty);
        let mut context = VisualTestContext::from_window(*window.deref(), cx);
        let object = context
            .update(|window, cx| runtime.instantiate(&view_type, window, cx))
            .expect("instantiate");
        // Reproduce the scaling test's order: A then B, then the view.
        let snapshot = context.update(|window, cx| {
            let mut last = None;
            for _ in 0..50 {
                last = Some(runtime.build_snapshot(&object, None, window, cx).expect("render"));
            }
            last.expect("snapshot")
        });
        context.draw(
            gpui::Point::default(),
            gpui::size(gpui::px(800.), gpui::px(600.)),
            |window, cx| {
                for _ in 0..50 {
                    let element = gpui_shell::materialize::materialize(&runtime, &snapshot, window, cx);
                    std::hint::black_box(&element);
                }
                gpui::div().into_any_element()
            },
        );
        let view = context.update(|_, cx| cx.new(|_| ScriptView::new(runtime.clone(), object)));

        draw(&mut context, &view);
        let mut last = runtime.metrics().read().script_renders();
        let theme_at_start = gpui_shell::theme::generation();
        println!("\n--- {rows} rows: baseline renders={last}, theme_gen={theme_at_start}");

        for frame in 0..50 {
            let dirty_before = context.update(|_, cx| view.read(cx).is_dirty());
            draw(&mut context, &view);
            let now = runtime.metrics().read().script_renders();
            if now != last {
                println!(
                    "frame {frame}: +{} script render(s); dirty_before={dirty_before}, \
                     theme_gen={} (start {theme_at_start})",
                    now - last,
                    gpui_shell::theme::generation(),
                );
                last = now;
            }
        }
        println!("--- {rows} rows: total extra renders over 50 frames = {}", last - runtime.metrics().read().script_renders() + (runtime.metrics().read().script_renders() - last));
    }
}

fn draw(context: &mut VisualTestContext, view: &Entity<ScriptView>) {
    let view = view.clone();
    context.draw(
        gpui::Point::default(),
        gpui::size(gpui::px(800.), gpui::px(600.)),
        move |_, _| view.into_any_element(),
    );
}

struct Empty;
impl gpui::Render for Empty {
    fn render(&mut self, _: &mut gpui::Window, _: &mut gpui::Context<Self>) -> impl gpui::IntoElement {
        gpui::div()
    }
}
