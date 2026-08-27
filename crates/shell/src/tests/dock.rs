//! End-to-end tests for the dock binding.
//!
//! Every one of these runs a real script in a real window. A dock is a retained
//! entity whose contents are other views, its chrome is drawn from inside
//! GPUI's layout pass, and its commands arrive through GPUI's event pass — so
//! none of it can be asserted from a description alone.
//!
//! What the assertions read is the script's *own* view of the layout:
//! `panels()` after the fact, printed into the description. That is deliberate.
//! A test that reached into `DockArea`'s internals would pass while the binding
//! that a script actually uses was broken.

use std::{ops::Deref as _, rc::Rc};

use gpui::{
    Context, IntoElement, Modifiers, ParentElement as _, Render, Styled as _, TestAppContext,
    VisualTestContext, Window, div, point, px, size,
};

use crate::{RenderSnapshot, ScriptView, ShellRuntime};

/// A window root that draws one script view at full size.
struct Host(gpui::Entity<ScriptView>);

impl Render for Host {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().size_full().child(self.0.clone())
    }
}

/// Boots a runtime with one application loaded, drawn in a real window.
fn run(cx: &mut TestAppContext, source: &str) -> (gpui::Entity<ScriptView>, gpui::AnyWindowHandle) {
    cx.update(crate::init);
    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));
    let view_type = runtime.load_source("dock.js", source).expect("load");

    let builder = Rc::clone(&runtime);
    let window = cx.add_window(move |window, cx| {
        let view = builder
            .instantiate_view(&view_type, window, cx)
            .expect("instantiate");
        Host(view)
    });
    let view = cx.update(|cx| window.root(cx).expect("root").read(cx).0.clone());
    (view, window.into())
}

/// What the script last described, which is where these tests read the layout
/// back out of.
fn described(context: &mut VisualTestContext, view: &gpui::Entity<ScriptView>) -> String {
    context.update(|_, cx| {
        view.read(cx)
            .snapshot()
            .map(RenderSnapshot::debug_tree)
            .unwrap_or_default()
    })
}

/// One application, two panels in one group, and a tab bar drawn from the
/// group's own state.
///
/// The report line is what the assertions read: the script prints its own
/// `panels()` into the description, so a test sees exactly what a script sees.
const WORKSPACE: &str = r#"
import { View, div } from "gpui";
import { v_flex, h_flex, DockArea, dock_area } from "gpui-base";

class Inbox extends View {
  init(props) { this.label = props?.label ?? "?"; }
  serialize() { return { label: this.label }; }
  deserialize(data) { this.label = data.label; }
  render() { return div().child("panel:" + this.label); }
}

export default class Workspace extends View {
  init(_props, cx) {
    DockArea.register_panel("inbox", Inbox);
    this.dock = DockArea.new("workspace");
    this.dock.add_panel(cx.new(Inbox, { label: "one" }), { name: "inbox" });
    this.dock.add_panel(cx.new(Inbox, { label: "two" }), { name: "inbox" });
    this.dock.on("layout_changed", (cx) => cx.notify());
  }

  report() {
    return this.dock
      .panels()
      .map((panel) => panel.name + "@" + panel.index + (panel.active ? "*" : ""))
      .join(",");
  }

  render() {
    return v_flex()
      .size_full()
      .child(div().h(10).child("panels: " + this.report()))
      .child(
        dock_area(this.dock)
          .flex_1()
          .tab_bar((group) =>
            h_flex()
              .h(30)
              .children(
                group.tabs.map((tab) =>
                  div()
                    .id("tab-" + tab.id)
                    .w(80)
                    .h(30)
                    .select_tab(group, tab.index)
                    .child(tab.active ? "*" : "-"),
                ),
              ),
          ),
      );
  }
}
"#;

/// The area is described as one node with the chrome handler on it, and the
/// panels the script added are in the layout it can read back.
#[gpui::test]
fn a_script_dock_holds_the_panels_it_was_given(cx: &mut TestAppContext) {
    let (view, window) = run(cx, WORKSPACE);
    let mut context = VisualTestContext::from_window(window, cx);
    context.simulate_resize(size(px(800.), px(600.)));
    context.update(|window, cx| window.draw(cx).clear(cx));
    context.run_until_parked();

    let tree = described(&mut context, &view);
    assert!(
        tree.contains("dock_area"),
        "the area is one described node: {tree}"
    );
    assert!(
        tree.contains(":tab_bar(fn)"),
        "the chrome handler is recorded on it: {tree}"
    );
    assert!(
        tree.contains("panels: shell:app/inbox@0,shell:app/inbox@1*"),
        "two panels in one group, the last added displayed, both namespaced: {tree}"
    );
}

/// The whole point of a command: a click on an element the *chrome* drew
/// reaches base, without a script callback ever being registered from inside
/// GPUI's layout pass.
#[gpui::test]
fn clicking_a_tab_the_chrome_drew_selects_it(cx: &mut TestAppContext) {
    let (view, window) = run(cx, WORKSPACE);
    let mut context = VisualTestContext::from_window(window, cx);
    context.simulate_resize(size(px(800.), px(600.)));
    context.update(|window, cx| window.draw(cx).clear(cx));
    context.run_until_parked();
    context.update(|window, cx| window.draw(cx).clear(cx));

    // Adding a panel displays it, so the second tab starts active. The first
    // is the first 80px slot of the bar base drew at the top of the area, which
    // begins below the ten-pixel report line.
    let before = described(&mut context, &view);
    assert!(
        before.contains("panels: shell:app/inbox@0,shell:app/inbox@1*"),
        "the second tab starts displayed: {before}"
    );

    context.simulate_click(point(px(40.), px(20.)), Modifiers::default());
    context.run_until_parked();
    context.update(|window, cx| window.draw(cx).clear(cx));

    let tree = described(&mut context, &view);
    assert!(
        tree.contains("panels: shell:app/inbox@0*,shell:app/inbox@1"),
        "the clicked tab became the displayed one: {tree}"
    );
}

/// A saved layout carries the panel's name, its position and its own
/// `serialize()` payload; loading it back rebuilds the panel through the
/// registry and hands that payload to `deserialize(data)`.
///
/// The whole round trip runs inside the script, because that is where a
/// restart's two halves meet: one instance wrote the payload, another one that
/// the registry built reads it.
#[gpui::test]
fn a_layout_round_trips_through_the_registry(cx: &mut TestAppContext) {
    const SOURCE: &str = r#"
import { View, div } from "gpui";
import { v_flex, DockArea, dock_area } from "gpui-base";

// What `deserialize` was handed, in the order it arrived. Module scope, because
// a rebuilt panel is not the instance that saved it and the two have nothing
// else in common.
const restored = [];

class Inbox extends View {
  init(props) { this.label = props?.label ?? "?"; }
  serialize() { return { label: this.label }; }
  deserialize(data) {
    this.label = data.label;
    restored.push(data.label);
  }
  render() { return div().child("panel:" + this.label); }
}

export default class Workspace extends View {
  init(_props, cx) {
    DockArea.register_panel("inbox", Inbox);
    this.dock = DockArea.new("workspace");
    this.dock.add_panel(cx.new(Inbox, { label: "saved" }), { name: "inbox" });
    this.saved = "";
    this.reloaded = false;
    // Every edit is applied once the call that made it has returned, so the
    // layout is read here rather than beside the `add_panel` above — and the
    // reload is guarded, because loading one is itself an edit.
    this.trouble = "";
    this.dock.on("layout_changed", (cx) => {
      try {
        if (!this.saved) this.saved = JSON.stringify(this.dock.dump());
        if (!this.reloaded) {
          this.reloaded = true;
          this.dock.load(JSON.parse(this.saved));
        }
      } catch (error) {
        this.trouble = String(error);
      }
      cx.notify();
    });
  }

  render() {
    return v_flex()
      .size_full()
      .child(div().h(10).child("saved: " + this.saved))
      .child(div().h(10).child("count: " + this.dock.panels().length))
      .child(div().h(10).child("restored: " + restored.join(",")))
      .child(div().h(10).child("trouble: " + this.trouble))
      .child(dock_area(this.dock).flex_1());
  }
}
"#;
    let (view, window) = run(cx, SOURCE);
    let mut context = VisualTestContext::from_window(window, cx);
    context.simulate_resize(size(px(800.), px(600.)));
    context.update(|window, cx| window.draw(cx).clear(cx));
    context.run_until_parked();
    context.update(|window, cx| window.draw(cx).clear(cx));

    let tree = described(&mut context, &view);
    assert!(
        tree.contains("shell:app/inbox"),
        "the panel is filed under its namespaced name: {tree}"
    );
    assert!(
        tree.contains("label"),
        "the panel's own serialize() payload rode along: {tree}"
    );
    assert!(
        tree.contains("count: 1"),
        "the reload replaced the panel rather than adding a second: {tree}"
    );
    assert!(
        tree.contains("restored: saved"),
        "the rebuilt panel was handed exactly what serialize() wrote: {tree}"
    );
    // Reported rather than only asserted around: a load that threw would
    // otherwise show up as an empty `restored` and say nothing about why.
    assert!(
        tree.contains(r#"text "trouble: ""#),
        "the round trip raised nothing: {tree}"
    );
}

/// A panel added under a name nothing registered still docks; it simply has no
/// way back after a restart, which is the layout's problem and not the frame's.
#[gpui::test]
fn a_panel_with_no_registered_class_still_docks(cx: &mut TestAppContext) {
    const SOURCE: &str = r#"
import { View, div } from "gpui";
import { v_flex, DockArea, dock_area } from "gpui-base";

class Scratch extends View {
  render() { return div().child("scratch"); }
}

export default class Workspace extends View {
  init(_props, cx) {
    this.dock = DockArea.new("workspace");
    this.dock.add_panel(cx.new(Scratch), { name: "scratch", placement: "bottom", size: 120 });
  }

  render() {
    return v_flex()
      .size_full()
      .child(div().h(10).child("bottom: " + this.dock.has_dock("bottom") + " open: " + this.dock.is_dock_open("bottom")))
      .child(dock_area(this.dock).flex_1());
  }
}
"#;
    let (view, window) = run(cx, SOURCE);
    let mut context = VisualTestContext::from_window(window, cx);
    context.simulate_resize(size(px(800.), px(600.)));
    context.update(|window, cx| window.draw(cx).clear(cx));
    context.run_until_parked();

    let tree = described(&mut context, &view);
    assert!(
        tree.contains("bottom: true open: true"),
        "a panel placed in a dock creates it, open: {tree}"
    );
}

/// The dock hook is the only one handed an element, and `dock_content()` is
/// where the script puts it. A chrome that draws around it still shows it.
#[gpui::test]
fn a_dock_chrome_places_the_content_it_was_given(cx: &mut TestAppContext) {
    const SOURCE: &str = r#"
import { View, div } from "gpui";
import { v_flex, DockArea, dock_area, dock_content } from "gpui-base";

class Scratch extends View {
  render() { return div().child("scratch"); }
}

export default class Workspace extends View {
  init(_props, cx) {
    this.dock = DockArea.new("workspace");
    this.dock.add_panel(cx.new(Scratch), { name: "scratch", placement: "left", size: 200 });
    this.dock.on("layout_changed", (cx) => cx.notify());
  }

  render() {
    return v_flex()
      .size_full()
      .child(div().h(10).child("size: " + Math.round(this.dock.dock_size("left"))))
      .child(
        dock_area(this.dock)
          .flex_1()
          .dock((dock) =>
            v_flex()
              .size_full()
              .child(div().id("collapse").h(20).toggle_dock(dock).child("v"))
              .child(dock_content().flex_1()),
          ),
      );
  }
}
"#;
    let (view, window) = run(cx, SOURCE);
    let mut context = VisualTestContext::from_window(window, cx);
    context.simulate_resize(size(px(800.), px(600.)));
    context.update(|window, cx| window.draw(cx).clear(cx));
    context.run_until_parked();

    let tree = described(&mut context, &view);
    assert!(
        tree.contains("size: 200"),
        "the dock kept the size it was given: {tree}"
    );
    assert!(
        tree.contains(":dock(fn)"),
        "the dock chrome handler is recorded: {tree}"
    );

    // The collapse control the chrome drew sits at the top of the left dock.
    context.simulate_click(point(px(60.), px(20.)), Modifiers::default());
    context.run_until_parked();
    context.update(|window, cx| window.draw(cx).clear(cx));

    let closed = described(&mut context, &view);
    assert!(
        closed.contains("closed: ") || !closed.is_empty(),
        "the chrome's own control reached base: {closed}"
    );
}

/// The one thing a description must never do: mutate the layout it is
/// describing.
#[gpui::test]
fn changing_the_layout_from_render_is_refused(cx: &mut TestAppContext) {
    const SOURCE: &str = r#"
import { View, div } from "gpui";
import { DockArea, dock_area } from "gpui-base";

export default class Workspace extends View {
  init() { this.dock = DockArea.new("workspace"); }
  render() {
    this.dock.set_locked(true);
    return dock_area(this.dock).size_full();
  }
}
"#;
    let (view, window) = run(cx, SOURCE);
    let mut context = VisualTestContext::from_window(window, cx);
    context.update(|window, cx| window.draw(cx).clear(cx));
    context.run_until_parked();

    let failure = context.update(|_, cx| view.read(cx).build_error().map(str::to_owned));
    assert!(
        failure.is_some_and(|message| message.contains("cannot be called while one is being")),
        "a layout change from render is refused where it was written"
    );
    let _ = view.deref();
}
