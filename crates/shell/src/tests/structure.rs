//! Whether a rebuild produces the structure it replaced, and what that would
//! be worth.
//!
//! §20.7 of `docs/gpui-shell.md` proposes a template cache: a description split
//! into a reusable structure and the dynamic slots inside it, so that a
//! value-only change writes slots instead of running the builder again. The
//! whole idea rests on an assumption nothing had measured — that a dirty render
//! usually produces the shape the previous one produced — and on a bound nobody
//! had counted: how much of a description is values a template could fill,
//! against handlers it could not.
//!
//! These tests answer both, in that order:
//!
//! ```text
//! does the shape repeat?   ── StructureFingerprint, counted by RuntimeMetrics
//! how much would it save?  ── the slot census below
//! ```
//!
//! The census is a `println!` rather than an assertion. It is a reading of a
//! synthetic panel, not a property of the runtime, and pinning it would make an
//! unrelated change to the panel look like a regression.

use std::ops::Deref;

use crate::{
    ScriptView, ShellRuntime,
    spec::{Component, SpecOp},
};
use gpui::{AppContext as _, Entity, IntoElement as _, TestAppContext, VisualTestContext};

const ENTRY: &str = "structure.js";

/// A view whose every render changes a value and nothing else.
const VALUES_ONLY: &str = r#"
import { View } from "gpui";
import { v_flex, h_flex } from "gpui-base";

export default class Quote extends View {
  init() {
    this.tick = 0;
  }

  render() {
    this.tick += 1;
    return v_flex()
      .gap(4)
      .child(h_flex().gap(6).child("AAPL").child(`${230 + this.tick}.42`))
      .child(h_flex().gap(6).child("MSFT").child(`${410 + this.tick}.08`));
  }
}
"#;

/// The same, with a handler on every row: the `CallbackId` behind each one is
/// minted fresh on every render and retired with the snapshot generation, so a
/// fingerprint that kept it would report a change every single time.
const VALUES_ONLY_WITH_HANDLERS: &str = r#"
import { View } from "gpui";
import { v_flex, h_flex, Button } from "gpui-base";

export default class Quote extends View {
  init() {
    this.tick = 0;
  }

  render() {
    this.tick += 1;
    return v_flex()
      .gap(4)
      .child(Button.new("aapl").on_click(() => this.tick).child(`${230 + this.tick}.42`))
      .child(Button.new("msft").on_click(() => this.tick).child(`${410 + this.tick}.08`));
  }
}
"#;

/// A view that alternates between two shapes.
const ALTERNATING_BRANCH: &str = r#"
import { View } from "gpui";
import { v_flex, h_flex } from "gpui-base";

export default class Branch extends View {
  init() {
    this.tick = 0;
  }

  render() {
    this.tick += 1;
    if (this.tick % 2 === 0) {
      return v_flex().gap(4).child("loading");
    }
    return v_flex().gap(4).child(h_flex().child("content"));
  }
}
"#;

/// A view that grows by one row per render.
const GROWING_LIST: &str = r#"
import { View } from "gpui";
import { v_flex } from "gpui-base";

export default class Growing extends View {
  init() {
    this.rows = 0;
  }

  render() {
    this.rows += 1;
    const children = [];
    for (let row = 0; row < this.rows; row += 1) {
      children.push(`row ${row}`);
    }
    return v_flex().gap(4).children(children);
  }
}
"#;

/// The census panel: a watchlist of the shape §20.7 names as the best case —
/// repeated rows, a handler each, and only the numbers moving.
const WATCHLIST: &str = r#"
import { View, div } from "gpui";
import { v_flex, h_flex, Button } from "gpui-base";

export default class Watchlist extends View {
  init() {
    this.tick = 0;
    this.rows = 40;
  }

  row(index) {
    const price = (100 + index + this.tick / 100).toFixed(2);
    return h_flex()
      .gap(6)
      .py(2)
      .px(6)
      .rounded(4)
      .bg("surface")
      .child(div().w(80).text_sm().text_color("foreground").child(`SYM${index}`))
      .child(div().w(80).text_sm().text_color("foreground").child(price))
      .child(div().w(60).text_sm().text_color("muted_foreground").child("+1.42%"))
      .child(Button.new(`trade-${index}`).px(8).py(2).on_click(() => index).child("Trade"));
  }

  render() {
    this.tick += 1;
    const rows = [];
    for (let index = 0; index < this.rows; index += 1) {
      rows.push(this.row(index));
    }
    return v_flex().size_full().p(12).gap(4).bg("background").children(rows);
  }
}
"#;

#[gpui::test]
fn a_value_only_change_repeats_the_structure(cx: &mut TestAppContext) {
    let (runtime, mut context, view) = script_view(cx, VALUES_ONLY);

    render_once(&mut context, &view);
    let first = runtime.metrics().read();
    assert_eq!(
        (first.structure_repeats(), first.structure_changes()),
        (0, 0),
        "a first build has no predecessor and is not a data point either way"
    );

    invalidate(&mut context, &view);
    invalidate(&mut context, &view);

    let reading = runtime.metrics().read();
    assert_eq!(
        (reading.structure_repeats(), reading.structure_changes()),
        (2, 0),
        "only the numbers moved, so both rebuilds described the shape they replaced"
    );
    assert_eq!(reading.structure_repeat_rate(), Some(1.0));
}

#[gpui::test]
fn a_fresh_handler_is_not_a_change_of_structure(cx: &mut TestAppContext) {
    let (runtime, mut context, view) = script_view(cx, VALUES_ONLY_WITH_HANDLERS);

    render_once(&mut context, &view);
    invalidate(&mut context, &view);

    let reading = runtime.metrics().read();
    assert_eq!(
        (reading.structure_repeats(), reading.structure_changes()),
        (1, 0),
        "every render mints new CallbackIds; counting them as shape would make \
         every description containing a handler look new"
    );
}

#[gpui::test]
fn taking_the_other_branch_changes_the_structure(cx: &mut TestAppContext) {
    let (runtime, mut context, view) = script_view(cx, ALTERNATING_BRANCH);

    render_once(&mut context, &view);
    invalidate(&mut context, &view);
    invalidate(&mut context, &view);

    let reading = runtime.metrics().read();
    assert_eq!(
        (reading.structure_repeats(), reading.structure_changes()),
        (0, 2),
        "the two branches describe different trees"
    );
    assert_eq!(reading.structure_repeat_rate(), Some(0.0));
}

#[gpui::test]
fn a_row_appearing_changes_the_structure(cx: &mut TestAppContext) {
    let (runtime, mut context, view) = script_view(cx, GROWING_LIST);

    render_once(&mut context, &view);
    invalidate(&mut context, &view);

    let reading = runtime.metrics().read();
    assert_eq!(
        (reading.structure_repeats(), reading.structure_changes()),
        (0, 1),
        "one more child is one more node and one more attachment"
    );
}

/// What a template could and could not fill, counted on a 40-row watchlist
/// whose prices move and whose structure does not.
///
/// Prints rather than asserts — see this module's comment. The three numbers
/// are §20.7's step 2: the slot ceiling (`arena.len()` against the positions
/// that actually differ), and the handler share that bounds it.
#[gpui::test]
fn the_slot_census_of_a_repeating_panel(cx: &mut TestAppContext) {
    let (runtime, mut context, object) = script_object(cx, WATCHLIST);

    let (before, after) = context.update(|window, cx| {
        let mut build = || {
            runtime
                .build_snapshot(&object, None, crate::policy::default(), window, cx)
                .expect("render")
        };
        // Three builds, comparing the last two: the first pays for lazily
        // initialized module state and is not representative of a steady tick.
        build();
        let before = census(&build());
        let after = census(&build());
        (before, after)
    });

    assert_eq!(
        before.structure, after.structure,
        "the watchlist's shape must repeat, or the census below is measuring \
         something other than the case a template serves"
    );

    let diff = before.diff(&after);
    let total_ops: usize = after.nodes.iter().map(|node| node.1.len()).sum();

    println!(
        "\n[E] slot census — 40-row watchlist, prices moving, shape repeating\
         \n    nodes                    {}\
         \n    recorded ops             {total_ops}\
         \n    components that differ   {} ({:.1}% of nodes)\
         \n    value ops that differ    {} ({:.1}% of ops)\
         \n    handler ops              {} ({:.1}% of ops) — always differ, never fillable\
         \n    slot ceiling             {} of {} positions ({:.1}%)",
        after.nodes.len(),
        diff.components,
        percent(diff.components, after.nodes.len()),
        diff.value_ops,
        percent(diff.value_ops, total_ops),
        diff.handler_ops,
        percent(diff.handler_ops, total_ops),
        diff.components + diff.value_ops + diff.handler_ops,
        after.nodes.len() + total_ops,
        percent(
            diff.components + diff.value_ops + diff.handler_ops,
            after.nodes.len() + total_ops
        ),
    );
}

fn percent(part: usize, whole: usize) -> f64 {
    if whole == 0 {
        return 0.0;
    }
    part as f64 * 100.0 / whole as f64
}

/// One description, flattened into owned data so two of them can be compared
/// after both borrows have ended.
struct Census {
    structure: crate::spec::StructureFingerprint,
    nodes: Vec<(Option<Component>, Vec<SpecOp>)>,
}

/// How many positions two descriptions of the same shape disagree on.
struct Diff {
    components: usize,
    value_ops: usize,
    handler_ops: usize,
}

impl Census {
    fn diff(&self, other: &Self) -> Diff {
        let mut diff = Diff {
            components: 0,
            value_ops: 0,
            handler_ops: 0,
        };

        for (mine, theirs) in self.nodes.iter().zip(&other.nodes) {
            if mine.0 != theirs.0 {
                diff.components += 1;
            }
            for (left, right) in mine.1.iter().zip(&theirs.1) {
                // Handlers are counted apart from values rather than among
                // them: a `CallbackId` differs on every render by construction,
                // so folding it in would overstate what a template could fill.
                if matches!(right, SpecOp::Callback(..) | SpecOp::ActionCallback(..)) {
                    diff.handler_ops += 1;
                } else if left != right {
                    diff.value_ops += 1;
                }
            }
        }

        diff
    }
}

fn census(snapshot: &crate::RenderSnapshot) -> Census {
    let arena = snapshot.arena();
    Census {
        structure: snapshot.structure(),
        nodes: (0..arena.len() as u32)
            .map(|id| {
                let node = arena.node(id).expect("node");
                (node.component().cloned(), node.ops().to_vec())
            })
            .collect(),
    }
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

fn script_view(
    cx: &mut TestAppContext,
    source: &str,
) -> (
    std::rc::Rc<ShellRuntime>,
    VisualTestContext,
    Entity<ScriptView>,
) {
    let (runtime, mut context, object) = script_object(cx, source);
    let view = context.update(|_, cx| cx.new(|_| ScriptView::new(runtime.clone(), object)));
    (runtime, context, view)
}

fn script_object(
    cx: &mut TestAppContext,
    source: &str,
) -> (
    std::rc::Rc<ShellRuntime>,
    VisualTestContext,
    crate::engine::ViewObject,
) {
    cx.update(|cx| crate::init(cx));

    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));

    let view_type = runtime.load_source(ENTRY, source).expect("load");

    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    let object = context.update(|window, cx| {
        runtime
            .instantiate(&view_type, window, cx)
            .expect("instantiate")
    });

    (runtime, context, object)
}

fn render_once(context: &mut VisualTestContext, view: &Entity<ScriptView>) {
    let view = view.clone();
    context.draw(
        gpui::Point::default(),
        gpui::size(gpui::px(400.), gpui::px(300.)),
        move |_, _| view.into_any_element(),
    );
}

/// Marks the view dirty the way `cx.notify()` does, then draws — which is the
/// only thing that runs the script again.
fn invalidate(context: &mut VisualTestContext, view: &Entity<ScriptView>) {
    context.update(|_, cx| view.update(cx, |view, _| view.invalidate()));
    render_once(context, view);
}
