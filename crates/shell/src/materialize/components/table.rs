//! The `Table` family — a semantic table, composed the way HTML composes one.
//!
//! There is no delegate, no row source and no per-item callback here. Base's
//! table is seven container types that a script nests by hand: a `Table` around
//! a `TableHeader` and a `TableBody`, each holding `TableRow`s, each holding
//! `TableHead`s or `TableCell`s. Everything a script would ask a delegate for —
//! which rows exist, what a cell says, what a click means — it already knows,
//! because it wrote the loop that built them.
//!
//! What the family does add is the part a nest of `div`s cannot express: the
//! accessibility tree. Each type contributes its role, and three of them carry
//! a one-based index — `TableRow::new(id, row_index)`,
//! `TableHead::new(id, column_index)`, `TableCell::new(id, column_index)` —
//! which is why those three constructors take a second argument rather than a
//! builder method. An index is not optional decoration on a table cell; a cell
//! that does not know its column announces itself out of place.
//!
//! `row_count` and `column_count` on the root are the other half of that: they
//! describe the *whole* table, including rows a script chose not to render, so
//! a screen reader can say "row 5 of 200" for a window onto a long list. A
//! table that renders everything it has can leave them out.
//!
//! `TableCaption` is here for composition rather than semantics: base gives it
//! no role today, so it is a plain identified container that happens to be
//! where a caption belongs.

use std::rc::Rc;

use gpui::{
    AnyElement, IntoElement, ParentElement, SharedString, StatefulInteractiveElement,
    StyleRefinement, Styled, prelude::FluentBuilder as _,
};
use gpui_base::{Table, TableBody, TableCaption, TableCell, TableHead, TableHeader, TableRow};

use crate::{
    engine::ShellRuntime,
    materialize::{
        Behavior, Children, StateStyles, dispatch_click, finish, warn_ignored_key,
        warn_unhonoured_a11y, with_active_and_focus, with_hover,
    },
};

/// The table root. Its identity comes from `new(id)`, so `id()` is ignored.
pub(in crate::materialize) fn table(
    runtime: &Rc<ShellRuntime>,
    id: String,
    refinement: StyleRefinement,
    behavior: Behavior,
    states: StateStyles,
    children: Children,
) -> AnyElement {
    warn_ignored_key(&behavior, "Table");
    warn_unhonoured_a11y(&behavior, "Table", &[]);
    let mut table = Table::new(SharedString::from(id))
        .when_some(behavior.row_count, |table, count| table.row_count(count))
        .when_some(behavior.column_count, |table, count| {
            table.column_count(count)
        });
    if let Some(label) = behavior.accessibility_label.clone() {
        table = table.accessibility_label(label);
    }
    part(runtime, table, refinement, behavior, states, children)
}

/// The header row group.
pub(in crate::materialize) fn table_header(
    runtime: &Rc<ShellRuntime>,
    id: String,
    refinement: StyleRefinement,
    behavior: Behavior,
    states: StateStyles,
    children: Children,
) -> AnyElement {
    warn_ignored_key(&behavior, "TableHeader");
    warn_unhonoured_a11y(&behavior, "TableHeader", &[]);
    let header = TableHeader::new(SharedString::from(id));
    part(runtime, header, refinement, behavior, states, children)
}

/// The body row group.
pub(in crate::materialize) fn table_body(
    runtime: &Rc<ShellRuntime>,
    id: String,
    refinement: StyleRefinement,
    behavior: Behavior,
    states: StateStyles,
    children: Children,
) -> AnyElement {
    warn_ignored_key(&behavior, "TableBody");
    warn_unhonoured_a11y(&behavior, "TableBody", &[]);
    let body = TableBody::new(SharedString::from(id));
    part(runtime, body, refinement, behavior, states, children)
}

/// One row, announced at `row_index` of the table's rows.
pub(in crate::materialize) fn table_row(
    runtime: &Rc<ShellRuntime>,
    id: String,
    row_index: usize,
    refinement: StyleRefinement,
    behavior: Behavior,
    states: StateStyles,
    children: Children,
) -> AnyElement {
    warn_ignored_key(&behavior, "TableRow");
    warn_unhonoured_a11y(&behavior, "TableRow", &[]);
    let row = TableRow::new(SharedString::from(id), row_index);
    part(runtime, row, refinement, behavior, states, children)
}

/// One column header, announced at `column_index` of the row's cells.
pub(in crate::materialize) fn table_head(
    runtime: &Rc<ShellRuntime>,
    id: String,
    column_index: usize,
    refinement: StyleRefinement,
    behavior: Behavior,
    states: StateStyles,
    children: Children,
) -> AnyElement {
    warn_ignored_key(&behavior, "TableHead");
    warn_unhonoured_a11y(&behavior, "TableHead", &[]);
    let head = TableHead::new(SharedString::from(id), column_index);
    part(runtime, head, refinement, behavior, states, children)
}

/// One data cell, announced at `column_index` of the row's cells.
pub(in crate::materialize) fn table_cell(
    runtime: &Rc<ShellRuntime>,
    id: String,
    column_index: usize,
    refinement: StyleRefinement,
    behavior: Behavior,
    states: StateStyles,
    children: Children,
) -> AnyElement {
    warn_ignored_key(&behavior, "TableCell");
    warn_unhonoured_a11y(&behavior, "TableCell", &[]);
    let cell = TableCell::new(SharedString::from(id), column_index);
    part(runtime, cell, refinement, behavior, states, children)
}

/// A visual caption container. Base gives it no accessibility relationship to
/// the table; name the `Table` itself with `accessibility_label(...)`.
pub(in crate::materialize) fn table_caption(
    runtime: &Rc<ShellRuntime>,
    id: String,
    refinement: StyleRefinement,
    behavior: Behavior,
    states: StateStyles,
    children: Children,
) -> AnyElement {
    warn_ignored_key(&behavior, "TableCaption");
    warn_unhonoured_a11y(&behavior, "TableCaption", &[]);
    let caption = TableCaption::new(SharedString::from(id));
    part(runtime, caption, refinement, behavior, states, children)
}

/// The tail every member of the family shares.
///
/// All seven are `Stateful<Div>` underneath, so all seven take pointer
/// handling and the full set of state styles; the only thing that differs
/// between them is the constructor and the role it carries. `on_click` is
/// wired here rather than left to the script's own wrapper because a row is
/// where a table's click naturally lands, and dropping it would make the one
/// interaction a table has unreachable.
///
/// The `disabled` guard mirrors `flex_element` rather than base: no table part
/// has a disabled state of its own, so `disabled(true)` would otherwise report
/// a dead control that still answers clicks.
fn part<E>(
    runtime: &Rc<ShellRuntime>,
    element: E,
    refinement: StyleRefinement,
    behavior: Behavior,
    states: StateStyles,
    children: Children,
) -> AnyElement
where
    E: Styled + ParentElement + StatefulInteractiveElement + IntoElement + 'static,
{
    let element = element.when_some(
        behavior.on_click.filter(|_| !behavior.disabled),
        |element, callback| {
            let runtime = Rc::downgrade(runtime);
            element.on_click(move |event, window, cx| {
                dispatch_click(&runtime, callback, event, window, cx);
            })
        },
    );
    let element = with_hover(element, &states);
    let element = with_active_and_focus(element, &states);
    finish(element, refinement, children)
}
