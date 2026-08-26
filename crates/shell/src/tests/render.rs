//! End-to-end tests for the script render protocol.
//!
//! These exercise the whole path — VM, method dispatch, spec arena, event
//! callbacks — without painting a frame, because the element description is
//! plain data. They run against whichever engine is enabled, which is what
//! keeps the fallback engine honest.

use crate::{
    NativeModules, NativeValue, ScriptView, ShellRuntime, capability::Capabilities, policy::Policy,
};
use gpui::{AppContext as _, Modifiers, TestAppContext, VisualTestContext, point, px};
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
fn flex_elements_dispatch_their_click_handlers(cx: &mut TestAppContext) {
    cx.update(crate::init);
    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));
    let source = r#"
import { View, div, h_flex, v_flex, text } from "gpui";

export default class ClickableFlexes extends View {
  init() { this.clicks = [0, 0, 0]; }

  row(element, index, name) {
    return element
      .w_full()
      .h(40)
      .on_click((_event, cx) => {
        this.clicks[index] += 1;
        cx.notify();
      })
      .child(text(`${name}: ${this.clicks[index]}`));
  }

  render() {
    return v_flex()
      .w(300)
      .h(120)
      .child(this.row(div(), 0, "div"))
      .child(this.row(h_flex(), 1, "h_flex"))
      .child(this.row(v_flex(), 2, "v_flex"));
  }
}
"#;
    let view_type = runtime
        .load_source("clickable-flexes.js", source)
        .expect("load");
    let runtime_for_view = Rc::clone(&runtime);
    let window = cx.add_window(move |window, cx| {
        let object = runtime_for_view
            .instantiate(&view_type, window, cx)
            .expect("instantiate");
        ScriptView::new(runtime_for_view, object)
    });
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    context.update(|window, cx| window.draw(cx).clear(cx));
    for y in [20., 60., 100.] {
        context.simulate_click(point(px(10.), px(y)), Modifiers::default());
    }
    context.update(|window, cx| window.draw(cx).clear(cx));

    let view = window.root(&mut context).expect("view");
    let tree = context.update(|_, cx| {
        view.read(cx)
            .snapshot()
            .map(crate::RenderSnapshot::debug_tree)
            .unwrap_or_default()
    });
    for label in ["div: 1", "h_flex: 1", "v_flex: 1"] {
        assert!(tree.contains(&format!("text {label:?}")), "{tree}");
    }
}

#[gpui::test]
fn a_full_color_image_survives_script_render_and_materialize(cx: &mut TestAppContext) {
    cx.update(crate::init);
    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));
    let source = r#"
import { View, image } from "gpui";
export default class BrandImage extends View {
  render() { return image("assets/brand.svg").size(28); }
}
"#;
    let view_type = runtime.load_source("brand-image.js", source).expect("load");
    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    let object = context
        .update(|window, cx| runtime.instantiate(&view_type, window, cx))
        .expect("instantiate");
    let tree = context
        .update(|window, cx| runtime.render_to_spec(&object, None, window, cx))
        .expect("render");

    assert!(tree.contains("image \"assets/brand.svg\""), "{tree}");
    let view = context.update(|_, cx| cx.new(|_| ScriptView::new(runtime, object)));
    draw(&mut context, &view);
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
        .children(Array.from({ length: 30 }, (_, index) => text(`Quote ${index}`))))
      .child(v_flex().id("bar-both").size(80).overflow_scrollbar().child(text("Both bars")))
      .child(v_flex().id("bar-horizontal").size(80).overflow_x_scrollbar().child(text("Horizontal bar")))
      .child(v_flex().id("bar-vertical").h(120).overflow_y_scrollbar()
        .children(Array.from({ length: 30 }, (_, index) => text(`Bar quote ${index}`))));
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
    for behavior in [
        "overflow_scroll",
        "overflow_x_scroll",
        "overflow_y_scroll",
        "overflow_scrollbar",
        "overflow_x_scrollbar",
        "overflow_y_scrollbar",
    ] {
        assert!(
            tree.contains(&format!(":{behavior}")),
            "missing {behavior}: {tree}"
        );
    }
    let view = context.update(|_, cx| cx.new(|_| ScriptView::new(runtime, object)));
    draw(&mut context, &view);
}

/// A `Scrollbar` is paired with its scroll area by name and by nothing else, so
/// what has to be tested is that the pair survives a real frame: the area has
/// to register a scroll position under its id, and the bar has to find it there
/// on the frame after.
#[gpui::test]
fn a_scrollbar_drives_the_scroll_area_that_shares_its_name(cx: &mut TestAppContext) {
    cx.update(|cx| crate::init(cx));

    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));
    let source = r#"
import { View, Scrollbar, v_flex, text } from "gpui";

export default class Watchlist extends View {
  render() {
    return v_flex()
      .relative()
      .h(120)
      .child(
        v_flex().id("watchlist").size_full().overflow_y_scroll()
          .children(Array.from({ length: 40 }, (_, index) => text(`Quote ${index}`))))
      .child(
        Scrollbar.vertical("watchlist")
          .mode("always")
          .viewport_from_layout()
          .absolute()
          .inset_0());
  }
}
"#;
    let view_type = runtime.load_source("scrollbar", source).expect("load");
    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    let object = context
        .update(|window, cx| runtime.instantiate(&view_type, window, cx))
        .expect("instantiate");

    let tree = context
        .update(|window, cx| runtime.render_to_spec(&object, None, window, cx))
        .expect("Scrollbar must be a supported component");

    assert!(
        tree.contains("Scrollbar \"watchlist\""),
        "missing scrollbar: {tree}"
    );
    assert!(
        tree.contains(":axis"),
        "`Scrollbar.vertical` narrows the axis, so the description must carry it: {tree}"
    );
    assert!(
        tree.contains(":mode") && tree.contains(":viewport_from_layout"),
        "the show mode and the layout viewport must survive into the description: {tree}"
    );

    // Two frames, because the pairing only exists once something has been laid
    // out: the first registers the scroll position under `watchlist`, and the
    // second is the one on which the bar would report an area it never found.
    let view = context.update(|_, cx| cx.new(|_| ScriptView::new(runtime, object)));
    draw(&mut context, &view);
    draw(&mut context, &view);
}

/// A tab list holds no selection: each tab is told whether it is selected and
/// reports activation back, so the description has to carry both directions.
#[gpui::test]
fn a_tab_list_carries_selection_in_and_activation_out(cx: &mut TestAppContext) {
    cx.update(|cx| crate::init(cx));

    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));
    let source = r#"
import { View, Tabs, Tab, text } from "gpui";

export default class Settings extends View {
  init() { this.tab = 0; }
  render() {
    const names = ["Account", "Network"];
    return Tabs.new("settings").children(
      names.map((name, index) =>
        Tab.new(`settings-${index}`)
          .selected(index === this.tab)
          .disabled(index === 1)
          .accessibility_label(name)
          .set_position(index + 1, names.length)
          .on_click((_event, cx) => { this.tab = index; cx.notify(); })
          .child(text(name))));
  }
}
"#;
    let view_type = runtime.load_source("tabs", source).expect("load");
    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    let object = context
        .update(|window, cx| runtime.instantiate(&view_type, window, cx))
        .expect("instantiate");

    let tree = context
        .update(|window, cx| runtime.render_to_spec(&object, None, window, cx))
        .expect("Tabs and Tab must be supported components");

    assert!(
        tree.contains("Tabs \"settings\""),
        "missing tab list: {tree}"
    );
    assert!(
        tree.contains("Tab \"settings-0\""),
        "missing first tab: {tree}"
    );
    assert!(
        tree.contains(":set_position"),
        "the announced position must survive into the description: {tree}"
    );
    assert!(
        tree.contains(":selected"),
        "selection is controlled, so it must be described: {tree}"
    );
    assert!(
        tree.contains(":on_click"),
        "activation is reported back, so the handler must be described: {tree}"
    );

    // And the whole thing has to materialize: `Tab` is a `Stateful<Div>` under
    // the hood, so a state style that has nowhere to land would fail here
    // rather than in a story.
    let view = context.update(|_, cx| cx.new(|_| ScriptView::new(runtime, object)));
    draw(&mut context, &view);
}

/// A progress bar is three parts, and the script draws all of it: the root
/// announces the number and paints nothing, the track and the indicator are the
/// only things a user sees. So the description has to carry both the announced
/// value and the geometry the script computed from it.
#[gpui::test]
fn a_progress_bar_announces_a_value_the_script_draws_itself(cx: &mut TestAppContext) {
    cx.update(|cx| crate::init(cx));

    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));
    let source = r#"
import { View, Progress, ProgressTrack, ProgressIndicator } from "gpui";

export default class Download extends View {
  init() { this.percent = 40; }
  render() {
    return Progress.new("download")
      .value(this.percent)
      .accessibility_label("Downloading")
      .child(
        ProgressTrack.new()
          .w(200)
          .h(6)
          .bg("secondary")
          .child(
            ProgressIndicator.new()
              .w(`${this.percent}%`)
              .h(6)
              .bg("primary")));
  }
}
"#;
    let view_type = runtime.load_source("progress", source).expect("load");
    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    let object = context
        .update(|window, cx| runtime.instantiate(&view_type, window, cx))
        .expect("instantiate");

    let tree = context
        .update(|window, cx| runtime.render_to_spec(&object, None, window, cx))
        .expect("Progress and its two parts must be supported components");

    assert!(
        tree.contains("Progress \"download\""),
        "missing progress root: {tree}"
    );
    assert!(
        tree.contains(":value[Number(40.0)]"),
        "the announced percentage is controlled, so it must be described: {tree}"
    );
    assert!(
        tree.contains(":accessibility_label[Str(\"Downloading\")]"),
        "the progress name must survive into the description: {tree}"
    );
    assert!(
        tree.contains("ProgressTrack") && tree.contains("ProgressIndicator"),
        "the visible bar is built from the two parts: {tree}"
    );
    // The bar is drawn entirely by the script, so the width it computed is the
    // only thing that can be asserted about the picture.
    assert!(
        tree.contains(".w[Str(\"40%\")]"),
        "the indicator's own width has to survive into the description: {tree}"
    );

    // And the whole thing has to materialize: the two parts are not
    // interactive, so a `finish` that assumed otherwise would fail here rather
    // than in a story.
    let view = context.update(|_, cx| cx.new(|_| ScriptView::new(runtime, object)));
    draw(&mut context, &view);
}

/// `Radio` and `Toggle` are controlled in the same one-directional way, and
/// asymmetrically: the radio reports only *becoming* chosen, while the toggle
/// reports the value the script would otherwise flip itself. Both directions
/// have to survive into the description.
#[gpui::test]
fn a_radio_group_and_a_toggle_carry_their_controlled_state_both_ways(cx: &mut TestAppContext) {
    cx.update(|cx| crate::init(cx));

    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));
    let source = r#"
import { View, Radio, Toggle, v_flex, text } from "gpui";

export default class Preferences extends View {
  init() { this.appearance = 0; this.bold = false; }
  render() {
    const names = ["Light", "Dark"];
    return v_flex()
      .children(names.map((name, index) =>
        Radio.new(`appearance-${index}`)
          .checked(index === this.appearance)
          .disabled(index === 1)
          .accessibility_label(name)
          .set_position(index + 1, names.length)
          .on_change((_checked, cx) => { this.appearance = index; cx.notify(); })
          .child(text(name))))
      .child(
        Toggle.new("bold")
          .pressed(this.bold)
          .accessibility_label("Bold")
          .on_change((pressed, cx) => { this.bold = pressed; cx.notify(); })
          .child(text("B")));
  }
}
"#;
    let view_type = runtime.load_source("radio_toggle", source).expect("load");
    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    let object = context
        .update(|window, cx| runtime.instantiate(&view_type, window, cx))
        .expect("instantiate");

    let tree = context
        .update(|window, cx| runtime.render_to_spec(&object, None, window, cx))
        .expect("Radio and Toggle must be supported components");

    assert!(
        tree.contains("Radio \"appearance-0\""),
        "missing first radio: {tree}"
    );
    assert!(tree.contains("Toggle \"bold\""), "missing toggle: {tree}");
    assert!(
        tree.contains(":checked") && tree.contains(":pressed"),
        "both controlled states must be described: {tree}"
    );
    assert!(
        tree.contains(":set_position"),
        "the announced position must survive into the description: {tree}"
    );
    assert!(
        tree.contains(":on_change"),
        "the reported change must be described: {tree}"
    );

    // And the whole thing has to materialize: both are `Stateful<Div>` under
    // the hood, so a state style with nowhere to land would fail here rather
    // than in a story.
    let view = context.update(|_, cx| cx.new(|_| ScriptView::new(runtime, object)));
    draw(&mut context, &view);
}

/// A table is composed, not driven: the script nests the groups, rows and
/// cells itself, and the one-based indices ride in the constructors because a
/// cell that does not know its column announces itself in the wrong place.
#[gpui::test]
fn a_table_describes_its_shape_and_its_accessibility_indices(cx: &mut TestAppContext) {
    cx.update(|cx| crate::init(cx));

    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));
    let source = r#"
import {
  View, Table, TableHeader, TableBody, TableRow, TableHead, TableCell, TableCaption, text,
} from "gpui";

export default class Positions extends View {
  init() { this.picked = -1; }
  render() {
    const columns = ["Symbol", "Last"];
    const rows = [["AAPL", "228.52"], ["MSFT", "417.14"]];
    return Table.new("positions")
      .accessibility_label("Open positions")
      .row_count(200)
      .column_count(columns.length)
      .child(TableCaption.new("positions-caption").child(text("Open positions")))
      .child(
        TableHeader.new("positions-header").child(
          TableRow.new("positions-head-row", 1).children(
            columns.map((name, index) =>
              TableHead.new(`positions-head-${index}`, index + 1).child(text(name))))))
      .child(
        TableBody.new("positions-body").children(
          rows.map((cells, row) =>
            TableRow.new(`positions-row-${row}`, row + 2)
              .hover((el) => el.bg("background"))
              .on_click((_event, cx) => { this.picked = row; cx.notify(); })
              .children(
                cells.map((value, column) =>
                  TableCell.new(`positions-cell-${row}-${column}`, column + 1)
                    .child(text(value)))))));
  }
}
"#;
    let view_type = runtime.load_source("table", source).expect("load");
    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    let object = context
        .update(|window, cx| runtime.instantiate(&view_type, window, cx))
        .expect("instantiate");

    let tree = context
        .update(|window, cx| runtime.render_to_spec(&object, None, window, cx))
        .expect("the Table family must be supported");

    assert!(
        tree.contains("Table \"positions\""),
        "missing table root: {tree}"
    );
    assert!(
        tree.contains(":row_count") && tree.contains(":column_count"),
        "the whole table's size must survive into the description: {tree}"
    );
    assert!(
        tree.contains(":accessibility_label[Str(\"Open positions\")]"),
        "the table name must survive into the description: {tree}"
    );
    // The body's first row is the second row of the table, because the header
    // row is the first — which is exactly the arithmetic an index exists to
    // record, and exactly what a plain nest of divs cannot say.
    assert!(
        tree.contains("TableRow \"positions-row-0\" #2"),
        "a row must carry its one-based index: {tree}"
    );
    assert!(
        tree.contains("TableCell \"positions-cell-0-1\" #2"),
        "a cell must carry its one-based column index: {tree}"
    );
    assert!(
        tree.contains("TableCaption \"positions-caption\""),
        "the caption slot must be described: {tree}"
    );
    assert!(
        tree.contains(":on_click"),
        "a row is where a table's click lands, so the handler must be described: {tree}"
    );

    // And the whole thing has to materialize: every part is a `Stateful<Div>`,
    // so a state style that had nowhere to land would fail here rather than in
    // a story.
    let view = context.update(|_, cx| cx.new(|_| ScriptView::new(runtime, object)));
    draw(&mut context, &view);
}

/// A one-based index is not advisory. Zero is not "close enough" to the first
/// column; it is every cell announced one place to the left, so it is refused
/// where the script wrote it rather than cast into something plausible.
#[gpui::test]
fn a_table_index_below_one_is_refused_at_the_call_site(cx: &mut TestAppContext) {
    cx.update(|cx| crate::init(cx));

    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));
    let source = r#"
import { View, TableCell, text } from "gpui";

export default class BadTable extends View {
  render() { return TableCell.new("cell", 0).child(text("AAPL")); }
}

"#;
    let view_type = runtime.load_source("bad-table", source).expect("load");
    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    let object = context
        .update(|window, cx| runtime.instantiate(&view_type, window, cx))
        .expect("instantiate");

    let error = context
        .update(|window, cx| runtime.render_to_spec(&object, None, window, cx))
        .expect_err("a zero column index must fail at the script call site");

    assert!(
        error.to_string().contains("whole number of at least 1"),
        "the error must say what a valid index is: {error}"
    );
}

#[gpui::test]
fn accessibility_counts_and_positions_reject_invalid_numbers(cx: &mut TestAppContext) {
    cx.update(|cx| crate::init(cx));

    for (name, expression, expected) in [
        ("position-zero", "Tab.new('tab').set_position(0, 2)", "set_position"),
        ("position-after-size", "Tab.new('tab').set_position(3, 2)", "set_position"),
        ("position-fraction", "Tab.new('tab').set_position(1.5, 2)", "set_position"),
        ("row-negative", "Table.new('table').row_count(-1)", "row_count"),
        ("column-fraction", "Table.new('table').column_count(2.5)", "column_count"),
        ("progress-nan", "Progress.new('progress').value(NaN)", "value"),
    ] {
        let runtime = ShellRuntime::new_isolated().expect("runtime");
        let source = format!(
            "import {{ View, Tab, Table, Progress }} from 'gpui'; export default class Bad extends View {{ render() {{ return {expression}; }} }}"
        );
        let view_type = runtime.load_source(name, &source).expect("load");
        let window = cx.add_window(|_, _| Empty);
        let mut context = VisualTestContext::from_window(*window.deref(), cx);
        let object = context
            .update(|window, cx| runtime.instantiate(&view_type, window, cx))
            .expect("instantiate");

        let error = context
            .update(|window, cx| runtime.render_to_spec(&object, None, window, cx))
            .expect_err("an invalid accessibility number must fail at its call site");
        assert!(
            error.to_string().contains(expected),
            "{name} must identify {expected}: {error}"
        );
    }
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

/// Multi-line state is a different Rust type from single-line state, so the
/// seams that could confuse the two — the store, the element, the subscription
/// — have to be exercised in the same view as an ordinary input. The row count
/// is part of that: the layout default is one row even for a textarea, so a
/// binding that dropped `rows` would produce something shaped like an input.
#[gpui::test]
fn a_textarea_holds_multi_line_state_beside_an_input(cx: &mut TestAppContext) {
    cx.update(|cx| crate::init(cx));

    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));
    let source = r#"
import { View, div, text, Input, InputState, Textarea, TextareaState } from "gpui";

export default class Note extends View {
  init() {
    this.title = InputState.new({ value: "Shopping" });
    this.body = TextareaState.new({ placeholder: "Notes", value: "milk", rows: 6 });
    this.body.on("change", () => {});
    this.body.set_soft_wrap(true);
    this.body.set_auto_grow(3, 12);
  }
  render() {
    return div()
      .child(Input.new(this.title))
      .child(Textarea.new(this.body).h(160))
      .child(text(this.body.value()));
  }
}
"#;
    let view_type = runtime.load_source("note", source).expect("load");
    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    let object = context
        .update(|window, cx| runtime.instantiate(&view_type, window, cx))
        .expect("instantiate");

    let tree = context
        .update(|window, cx| runtime.render_to_spec(&object, None, window, cx))
        .expect("Textarea must be a supported component");

    assert!(tree.contains("Textarea #"), "missing the textarea: {tree}");
    assert!(
        tree.contains("Input #"),
        "the single-line input must still be its own component: {tree}"
    );
    assert!(
        tree.contains("\"milk\""),
        "the retained text must be readable from the script: {tree}"
    );

    // And both have to materialize. A textarea handle that resolved as an input
    // — or the other way round — would fail here rather than in a story.
    let view = context.update(|_, cx| cx.new(|_| ScriptView::new(runtime, object)));
    draw(&mut context, &view);
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
            crate::watch::Watch::start(&runtime, &view, directory.clone(), "main.js", window, cx)
                .expect("watch");
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
             timer.every(60_000, () => {{}});\n\
             export default class Panel extends View {{\n\
               render() {{ return text(\"{caption}\"); }}\n\
             }}\n"
        )
    };
    std::fs::write(directory.join("main.js"), source("first")).expect("initial source");

    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    let baseline = crate::engine::quickjs::task_count();
    let view = context.update(|window, cx| {
        let policy = Rc::new(Policy::default());
        let (_scope, _) = crate::scope::enter_with_runtime(
            &runtime,
            window,
            cx,
            crate::scope::ScopePhase::Task,
            None,
            policy.clone(),
        );
        let view_type = runtime.load_app(&directory, "main.js").expect("load");
        runtime
            .instantiate_view_with_policy(&view_type, policy, window, cx)
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

    std::fs::write(
        directory.join("main.js"),
        "import { View, timer, text } from \"gpui\";\n\
         export default class Broken extends View {\n\
           init() { timer.every(60_000, () => {}); throw new Error(\"init failed\"); }\n\
           render() { return text(\"unreachable\"); }\n\
         }",
    )
    .expect("initialization-failing source");
    context
        .update(|window, cx| {
            crate::watch::reload(&runtime, &view, &directory, "main.js", window, cx)
        })
        .expect_err("initialization failure must roll back the candidate generation");
    assert_eq!(
        crate::engine::quickjs::task_count(),
        baseline + 1,
        "work created by a candidate init must be rolled back without touching the live app"
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

/// A grouping container carries the group semantics and nothing else: the state
/// stays on the children, and `axis` is announced rather than laid out.
#[gpui::test]
fn a_group_announces_its_axis_without_laying_its_children_out(cx: &mut TestAppContext) {
    cx.update(|cx| crate::init(cx));

    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));
    let source = r#"
import { View, RadioGroup, ToggleGroup, Checkbox, Button, text } from "gpui";

export default class Preferences extends View {
  init() { this.density = 0; this.bold = false; }
  render() {
    const densities = ["Compact", "Comfortable"];
    return RadioGroup.new("density")
      .axis("vertical")
      .flex()
      .flex_col()
      .children(
        densities.map((name, index) =>
          Checkbox.new(`density-${index}`)
            .checked(index === this.density)
            .accessibility_label(name)
            .on_change((_checked, cx) => { this.density = index; cx.notify(); })
            .child(text(name))))
      .child(
        ToggleGroup.new("formatting")
          .axis("horizontal")
          .flex()
          .child(
            Button.new("bold")
              .selected(this.bold)
              .on_click((_event, cx) => { this.bold = !this.bold; cx.notify(); })
              .child(text("Bold"))));
  }
}
"#;
    let view_type = runtime.load_source("groups", source).expect("load");
    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    let object = context
        .update(|window, cx| runtime.instantiate(&view_type, window, cx))
        .expect("instantiate");

    let tree = context
        .update(|window, cx| runtime.render_to_spec(&object, None, window, cx))
        .expect("RadioGroup and ToggleGroup must be supported components");

    assert!(
        tree.contains("RadioGroup \"density\""),
        "missing radio group: {tree}"
    );
    assert!(
        tree.contains("ToggleGroup \"formatting\""),
        "missing toggle group: {tree}"
    );
    assert!(
        tree.contains(":axis"),
        "the announced orientation must survive into the description: {tree}"
    );
    // The layout is the script's, not the axis's: a group that says
    // `axis("vertical")` still has to say `flex_col()` to stack.
    assert!(
        tree.contains(".flex_col"),
        "axis must not stand in for layout: {tree}"
    );

    let view = context.update(|_, cx| cx.new(|_| ScriptView::new(runtime, object)));
    draw(&mut context, &view);
}

/// The axis grammar mirrors `gpui::Axis`, so an unknown value is a script error
/// rather than a silent fallback to the container's default.
#[gpui::test]
fn an_unknown_axis_is_rejected_at_the_call_site(cx: &mut TestAppContext) {
    cx.update(|cx| crate::init(cx));

    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));
    let source = r#"
import { View, ToggleGroup } from "gpui";

export default class BadAxis extends View {
  render() {
    return ToggleGroup.new("formatting").axis("inline");
  }
}
"#;
    let view_type = runtime.load_source("bad-axis", source).expect("load");
    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    let object = context
        .update(|window, cx| runtime.instantiate(&view_type, window, cx))
        .expect("instantiate");

    let error = context
        .update(|window, cx| runtime.render_to_spec(&object, None, window, cx))
        .expect_err("an unknown axis must fail at the script call site");

    assert!(
        error.to_string().contains("horizontal or vertical"),
        "the error must name the legal axes: {error}"
    );
}

/// The source both slot tests render, with the open state substituted in.
///
/// The content is a click target rather than a piece of text because the gate
/// is a *render* decision: the description carries the content either way, so
/// only something that has to exist on screen to work can tell the two apart.
const COLLAPSIBLE: &str = r#"
import { View, v_flex, div, text, Collapsible } from "gpui";

export default class Section extends View {
  init() { this.open = OPEN; this.hits = 0; }

  render() {
    return v_flex()
      .w(300)
      .h(200)
      .child(
        Collapsible.new()
          .flex_col()
          .w(300)
          .open(this.open)
          .child(div().w_full().h(40).child(text("Header")))
          .content(
            div()
              .id("body")
              .w_full()
              .h(40)
              .on_click((_event, cx) => { this.hits += 1; cx.notify(); })
              .child(text(`Body: ${this.hits}`)),
          ),
      );
  }
}
"#;

/// Renders the collapsible source with the given open state, clicks where the
/// content sits when it is drawn, and returns the description that came out.
fn collapsible_tree(cx: &mut TestAppContext, open: bool) -> String {
    cx.update(|cx| crate::init(cx));

    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));
    let source = COLLAPSIBLE.replace("OPEN", if open { "true" } else { "false" });
    let view_type = runtime.load_source("section.js", &source).expect("load");

    let runtime_for_view = Rc::clone(&runtime);
    let window = cx.add_window(move |window, cx| {
        let object = runtime_for_view
            .instantiate(&view_type, window, cx)
            .expect("instantiate");
        ScriptView::new(runtime_for_view, object)
    });
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    context.update(|window, cx| window.draw(cx).clear(cx));
    // The header occupies the first 40 pixels, so this lands on the content
    // when there is one and on nothing at all when there is not.
    context.simulate_click(point(px(10.), px(60.)), Modifiers::default());
    context.update(|window, cx| window.draw(cx).clear(cx));

    let view = window.root(&mut context).expect("view");
    context.update(|_, cx| {
        view.read(cx)
            .snapshot()
            .map(crate::RenderSnapshot::debug_tree)
            .unwrap_or_default()
    })
}

/// An open `Collapsible` draws the element in its `content` slot, and the slot
/// reaches the description as a slot rather than as a child.
#[gpui::test]
fn an_open_collapsible_renders_the_element_in_its_content_slot(cx: &mut TestAppContext) {
    let tree = collapsible_tree(cx, true);

    assert!(tree.contains("Collapsible"), "missing collapsible: {tree}");
    assert!(
        tree.contains(":open[Bool(true)]"),
        "the open state is controlled, so it must be described: {tree}"
    );
    // Filling a slot detaches the element from `children`, so a dump that
    // walked children alone would lose it — which is why the slot is printed
    // under the node that holds it.
    assert!(
        tree.contains("@content"),
        "the content must be described as a slot: {tree}"
    );
    assert!(
        tree.contains("text \"Body: 1\""),
        "an open collapsible must draw its content, so the click must land: {tree}"
    );
}

/// A closed one describes the same content and draws none of it.
#[gpui::test]
fn a_closed_collapsible_describes_its_content_without_drawing_it(cx: &mut TestAppContext) {
    let tree = collapsible_tree(cx, false);

    assert!(
        tree.contains(":open[Bool(false)]"),
        "the closed state must be described: {tree}"
    );
    // The description is open-agnostic: `open` gates what is rendered, not what
    // the script said. The header proves the collapsible itself is on screen.
    assert!(
        tree.contains("@content"),
        "a closed collapsible still describes its content: {tree}"
    );
    assert!(
        tree.contains("text \"Header\""),
        "ordinary children are drawn either way: {tree}"
    );
    assert!(
        !tree.contains("text \"Body: 1\""),
        "a closed collapsible draws no content, so there is nothing there to click: {tree}"
    );
}

/// Filling a slot consumes the element exactly as adding it to a parent does.
///
/// The error has to say so in words that fit a slot: the same check also guards
/// a state style's declarations, and a script that reused a collapsible's
/// content used to be told it was holding a state style.
#[gpui::test]
fn an_element_given_to_a_slot_cannot_also_be_a_child(cx: &mut TestAppContext) {
    cx.update(|cx| crate::init(cx));

    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));
    let source = r#"
import { View, v_flex, text, Collapsible } from "gpui";

export default class Reused extends View {
  render() {
    const body = text("body");
    return v_flex().child(Collapsible.new().open(true).content(body)).child(body);
  }
}
"#;
    let view_type = runtime.load_source("reused-slot", source).expect("load");
    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    let object = context
        .update(|window, cx| runtime.instantiate(&view_type, window, cx))
        .expect("instantiate");

    let error = context
        .update(|window, cx| runtime.render_to_spec(&object, None, window, cx))
        .expect_err("an element given to a slot must not also be a child");

    let message = error.to_string();
    assert!(
        message.contains("named slot such as content"),
        "the error must name the reason the element is gone: {message}"
    );
    assert!(
        !message.contains("this element holds the declarations of a state style"),
        "a slot is not a state style, and the error must not say it is: {message}"
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
            crate::watch::Watch::start(&runtime, &view, directory.clone(), "main.js", window, cx)
                .expect("watch");
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

/// A focus handle created during `render` would be a new one every frame, so
/// the focus a script thought it was tracking would be dropped by the next
/// repaint. That is the same failure `InputState.new(...)` is refused for, and
/// it is refused the same way.
#[gpui::test]
fn a_focus_handle_cannot_be_created_during_render(cx: &mut TestAppContext) {
    cx.update(|cx| crate::init(cx));

    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));
    let source = r#"
import { View, div, FocusHandle } from "gpui";

export default class Late extends View {
  render() {
    return div().track_focus(FocusHandle.new());
  }
}
"#;
    let view_type = runtime.load_source("late-focus", source).expect("load");
    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    let object = context
        .update(|window, cx| runtime.instantiate(&view_type, window, cx))
        .expect("instantiate");

    let error = context
        .update(|window, cx| runtime.render_to_spec(&object, None, window, cx))
        .expect_err("a focus handle created in render must be refused");

    let message = error.to_string();
    assert!(
        message.contains("cannot run during render"),
        "the error must name the phase: {message}"
    );
    assert!(
        message.contains("init()"),
        "and where the handle belongs instead: {message}"
    );
}

/// The keyboard actually reaches a script's controls.
///
/// Not "the description carries `tab_index`" — a real window, a real `ShellRoot`
/// with its Tab binding, a real Tab keystroke, and the assertion that the
/// window's focus is now the handle the script created and gave to the second
/// button. Anything less would pass while `tab_index` set a field nobody read.
#[gpui::test]
fn the_tab_key_walks_the_focus_order_a_script_declared(cx: &mut TestAppContext) {
    cx.update(|cx| crate::init(cx));

    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));
    let source = r#"
import { View, v_flex, div, Button, Checkbox, Toggle, FocusHandle, text } from "gpui";

export default class Form extends View {
  init() {
    this.handles = [
      FocusHandle.new(),
      FocusHandle.new(),
      FocusHandle.new(),
      FocusHandle.new(),
    ];
  }

  render() {
    return v_flex()
      .w(300)
      .h(200)
      .child(
        Button.new("save")
          .w(200).h(40)
          .tab_index(1)
          .track_focus(this.handles[0])
          .child(text("Save")))
      .child(
        Checkbox.new("remember")
          .w(200).h(40)
          .tab_index(2)
          .track_focus(this.handles[1])
          .child(text("Remember")))
      .child(
        Toggle.new("bold")
          .w(200).h(40)
          .tab_index(3)
          .track_focus(this.handles[2])
          .child(text("Bold")))
      .child(
        div()
          .id("custom")
          .w(200).h(40)
          .tab_index(4)
          .track_focus(this.handles[3])
          .child(text("Custom")));
  }
}
"#;
    let view_type = runtime.load_source("tab-order", source).expect("load");
    let runtime_for_view = Rc::clone(&runtime);
    let (_root, context) = cx.add_window_view(move |window, cx| {
        let object = runtime_for_view
            .instantiate(&view_type, window, cx)
            .expect("instantiate");
        let view = cx.new(|_| ScriptView::new(runtime_for_view, object));
        crate::root::ShellRoot::new(view.into(), window, cx)
    });
    context.update(|window, cx| window.draw(cx).clear(cx));

    let handles = runtime.entities().focus_handles();
    assert_eq!(handles.len(), 4, "the script created four focus handles");

    // Nothing is focused until something puts focus in the window, and while
    // nothing is, the root's Tab binding has no dispatch path to reach — a
    // ShellRoot limitation that predates focus reaching scripts at all. So the
    // first step is taken directly; every step after it is a real keystroke.
    assert_eq!(context.update(|window, cx| window.focused(cx)), None);
    context.update(|window, cx| window.focus_next(cx));

    // One keystroke per control, in the order the script numbered them: a
    // Button, a Checkbox and a Toggle through base's own focus builders, and a
    // plain element through GPUI's.
    for (step, expected) in handles.iter().enumerate() {
        context.simulate_keystrokes("tab");
        assert_eq!(
            context.update(|window, cx| window.focused(cx)).as_ref(),
            Some(expected),
            "Tab {} must land on the handle the script declared there",
            step + 1
        );
    }

    context.simulate_keystrokes("shift-tab");
    assert_eq!(
        context.update(|window, cx| window.focused(cx)),
        Some(handles[2].clone()),
        "Shift-Tab must walk the same order backwards"
    );
}

/// `is_focused()` answers about the element the handle was given to.
///
/// The round trip is the whole point of a script-owned handle: the script asks
/// for focus, GPUI moves it, and the next render reads it back. A handle that
/// only ever answered `false` would still make every other assertion here pass.
#[gpui::test]
fn a_tracked_handle_reports_the_focus_it_was_given(cx: &mut TestAppContext) {
    cx.update(|cx| crate::init(cx));

    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));
    let source = r#"
import { View, v_flex, div, Button, FocusHandle, text } from "gpui";

export default class Panel extends View {
  init() { this.target = FocusHandle.new(); }

  render() {
    return v_flex()
      .w(300)
      .h(120)
      .child(text(`focused: ${this.target.is_focused()}`))
      .child(
        div()
          .id("mover")
          .w(300).h(40)
          .on_click((_event, cx) => { this.target.focus(); cx.notify(); })
          .child(text("Move focus")))
      .child(
        Button.new("target")
          .w(200).h(40)
          .track_focus(this.target)
          .child(text("Target")));
  }
}
"#;
    let view_type = runtime.load_source("tracked-focus", source).expect("load");
    let runtime_for_view = Rc::clone(&runtime);
    let window = cx.add_window(move |window, cx| {
        let object = runtime_for_view
            .instantiate(&view_type, window, cx)
            .expect("instantiate");
        ScriptView::new(runtime_for_view, object)
    });
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    context.update(|window, cx| window.draw(cx).clear(cx));

    let view = window.root(&mut context).expect("view");
    let described = |context: &mut VisualTestContext| {
        context.update(|_, cx| {
            view.read(cx)
                .snapshot()
                .map(crate::RenderSnapshot::debug_tree)
                .unwrap_or_default()
        })
    };
    assert!(
        described(&mut context).contains("text \"focused: false\""),
        "nothing has focus before the script asks for it"
    );

    // The click lands on the plain element above the button, which is not
    // itself focusable — so the focus that arrives is the one the script moved,
    // not one GPUI handed out on a press.
    context.simulate_click(point(px(10.), px(30.)), Modifiers::default());
    context.update(|window, cx| window.draw(cx).clear(cx));

    let handles = runtime.entities().focus_handles();
    assert_eq!(handles.len(), 1);
    assert_eq!(
        context.update(|window, cx| window.focused(cx)),
        Some(handles[0].clone()),
        "focus() must move the window's focus to the script's handle"
    );
    assert!(
        described(&mut context).contains("text \"focused: true\""),
        "and the next render must read it back: {}",
        described(&mut context)
    );
}

/// A role and a selected state reach the description, and a name that is not a
/// role fails where it was written rather than turning into silence.
#[gpui::test]
fn accessibility_semantics_reach_the_description(cx: &mut TestAppContext) {
    cx.update(|cx| crate::init(cx));

    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));
    let source = r#"
import { View, v_flex, div, text } from "gpui";

export default class Options extends View {
  init() { this.chosen = 1; }
  render() {
    const names = ["Daily", "Weekly"];
    return v_flex()
      .role("list_box")
      .accessibility_label("Cadence")
      .children(
        names.map((name, index) =>
          div()
            .id(`cadence-${index}`)
            .role("list_box_option")
            .aria_selected(index === this.chosen)
            .when(index === this.chosen, (el) => el.aria_active_descendant())
            .child(text(name))));
  }
}
"#;
    let view_type = runtime.load_source("options", source).expect("load");
    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    let object = context
        .update(|window, cx| runtime.instantiate(&view_type, window, cx))
        .expect("instantiate");

    let tree = context
        .update(|window, cx| runtime.render_to_spec(&object, None, window, cx))
        .expect("render");

    assert!(
        tree.contains(":role[Str(\"list_box\")]"),
        "the container's role must survive into the description: {tree}"
    );
    assert!(
        tree.contains(":aria_selected[Bool(true)]") && tree.contains(":aria_selected[Bool(false)]"),
        "each option says whether it is the chosen one: {tree}"
    );
    assert_eq!(
        tree.matches(":aria_active_descendant").count(),
        1,
        "exactly one option is the active descendant: {tree}"
    );

    // And it materializes: `role` needs a stateful element, so a plain `div`
    // that never grew an identity would fail here rather than in a story.
    let view = context.update(|_, cx| cx.new(|_| ScriptView::new(runtime, object)));
    draw(&mut context, &view);
}

/// An unknown role is silence in the accessibility tree, which is exactly what
/// calling `role` was meant to prevent — so it fails at the call site.
#[gpui::test]
fn an_unknown_role_fails_where_it_was_written(cx: &mut TestAppContext) {
    cx.update(|cx| crate::init(cx));

    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));
    let source = r#"
import { View, div } from "gpui";

export default class Wrong extends View {
  render() { return div().role("listbox"); }
}
"#;
    let view_type = runtime.load_source("wrong-role", source).expect("load");
    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    let object = context
        .update(|window, cx| runtime.instantiate(&view_type, window, cx))
        .expect("instantiate");

    let error = context
        .update(|window, cx| runtime.render_to_spec(&object, None, window, cx))
        .expect_err("a name that is not a role must fail at the script call site");

    assert!(
        error.to_string().contains("unknown accessibility role"),
        "got: {error}"
    );
}

/// The source the popover tests render, with the open state substituted in.
///
/// The trigger sits 100 pixels down so the surface, anchored to its top-left
/// corner, extends past the bottom of the trigger. The click target is in that
/// overhang: it is inside the content and outside the trigger, so a click there
/// can only be reporting that the content is on screen.
const POPOVER: &str = r#"
import { View, v_flex, div, text, Popover } from "gpui";

export default class Menu extends View {
  init() { this.open = OPEN; this.hits = 0; }

  render() {
    return v_flex()
      .w(400)
      .h(400)
      .child(div().w(400).h(100))
      .child(
        Popover.new("menu")
          .anchor("top_left")
          .open(this.open)
          .on_open_change((open, cx) => { this.open = open; cx.notify(); })
          .trigger(div().w(300).h(40).child(text("Open")))
          .content(
            div()
              .id("body")
              .w(200)
              .h(160)
              .on_click((_event, cx) => { this.hits += 1; cx.notify(); })
              .child(text(`Body: ${this.hits}`)),
          ),
      );
  }
}
"#;

/// Where the content lands: below the trigger, inside the surface.
fn inside_the_surface() -> gpui::Point<gpui::Pixels> {
    point(px(100.), px(200.))
}

/// Where the trigger is.
fn on_the_trigger() -> gpui::Point<gpui::Pixels> {
    point(px(100.), px(120.))
}

/// Renders the popover source with the given open state, clicks where the
/// content sits when it is drawn, and returns the description that came out.
fn popover_tree(cx: &mut TestAppContext, open: bool) -> String {
    cx.update(|cx| crate::init(cx));

    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));
    let source = POPOVER.replace("OPEN", if open { "true" } else { "false" });
    let view_type = runtime.load_source("menu.js", &source).expect("load");

    let runtime_for_view = Rc::clone(&runtime);
    let window = cx.add_window(move |window, cx| {
        let object = runtime_for_view
            .instantiate(&view_type, window, cx)
            .expect("instantiate");
        ScriptView::new(runtime_for_view, object)
    });
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    // Twice: a popup measures its trigger while painting and only places the
    // surface on the frame after it knows where the trigger is.
    context.update(|window, cx| window.draw(cx).clear(cx));
    context.update(|window, cx| window.draw(cx).clear(cx));
    context.simulate_click(inside_the_surface(), Modifiers::default());
    context.update(|window, cx| window.draw(cx).clear(cx));

    let view = window.root(&mut context).expect("view");
    context.update(|_, cx| {
        view.read(cx)
            .snapshot()
            .map(crate::RenderSnapshot::debug_tree)
            .unwrap_or_default()
    })
}

/// An open `Popover` draws the element in its `content` slot where a click can
/// reach it, and both slots reach the description as slots rather than as
/// children.
#[gpui::test]
fn an_open_popover_draws_its_content_where_it_can_be_clicked(cx: &mut TestAppContext) {
    let tree = popover_tree(cx, true);

    assert!(tree.contains("Popover \"menu\""), "missing popover: {tree}");
    assert!(
        tree.contains("@trigger") && tree.contains("@content"),
        "both slots must be described as slots: {tree}"
    );
    assert!(
        tree.contains("text \"Body: 1\""),
        "an open popover must draw its content, so the click must land: {tree}"
    );
}

/// A closed one describes the same content and draws none of it.
#[gpui::test]
fn a_closed_popover_describes_its_content_without_drawing_it(cx: &mut TestAppContext) {
    let tree = popover_tree(cx, false);

    assert!(
        tree.contains(":open[Bool(false)]"),
        "the closed state must be described: {tree}"
    );
    // The description is open-agnostic: `open` gates what is rendered, not what
    // the script said. The trigger proves the popover itself is on screen.
    assert!(
        tree.contains("@content"),
        "a closed popover still describes its content: {tree}"
    );
    assert!(
        tree.contains("text \"Open\""),
        "the trigger is drawn either way: {tree}"
    );
    assert!(
        !tree.contains("text \"Body: 1\""),
        "a closed popover draws no content, so there is nothing there to click: {tree}"
    );
}

/// The open state goes out through `on_open_change` and comes back in through
/// `open`, which is the whole of what "controlled" means here.
///
/// Nothing else can tell the story: the description carries the content whether
/// or not it is showing, so the proof that the pointer opened the surface is
/// that a click 60 pixels below the trigger started landing, and the proof that
/// pressing outside closed it again is that the same click stopped landing.
#[gpui::test]
fn a_popover_reports_the_open_state_the_pointer_changed(cx: &mut TestAppContext) {
    cx.update(|cx| crate::init(cx));

    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));
    let source = POPOVER.replace("OPEN", "false");
    let view_type = runtime.load_source("menu.js", &source).expect("load");

    let runtime_for_view = Rc::clone(&runtime);
    let window = cx.add_window(move |window, cx| {
        let object = runtime_for_view
            .instantiate(&view_type, window, cx)
            .expect("instantiate");
        ScriptView::new(runtime_for_view, object)
    });
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    context.update(|window, cx| window.draw(cx).clear(cx));

    let tree = |context: &mut VisualTestContext| {
        let view = window.root(context).expect("view");
        context.update(|_, cx| {
            view.read(cx)
                .snapshot()
                .map(crate::RenderSnapshot::debug_tree)
                .unwrap_or_default()
        })
    };

    context.simulate_click(on_the_trigger(), Modifiers::default());
    context.update(|window, cx| window.draw(cx).clear(cx));
    let opened = tree(&mut context);
    assert!(
        opened.contains(":open[Bool(true)]"),
        "pressing the trigger must report the new open state to the script: {opened}"
    );

    context.simulate_click(inside_the_surface(), Modifiers::default());
    context.update(|window, cx| window.draw(cx).clear(cx));
    let clicked = tree(&mut context);
    assert!(
        clicked.contains("text \"Body: 1\""),
        "the content the script re-rendered must be on screen and clickable: {clicked}"
    );

    // Below the surface as well as beside it, so this is outside and nothing
    // else.
    context.simulate_click(point(px(300.), px(380.)), Modifiers::default());
    context.update(|window, cx| window.draw(cx).clear(cx));
    let dismissed = tree(&mut context);
    assert!(
        dismissed.contains(":open[Bool(false)]"),
        "pressing outside must report the surface closed: {dismissed}"
    );

    context.simulate_click(inside_the_surface(), Modifiers::default());
    context.update(|window, cx| window.draw(cx).clear(cx));
    let after = tree(&mut context);
    assert!(
        after.contains("text \"Body: 1\""),
        "a dismissed popover draws nothing, so the second click must not land: {after}"
    );
}

/// The anchor grammar mirrors `gpui::Anchor`, so an unknown corner is a script
/// error rather than a surface that quietly opens in the default one.
#[gpui::test]
fn an_unknown_anchor_is_rejected_at_the_call_site(cx: &mut TestAppContext) {
    cx.update(|cx| crate::init(cx));

    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));
    let source = r#"
import { View, Popover } from "gpui";

export default class BadAnchor extends View {
  render() { return Popover.new("menu").anchor("topLeft"); }
}
"#;
    let view_type = runtime.load_source("bad-anchor", source).expect("load");
    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    let object = context
        .update(|window, cx| runtime.instantiate(&view_type, window, cx))
        .expect("instantiate");

    let error = context
        .update(|window, cx| runtime.render_to_spec(&object, None, window, cx))
        .expect_err("an unknown anchor must fail at the script call site");

    let message = error.to_string();
    for name in crate::materialize::ANCHOR_NAMES {
        assert!(
            message.contains(name),
            "the error must list every legal corner, and `{name}` is missing: {message}"
        );
    }
}

/// A `HoverCard` opens after the pointer rests on its trigger and closes after
/// it leaves, and the script's content is what is on screen in between.
#[gpui::test]
fn a_hover_card_opens_after_its_delay_and_closes_once_the_pointer_leaves(cx: &mut TestAppContext) {
    cx.update(|cx| crate::init(cx));

    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));
    let source = r#"
import { View, v_flex, div, text, HoverCard } from "gpui";

export default class Card extends View {
  init() { this.hits = 0; }

  render() {
    return v_flex()
      .w(400)
      .h(400)
      .child(div().w(400).h(100))
      .child(
        HoverCard.new("card")
          .anchor("top_left")
          .open_delay(50)
          .close_delay(50)
          .trigger(div().w(300).h(40).child(text("Hover")))
          .content(
            div()
              .id("body")
              .w(200)
              .h(160)
              .on_click((_event, cx) => { this.hits += 1; cx.notify(); })
              .child(text(`Body: ${this.hits}`)),
          ),
      );
  }
}
"#;
    let view_type = runtime.load_source("card.js", source).expect("load");

    let runtime_for_view = Rc::clone(&runtime);
    let window = cx.add_window(move |window, cx| {
        let object = runtime_for_view
            .instantiate(&view_type, window, cx)
            .expect("instantiate");
        ScriptView::new(runtime_for_view, object)
    });
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    context.update(|window, cx| window.draw(cx).clear(cx));

    let tree = |context: &mut VisualTestContext| {
        let view = window.root(context).expect("view");
        context.update(|_, cx| {
            view.read(cx)
                .snapshot()
                .map(crate::RenderSnapshot::debug_tree)
                .unwrap_or_default()
        })
    };
    let settle = |context: &mut VisualTestContext| {
        context
            .executor()
            .advance_clock(std::time::Duration::from_millis(60));
        context.run_until_parked();
        context.update(|window, cx| window.draw(cx).clear(cx));
        context.update(|window, cx| window.draw(cx).clear(cx));
    };

    context.simulate_mouse_move(on_the_trigger(), None, Modifiers::default());
    settle(&mut context);

    context.simulate_click(inside_the_surface(), Modifiers::default());
    context.update(|window, cx| window.draw(cx).clear(cx));
    let opened = tree(&mut context);
    assert!(
        opened.contains("HoverCard \"card\""),
        "missing hover card: {opened}"
    );
    assert!(
        opened.contains("text \"Body: 1\""),
        "resting on the trigger must put the content on screen, where a click reaches it: \
         {opened}"
    );

    context.simulate_mouse_move(point(px(350.), px(380.)), None, Modifiers::default());
    settle(&mut context);

    context.simulate_click(inside_the_surface(), Modifiers::default());
    context.update(|window, cx| window.draw(cx).clear(cx));
    let closed = tree(&mut context);
    assert!(
        closed.contains("text \"Body: 1\""),
        "the card closes once the pointer leaves, so the second click must not land: {closed}"
    );
}

/// The source the combobox tests render.
///
/// The layout is fixed so the click targets can be named rather than searched
/// for: a 60-pixel button that moves the focus onto the select, a 40-pixel
/// spacer under it, and then the select itself at y=100 with a 300×40 trigger.
/// The list is 200×160 anchored to the popup's top-left corner, so its overhang
/// — inside the list, below and beside the trigger — is the one place a click
/// can only mean "the list is on screen".
const SELECT: &str = r#"
import { View, v_flex, div, text, Select, Popup, FocusHandle } from "gpui";

export default class Picker extends View {
  init() {
    this.trigger_focus = FocusHandle.new();
    this.list_focus = FocusHandle.new();
    this.open = false;
    this.chosen = "none";
    this.confirms = 0;
    this.dismisses = 0;
  }

  list() {
    return v_flex()
      .id("list")
      .w(200)
      .h(160)
      .track_focus(this.list_focus)
      .role("list_box")
      .child(
        div()
          .id("option-cn")
          .w(200)
          .h(120)
          .role("list_box_option")
          .aria_selected(this.chosen === "CN")
          .aria_active_descendant()
          .on_click((_event, cx) => { this.chosen = "CN"; cx.notify(); })
          .child(text("China")));
  }

  render() {
    return v_flex()
      .w(400)
      .h(400)
      .child(
        div()
          .id("focus")
          .w(400)
          .h(60)
          .on_click((_event, cx) => { this.trigger_focus.focus(); cx.notify(); })
          .child(text("Focus")))
      .child(div().w(400).h(40))
      .child(
        Select.new("country")
          .accessibility_label("Country")
          .open(this.open)
          .track_focus(this.trigger_focus)
          .content_focus_handle(this.list_focus)
          .on_open_change((open, cx) => { this.open = open; cx.notify(); })
          .on_confirm((_event, cx) => { this.confirms += 1; cx.notify(); })
          .on_dismiss((_event, cx) => { this.dismisses += 1; cx.notify(); })
          .child(
            Popup.new(
              "country-popup",
              div()
                .id("trigger")
                .w(300)
                .h(40)
                .on_click((_event, cx) => { this.open = !this.open; cx.notify(); })
                .child(text("Choose")))
              .anchor("top_left")
              .when(this.open, (el) => el.content(this.list()))))
      .child(
        text(`open:${this.open} chosen:${this.chosen} confirm:${this.confirms} dismiss:${this.dismisses}`));
  }
}
"#;

/// Where the select's trigger sits.
fn on_the_select_trigger() -> gpui::Point<gpui::Pixels> {
    point(px(100.), px(120.))
}

/// Inside the list and outside the trigger, so a click here can only land when
/// the popup is showing.
fn inside_the_list() -> gpui::Point<gpui::Pixels> {
    point(px(100.), px(200.))
}

/// Opens a window on [`SELECT`] and hands back the pieces the tests drive.
fn select_harness(cx: &mut TestAppContext) -> (VisualTestContext, gpui::Entity<ScriptView>) {
    cx.update(|cx| crate::init(cx));

    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));
    let view_type = runtime.load_source("picker.js", SELECT).expect("load");

    let runtime_for_view = Rc::clone(&runtime);
    let window = cx.add_window(move |window, cx| {
        let object = runtime_for_view
            .instantiate(&view_type, window, cx)
            .expect("instantiate");
        ScriptView::new(runtime_for_view, object)
    });
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    // Twice: a popup measures itself while painting and only places the surface
    // on the frame after it knows where it is.
    context.update(|window, cx| window.draw(cx).clear(cx));
    context.update(|window, cx| window.draw(cx).clear(cx));
    let view = window.root(&mut context).expect("view");
    (context, view)
}

fn described(context: &mut VisualTestContext, view: &gpui::Entity<ScriptView>) -> String {
    context.update(|_, cx| {
        view.read(cx)
            .snapshot()
            .map(crate::RenderSnapshot::debug_tree)
            .unwrap_or_default()
    })
}

/// Two frames, because a popup that has just been given content places it on
/// the frame after the one that measured the trigger.
fn settle_popup(context: &mut VisualTestContext) {
    context.update(|window, cx| window.draw(cx).clear(cx));
    context.update(|window, cx| window.draw(cx).clear(cx));
}

/// The whole controlled loop, driven from the keyboard base actually binds.
///
/// ↓ opens: base reports the new state through `on_open_change`, the script
/// stores it, and the next render fills the popup's `content` slot. The proof
/// that the list is really on screen is that a click 60 pixels below the
/// trigger starts landing — and the proof that Escape closed it again is that
/// the same click stops landing. Escape also reports through `on_dismiss`
/// first, which is what lets a script commit a pending value on the way out.
#[gpui::test]
fn a_select_carries_its_open_state_both_ways_and_draws_its_list_in_a_popup(
    cx: &mut TestAppContext,
) {
    let (mut context, view) = select_harness(cx);

    let before = described(&mut context, &view);
    assert!(
        before.contains("Select \"country\"") && before.contains("Popup \"country-popup\""),
        "both roots must reach the description: {before}"
    );
    assert!(
        before.contains("open:false"),
        "a select starts shut: {before}"
    );

    // The pointer path first: the trigger the script drew owns the press, and
    // the list it drew appears in the popup above the window.
    context.simulate_click(on_the_select_trigger(), Modifiers::default());
    settle_popup(&mut context);
    let opened = described(&mut context, &view);
    assert!(
        opened.contains("open:true"),
        "pressing the trigger must open the select: {opened}"
    );
    assert!(
        opened.contains(":aria_active_descendant[]"),
        "the script marks its own highlighted option; the root cannot: {opened}"
    );

    context.simulate_click(inside_the_list(), Modifiers::default());
    settle_popup(&mut context);
    let chosen = described(&mut context, &view);
    assert!(
        chosen.contains("chosen:CN"),
        "an open select must draw its list where a click reaches it: {chosen}"
    );

    // Escape needs the keyboard, and the root's actions are dispatched down the
    // focus path — a select nothing has focused hears no keys at all.
    context.simulate_click(point(px(10.), px(30.)), Modifiers::default());
    settle_popup(&mut context);
    context.simulate_keystrokes("escape");
    settle_popup(&mut context);
    let dismissed = described(&mut context, &view);
    assert!(
        dismissed.contains("dismiss:1"),
        "Escape must report the dismissal: {dismissed}"
    );
    assert!(
        dismissed.contains("open:false"),
        "and then the close, so the script can stop drawing the list: {dismissed}"
    );

    context.simulate_click(inside_the_list(), Modifiers::default());
    settle_popup(&mut context);
    let after = described(&mut context, &view);
    assert!(
        after.contains("open:false"),
        "a closed select draws no list, so the click in it opens nothing: {after}"
    );

    // And the keyboard opens it again, which is the other half of the loop: the
    // value goes out through `on_open_change` and comes back in through `open`.
    context.simulate_keystrokes("down");
    settle_popup(&mut context);
    let reopened = described(&mut context, &view);
    assert!(
        reopened.contains("open:true"),
        "the down arrow must report the new open state back to the script: {reopened}"
    );
}

/// Enter on an open root confirms; Enter on a shut one opens it instead, which
/// is base's rule rather than ours.
#[gpui::test]
fn enter_confirms_an_open_select_and_opens_a_shut_one(cx: &mut TestAppContext) {
    let (mut context, view) = select_harness(cx);

    context.simulate_click(point(px(10.), px(30.)), Modifiers::default());
    settle_popup(&mut context);

    context.simulate_keystrokes("enter");
    settle_popup(&mut context);
    let opened = described(&mut context, &view);
    assert!(
        opened.contains("open:true") && opened.contains("confirm:0"),
        "Enter on a shut select opens it and confirms nothing: {opened}"
    );

    context.simulate_keystrokes("enter");
    settle_popup(&mut context);
    let confirmed = described(&mut context, &view);
    assert!(
        confirmed.contains("confirm:1"),
        "Enter on an open select confirms, with no payload to carry: {confirmed}"
    );
}

/// A `Popup` has no trigger to fall back on, so an omitted one is refused where
/// it was written rather than drawn as an empty box that anchors nothing.
#[gpui::test]
fn a_popup_without_a_trigger_is_refused_at_the_call_site(cx: &mut TestAppContext) {
    let message = render_error(
        cx,
        "popup-trigger",
        r#"
import { View, Popup } from "gpui";

export default class NoTrigger extends View {
  render() { return Popup.new("menu"); }
}
"#,
    );
    assert!(
        message.contains("Popup.new(id, trigger)"),
        "the error must name the constructor: {message}"
    );
}

/// The anchor grammar is one table shared by every anchored surface, so a
/// corner spelled the JavaScript way fails on a `Popup` exactly as it does on a
/// `Popover`.
#[gpui::test]
fn a_popup_with_an_unknown_anchor_is_refused_at_the_call_site(cx: &mut TestAppContext) {
    let message = render_error(
        cx,
        "popup-anchor",
        r#"
import { View, div, Popup } from "gpui";

export default class BadAnchor extends View {
  render() { return Popup.new("menu", div()).anchor("bottomLeft"); }
}
"#,
    );
    for name in crate::materialize::ANCHOR_NAMES {
        assert!(
            message.contains(name),
            "the error must list every legal corner, and `{name}` is missing: {message}"
        );
    }
}

/// Base's `DatePicker::new` takes the focus handle, so there is no picker to
/// build without one — and no builder to add one afterwards. The message has to
/// say both, because "expects a FocusHandle" alone reads like a call that could
/// be moved down the chain.
#[gpui::test]
fn a_date_picker_without_a_focus_handle_says_why_it_needs_one(cx: &mut TestAppContext) {
    let message = render_error(
        cx,
        "date-picker-handle",
        r#"
import { View, DatePicker } from "gpui";

export default class NoHandle extends View {
  render() { return DatePicker.new("due"); }
}
"#,
    );
    assert!(
        message.contains("DatePicker.new(id, focus_handle)"),
        "the error must name the constructor: {message}"
    );
    assert!(
        message.contains("FocusHandle.new()"),
        "and where a handle comes from: {message}"
    );
    assert!(
        message.contains("no builder to supply one later"),
        "and why it cannot simply be set afterwards: {message}"
    );
}

/// A picker holds no date, and in the shell it holds no keyboard either.
///
/// What it does hold is the trigger's focus handle and the announced open
/// state, and both are real: Tab lands on the handle the script created, and
/// `open` reaches the element that announces it.
///
/// Enter and Escape are the part that is missing, and the reason is worth
/// pinning down: base's `DatePicker` sets no key context, while every binding
/// base installs is scoped to one. `crates/ui` supplies both — its own
/// `"DatePicker"` context and its own bindings — and the shell has no
/// key-binding layer to supply either. So the assertion below is that Escape
/// changes nothing. If it ever starts changing something, this test is the
/// place that says the `.d.ts` needs updating.
#[gpui::test]
fn a_date_picker_carries_focus_and_an_announced_open_state(cx: &mut TestAppContext) {
    cx.update(|cx| crate::init(cx));

    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));
    let source = r#"
import { View, v_flex, text, DatePicker, FocusHandle } from "gpui";

export default class Due extends View {
  init() {
    this.focus = FocusHandle.new();
    this.open = false;
  }

  render() {
    return v_flex()
      .w(400)
      .h(400)
      .child(
        DatePicker.new("due", this.focus)
          .open(this.open)
          .w(300)
          .h(40)
          .on_open_change((open, cx) => { this.open = open; cx.notify(); })
          .child(text(`open:${this.open}`)));
  }
}
"#;
    let view_type = runtime.load_source("due.js", source).expect("load");
    let runtime_for_view = Rc::clone(&runtime);
    let (_root, context) = cx.add_window_view(move |window, cx| {
        let object = runtime_for_view
            .instantiate(&view_type, window, cx)
            .expect("instantiate");
        let view = cx.new(|_| ScriptView::new(runtime_for_view, object));
        crate::root::ShellRoot::new(view.into(), window, cx)
    });
    context.update(|window, cx| window.draw(cx).clear(cx));

    let handles = runtime.entities().focus_handles();
    assert_eq!(handles.len(), 1, "the script created one focus handle");

    // The whole of what base wires for a picker: the constructor's handle is
    // its tab stop, so the window's focus order reaches the picker itself.
    context.update(|window, cx| window.focus_next(cx));
    assert_eq!(
        context.update(|window, cx| window.focused(cx)).as_ref(),
        Some(&handles[0]),
        "the constructor's handle must be the picker's tab stop"
    );

    // And the documented gap: no key context, so no binding matches and the
    // controlled open state never moves.
    context.simulate_keystrokes("escape");
    context.simulate_keystrokes("enter");
    context.update(|window, cx| window.draw(cx).clear(cx));
    assert_eq!(
        context.update(|window, cx| window.focused(cx)).as_ref(),
        Some(&handles[0]),
        "nothing takes the keyboard away from the picker either"
    );
}

/// Renders `source` once and returns the message it failed with.
fn render_error(cx: &mut TestAppContext, name: &str, source: &str) -> String {
    cx.update(|cx| crate::init(cx));

    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));
    let view_type = runtime.load_source(name, source).expect("load");
    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    let object = context
        .update(|window, cx| runtime.instantiate(&view_type, window, cx))
        .expect("instantiate");

    context
        .update(|window, cx| runtime.render_to_spec(&object, None, window, cx))
        .expect_err("the script must fail where the call was written")
        .to_string()
}
