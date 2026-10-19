//! Controls showing rows of data: Table, List.

use std::{rc::Rc, sync::Arc};

use gpui_kit::{
    AnyElement, AppContext as _, Context, ElementId, Empty, Entity, InteractiveElement as _,
    IntoElement, ParentElement as _, Render, Role, SharedString, Size,
    StatefulInteractiveElement as _, StyleRefinement, Styled as _, Window,
    base::{
        Button, StyledExt as _, Table, TableBody, TableCaption, TableCell, TableHead, TableHeader,
        TableRow, TestSupportExt as _, VirtualListScrollHandle, v_virtual_list,
    },
    div,
    prelude::FluentBuilder as _,
    px, size,
};
use serde::Deserialize;
use serde_json::json;

use crate::{Control, Events, PropType, Props, StateStyles, catalog::class_style};

pub(super) fn controls() -> Vec<Control> {
    vec![table(), list()]
}

/// The id of a control's part named `name`.
fn part(props: &Props, name: impl Into<SharedString>) -> ElementId {
    ElementId::NamedChild(Arc::new(props.id()), name.into())
}

/// Prop `name`, a row or item index; `None` when unset or negative.
fn index(props: &Props, name: &str) -> Option<usize> {
    props
        .has(name)
        .then(|| props.number(name))
        .filter(|ix| *ix >= 0.0)
        .map(|ix| ix as usize)
}

/// A column of a table.
#[derive(Deserialize)]
struct Column {
    title: SharedString,
    width: Option<f32>,
    #[serde(default)]
    align: Align,
}

#[derive(Deserialize, Default, Clone, Copy)]
#[serde(rename_all = "lowercase")]
enum Align {
    #[default]
    Left,
    Center,
    Right,
}

/// `cell` in `column`: its fixed width and the alignment of its text.
fn column<E: gpui_kit::Styled + gpui_kit::prelude::FluentBuilder>(
    cell: E,
    column: Option<&Column>,
) -> E {
    let Some(column) = column else {
        return cell;
    };
    cell.when_some(column.width, |cell, width| cell.flex_none().w(px(width)))
        .map(|cell| match column.align {
            Align::Left => cell.text_left(),
            Align::Center => cell.text_center(),
            Align::Right => cell.text_right(),
        })
}

const ROW_STATES: [&str; 3] = ["rowHoverClass", "rowActiveClass", ""];

fn table() -> Control {
    Control::new("Table", |props, events, _, cx| {
        let columns = props.json::<Vec<Column>>("columns").unwrap_or_default();
        let rows = props
            .json::<Vec<Vec<SharedString>>>("rows")
            .unwrap_or_default();
        let selected = index(props, "selected");
        let caption = props
            .string("caption")
            .filter(|caption| !caption.is_empty());
        props
            .styled(Table::new(props.id()), cx)
            .row_count(rows.len() + 1)
            .column_count(columns.len())
            .when_some(props.string("accessibilityLabel"), |this, label| {
                this.accessibility_label(label)
            })
            .child(
                TableHeader::new(part(props, "header")).child(
                    props
                        .styled_by(
                            "headerClass",
                            TableRow::new(part(props, "header-row"), 1),
                            cx,
                        )
                        .children(columns.iter().enumerate().map(|(ix, col)| {
                            column(
                                props.styled_by(
                                    "headClass",
                                    TableHead::new(part(props, format!("head-{ix}")), ix + 1),
                                    cx,
                                ),
                                Some(col),
                            )
                            .child(col.title.clone())
                        })),
                ),
            )
            .child(
                props
                    .styled_by("bodyClass", TableBody::new(part(props, "body")), cx)
                    .children(rows.into_iter().enumerate().map(|(row_ix, row)| {
                        let is_selected = selected == Some(row_ix);
                        let events = events.clone();
                        props
                            .states(
                                ROW_STATES,
                                props.styled_by(
                                    "rowClass",
                                    TableRow::new(part(props, format!("row-{row_ix}")), row_ix + 2),
                                    cx,
                                ),
                                cx,
                            )
                            .aria_selected(is_selected)
                            .when(is_selected, |row| {
                                props.styled_by("selectedRowClass", row, cx)
                            })
                            .on_click(move |_, _, _| {
                                events.emit("select", json!({ "value": row_ix }))
                            })
                            .children(row.into_iter().enumerate().map(|(ix, text)| {
                                column(
                                    props.styled_by(
                                        "cellClass",
                                        TableCell::new(
                                            part(props, format!("cell-{row_ix}-{ix}")),
                                            ix + 1,
                                        ),
                                        cx,
                                    ),
                                    columns.get(ix),
                                )
                                .child(text)
                            }))
                    })),
            )
            .when_some(caption, |this, caption| {
                this.child(
                    props
                        .styled_by(
                            "captionClass",
                            TableCaption::new(part(props, "caption")),
                            cx,
                        )
                        .child(caption),
                )
            })
            .into_any_element()
    })
    .documentation(
        "GPUI Base's Table: a header row of `columns` over `rows` of text, one cell per column, \
         with an optional `caption` after them. A click on a row selects it (`select`); \
         `selected` marks that row.",
    )
    .prop(
        "columns",
        PropType::Json(
            "{ title: string; width?: number; align?: \"left\" | \"center\" | \"right\" }[]",
        ),
    )
    .prop("rows", PropType::Json("string[][]"))
    .prop("caption", PropType::String)
    .prop("selected", PropType::Number)
    .prop("accessibilityLabel", PropType::String)
    .prop("rootClass", PropType::String)
    .prop("headerClass", PropType::String)
    .prop("headClass", PropType::String)
    .prop("bodyClass", PropType::String)
    .prop("rowClass", PropType::String)
    .states(ROW_STATES)
    .prop("selectedRowClass", PropType::String)
    .prop("cellClass", PropType::String)
    .prop("captionClass", PropType::String)
    .event("select", &[("value", PropType::Number)])
}

const ITEM_STATES: [&str; 3] = ["itemHoverClass", "itemActiveClass", ""];

/// What a list's rows are drawn from. Base's virtual list draws only the
/// rows in view, asking this entity for them as it scrolls.
struct ListRows {
    id: ElementId,
    items: Vec<SharedString>,
    selected: Option<usize>,
    height: f32,
    item: StyleRefinement,
    selected_item: StyleRefinement,
    states: StateStyles,
    events: Option<Events>,
}

impl ListRows {
    fn row(&self, ix: usize) -> AnyElement {
        let is_selected = self.selected == Some(ix);
        let events = self.events.clone();
        let selected_item = self.selected_item.clone();
        let button = Button::new(ElementId::NamedChild(
            Arc::new(self.id.clone()),
            format!("item-{ix}").into(),
        ))
        .refine_style(&self.item)
        .h(px(self.height));
        self.states
            .clone()
            .apply(button)
            .role(Role::ListBoxOption)
            .focusable(false)
            .selected(is_selected)
            .aria_selected(is_selected)
            .styles(|styles| styles.selected(|style| style.refine_style(&selected_item)))
            .child(self.items[ix].clone())
            .on_click(move |_, _, _| {
                if let Some(events) = &events {
                    events.emit("select", json!({ "value": ix }));
                }
            })
            .into_any_element()
    }
}

impl Render for ListRows {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        Empty
    }
}

struct ListState {
    rows: Entity<ListRows>,
    scroll: VirtualListScrollHandle,
    /// Each item's size, kept while the count and the height hold.
    sizes: Rc<Vec<Size<gpui_kit::Pixels>>>,
}

/// The item height when the app sets none.
const DEFAULT_ITEM_HEIGHT: f64 = 32.0;

fn list() -> Control {
    Control::stateful(
        "List",
        |props, _, _, cx| ListState {
            rows: cx.new(|_| ListRows {
                id: props.id(),
                items: Vec::new(),
                height: 0.,
                selected: None,
                item: StyleRefinement::default(),
                selected_item: StyleRefinement::default(),
                states: StateStyles::default(),
                events: None,
            }),
            scroll: VirtualListScrollHandle::new(),
            sizes: Rc::default(),
        },
        |state, props, events, _, cx| {
            let items = props.strings("items");
            let height = Some(props.number("itemHeight"))
                .filter(|height| *height > 0.0)
                .unwrap_or(DEFAULT_ITEM_HEIGHT) as f32;
            let item_size = size(px(0.), px(height));
            if state.sizes.len() != items.len() || state.sizes.first() != Some(&item_size) {
                state.sizes = Rc::new(vec![item_size; items.len()]);
            }
            let style = |name: &str| {
                props
                    .string(name)
                    .map(|class| class_style(&class, cx))
                    .unwrap_or_default()
            };
            let (item, selected_item) = (style("itemClass"), style("selectedItemClass"));
            let states = props.state_styles(ITEM_STATES, cx);
            state.rows.update(cx, |rows, _| {
                rows.items = items;
                rows.height = height;
                rows.selected = index(props, "selected");
                rows.item = item;
                rows.selected_item = selected_item;
                rows.states = states;
                rows.events = Some(events.clone());
            });
            props
                .styled(div().id(props.id()).test_support().role(Role::ListBox), cx)
                .when_some(props.string("accessibilityLabel"), |this, label| {
                    this.aria_label(label)
                })
                .child(
                    v_virtual_list(
                        state.rows.clone(),
                        part(props, "list"),
                        state.sizes.clone(),
                        |rows, range, _, _| range.map(|ix| rows.row(ix)).collect::<Vec<_>>(),
                    )
                    .track_scroll(&state.scroll)
                    .size_full(),
                )
                .into_any_element()
        },
    )
    .documentation(
        "GPUI Base's VirtualList as a list of `items`, each `itemHeight` tall (32 by default). \
         Only the rows in view are drawn, so it holds any number of items, and it scrolls \
         itself: give it a fixed height in `rootClass` (`h_64`), or it is as tall as all its \
         rows. A click on an item selects it (`select`); `selected` marks that item.",
    )
    .prop("items", PropType::Strings)
    .prop("itemHeight", PropType::Number)
    .prop("selected", PropType::Number)
    .prop("accessibilityLabel", PropType::String)
    .prop("rootClass", PropType::String)
    .prop("itemClass", PropType::String)
    .states(ITEM_STATES)
    .prop("selectedItemClass", PropType::String)
    .event("select", &[("value", PropType::Number)])
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use gpui_kit::{ElementId, ScrollDelta, TestAppContext, point, px};
    use serde_json::{Value, json};

    use crate::testing::{Fixture, TestWindowExt as _};

    /// The id of the fixture control's part `name`.
    fn part(fixture: &Fixture, name: &str) -> ElementId {
        ElementId::NamedChild(Arc::new(fixture.id.clone()), name.to_owned().into())
    }

    fn select(value: usize) -> Vec<(String, Value)> {
        vec![("select".into(), json!({ "value": value }))]
    }

    fn columns() -> Value {
        json!([
            { "title": "Name", "width": 120 },
            { "title": "Status", "align": "center" },
            { "title": "Version", "width": 80, "align": "right" },
        ])
    }

    fn rows() -> Value {
        json!([
            ["gpui-base", "Stable", "0.4.1"],
            ["gpui-component", "Active", "0.4.1"],
        ])
    }

    #[gpui_kit::test]
    fn test_table_builder(cx: &mut TestAppContext) {
        let table = Fixture::new(
            cx,
            "Table",
            json!({
                "columns": columns(),
                "rows": rows(),
                "caption": "Crates",
                "selected": 1,
                "accessibilityLabel": "Crates",
                "rootClass": "w_96 border_1 border_border",
                "headerClass": "flex bg_muted",
                "headClass": "flex_1 px_2 py_1 font_medium",
                "bodyClass": "flex flex_col",
                "rowClass": "flex border_t_1 border_border",
                "rowHoverClass": "bg_muted",
                "rowActiveClass": "bg_secondary",
                "selectedRowClass": "bg_accent",
                "cellClass": "flex_1 px_2 py_1",
                "captionClass": "p_2 text_muted_foreground",
            }),
        );
        let size = table.measure(cx);
        assert!(size.width > px(0.) && size.height > px(0.));
        table.update(cx, |window, _| {
            assert_eq!(window.find(table.id.clone()).label(), Some("Crates"));
            assert!(window.try_find(part(&table, "caption")).is_some());
            assert!(window.try_find(part(&table, "head-2")).is_some());
            assert!(window.try_find(part(&table, "cell-1-2")).is_some());
            assert!(window.find(part(&table, "row-1")).selected() == Some(true));
            assert_ne!(window.find(part(&table, "row-0")).selected(), Some(true));
            // A column's width is fixed; a column without one takes its class.
            let name = window.find(part(&table, "cell-0-0")).bounds().size.width;
            let version = window.find(part(&table, "head-2")).bounds().size.width;
            assert_eq!((name, version), (px(120.), px(80.)));
        });
    }

    #[gpui_kit::test]
    fn test_table_click(cx: &mut TestAppContext) {
        let table = Fixture::new(cx, "Table", json!({ "columns": columns(), "rows": rows() }));
        table.update(cx, |window, cx| window.click(part(&table, "row-1"), cx));
        assert_eq!(table.take_events(), select(1));
        table.update(cx, |window, cx| window.click(part(&table, "cell-0-2"), cx));
        assert_eq!(table.take_events(), select(0));
        // The header is no row to select.
        table.update(cx, |window, cx| window.click(part(&table, "head-0"), cx));
        assert!(table.take_events().is_empty());
    }

    #[gpui_kit::test]
    fn test_table_controlled(cx: &mut TestAppContext) {
        let table = Fixture::new(cx, "Table", json!({ "columns": columns(), "rows": rows() }));
        table.update(cx, |window, _| {
            assert_ne!(window.find(part(&table, "row-0")).selected(), Some(true));
            assert!(window.try_find(part(&table, "caption")).is_none());
        });
        table.set_prop(cx, "selected", Some(json!(0)));
        table.set_prop(
            cx,
            "rows",
            Some(json!([["a", "b", "c"], ["d", "e", "f"], ["g", "h", "i"]])),
        );
        table.update(cx, |window, _| {
            assert!(window.find(part(&table, "row-0")).selected() == Some(true));
            assert!(window.try_find(part(&table, "row-2")).is_some());
        });
        table.set_prop(cx, "selected", None);
        table.update(cx, |window, _| {
            assert_ne!(window.find(part(&table, "row-0")).selected(), Some(true));
        });
        assert!(table.take_events().is_empty());
    }

    fn items(count: usize) -> Value {
        json!(
            (0..count)
                .map(|ix| format!("Item {ix}"))
                .collect::<Vec<_>>()
        )
    }

    #[gpui_kit::test]
    fn test_list_builder(cx: &mut TestAppContext) {
        let list = Fixture::new(
            cx,
            "List",
            json!({
                "items": items(3),
                "itemHeight": 24,
                "selected": 2,
                "accessibilityLabel": "Fruit",
                "rootClass": "w_48 h_40 border_1 border_border",
                "itemClass": "flex items_center px_2 w_full",
                "itemHoverClass": "bg_muted",
                "itemActiveClass": "bg_secondary",
                "selectedItemClass": "bg_accent text_accent_foreground",
            }),
        );
        let size = list.measure(cx);
        // The height is the app's, not the rows'.
        assert_eq!(size.height, px(160.));
        assert!(size.width > px(0.));
        list.update(cx, |window, _| {
            assert_eq!(window.find(list.id.clone()).label(), Some("Fruit"));
            let item = window.find(part(&list, "item-2"));
            assert!(item.selected() == Some(true));
            assert_eq!(item.bounds().size.height, px(24.));
            assert_ne!(window.find(part(&list, "item-0")).selected(), Some(true));
        });
    }

    /// Base's virtual list draws only the rows in view, and scrolls itself.
    #[gpui_kit::test]
    fn test_list_draws_only_visible_rows(cx: &mut TestAppContext) {
        let list = Fixture::new(
            cx,
            "List",
            json!({
                "items": items(10_000),
                "rootClass": "w_48 h_40",
                "itemClass": "w_full",
            }),
        );
        list.update(cx, |window, _| {
            assert!(window.try_find(part(&list, "item-0")).is_some());
            assert!(window.try_find(part(&list, "item-4")).is_some());
            assert!(window.try_find(part(&list, "item-20")).is_none());
            assert!(window.try_find(part(&list, "item-9999")).is_none());
        });
        let delta = ScrollDelta::Pixels(point(px(0.), px(-32. * 100.)));
        list.update(cx, |window, cx| {
            window.scroll(part(&list, "item-0"), delta, cx)
        });
        list.update(cx, |window, _| {
            assert!(window.try_find(part(&list, "item-0")).is_none());
            assert!(window.try_find(part(&list, "item-100")).is_some());
        });
    }

    #[gpui_kit::test]
    fn test_list_click(cx: &mut TestAppContext) {
        let list = Fixture::new(
            cx,
            "List",
            json!({ "items": items(3), "rootClass": "w_48 h_40", "itemClass": "w_full" }),
        );
        list.update(cx, |window, cx| window.click(part(&list, "item-1"), cx));
        assert_eq!(list.take_events(), select(1));
        list.update(cx, |window, cx| window.click(part(&list, "item-2"), cx));
        assert_eq!(list.take_events(), select(2));
    }

    #[gpui_kit::test]
    fn test_list_controlled(cx: &mut TestAppContext) {
        let list = Fixture::new(
            cx,
            "List",
            json!({ "items": items(3), "rootClass": "h_40" }),
        );
        list.set_prop(cx, "selected", Some(json!(1)));
        list.set_prop(cx, "items", Some(items(5)));
        list.update(cx, |window, _| {
            assert!(window.find(part(&list, "item-1")).selected() == Some(true));
            assert!(window.try_find(part(&list, "item-4")).is_some());
        });
        list.set_prop(cx, "itemHeight", Some(json!(40)));
        list.update(cx, |window, _| {
            assert_eq!(
                window.find(part(&list, "item-0")).bounds().size.height,
                px(40.)
            );
        });
        assert!(list.take_events().is_empty());
    }
}
