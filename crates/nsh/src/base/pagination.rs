//! Page navigation: Pagination.

use gpui_kit::{
    ElementId, IntoElement as _, ParentElement as _,
    base::{Button, Pagination, PaginationItem, PaginationState},
    div,
    prelude::FluentBuilder as _,
};
use serde_json::json;

use crate::{Control, PropType};

pub(super) fn controls() -> Vec<Control> {
    vec![pagination()]
}

/// The interaction-state class props of every page, Previous and Next
/// button.
const BUTTON_STATES: [&str; 3] = ["buttonHoverClass", "buttonActiveClass", "buttonFocusClass"];

fn pagination() -> Control {
    Control::new("Pagination", |props, events, _, cx| {
        let mut state = PaginationState::new(
            props.number("value") as usize,
            props.number("totalPages") as usize,
        );
        if props.has("visiblePages") {
            state = state.visible_pages(props.number("visiblePages") as usize);
        }
        let state = state.disabled(props.boolean("disabled")).on_change({
            let events = events.clone();
            move |page, _, _| events.emit("change", json!({ "value": page }))
        });
        let button = |id: ElementId, class: &str, target: Option<usize>| {
            let state = state.clone();
            props
                .styled_by(class, Button::new(id), cx)
                // A button with nowhere to go is disabled: no states.
                .when(target.is_some(), |this| {
                    props.states(BUTTON_STATES, this, cx)
                })
                .disabled(target.is_none())
                .styles(|styles| {
                    styles.disabled(|style| props.styled_by("disabledClass", style, cx))
                })
                .when_some(target, |this, page| {
                    this.on_click(move |_, window, cx| state.request_page(page, window, cx))
                })
        };

        props
            .styled(Pagination::new(props.id(), state.clone()), cx)
            .when_some(props.string("accessibilityLabel"), |this, label| {
                this.accessibility_label(label)
            })
            .when_some(props.string("previousLabel"), |this, label| {
                this.child(
                    button("previous".into(), "previousClass", state.previous_page()).child(label),
                )
            })
            .children(state.items().into_iter().map(|item| {
                match item {
                    PaginationItem::Page(page) => button(
                        ElementId::named_usize("page", page),
                        "pageClass",
                        (!state.is_disabled()).then_some(page),
                    )
                    .selected(page == state.current_page())
                    .styles(|styles| {
                        styles.selected(|style| props.styled_by("currentPageClass", style, cx))
                    })
                    .child(page.to_string())
                    .into_any_element(),
                    PaginationItem::Ellipsis(_) => props
                        .styled_by("ellipsisClass", div(), cx)
                        .child(props.string("ellipsisLabel").unwrap_or("…".into()))
                        .into_any_element(),
                }
            }))
            .when_some(props.string("nextLabel"), |this, label| {
                this.child(button("next".into(), "nextClass", state.next_page()).child(label))
            })
            .into_any_element()
    })
    .documentation(
        "GPUI Base's Pagination: a navigation landmark of page buttons, with an ellipsis for \
         each run of hidden pages. Previous and Next show only when their labels are set.",
    )
    .prop("value", PropType::Number)
    .prop("totalPages", PropType::Number)
    .prop("visiblePages", PropType::Number)
    .prop("disabled", PropType::Boolean)
    .prop("accessibilityLabel", PropType::String)
    .prop("previousLabel", PropType::String)
    .prop("nextLabel", PropType::String)
    .prop("ellipsisLabel", PropType::String)
    .prop("rootClass", PropType::String)
    .prop("pageClass", PropType::String)
    .prop("currentPageClass", PropType::String)
    .prop("previousClass", PropType::String)
    .prop("nextClass", PropType::String)
    .states(BUTTON_STATES)
    .prop("ellipsisClass", PropType::String)
    .prop("disabledClass", PropType::String)
    .event("change", &[("value", PropType::Number)])
}

#[cfg(test)]
mod tests {
    use gpui_kit::{ElementId, TestAppContext};
    use serde_json::json;

    use crate::testing::{Fixture, TestWindowExt as _};

    fn page(page: usize) -> ElementId {
        ElementId::named_usize("page", page)
    }

    fn change(page: usize) -> Vec<(String, serde_json::Value)> {
        vec![("change".into(), json!({ "value": page }))]
    }

    #[gpui_kit::test]
    fn test_pagination_builder(cx: &mut TestAppContext) {
        let pagination = Fixture::new(
            cx,
            "Pagination",
            json!({
                "value": 3,
                "totalPages": 5,
                "visiblePages": 7,
                "accessibilityLabel": "Results pages",
                "previousLabel": "Previous",
                "nextLabel": "Next",
                "ellipsisLabel": "...",
                "rootClass": "flex items_center gap_1",
                "pageClass": "px_2 rounded_md",
                "currentPageClass": "bg_primary text_primary_foreground",
                "previousClass": "px_2",
                "nextClass": "px_2",
                "ellipsisClass": "px_1 text_muted_foreground",
                "disabledClass": "opacity_50",
            }),
        );
        let size = pagination.measure(cx);
        assert!(size.width > gpui_kit::px(0.) && size.height > gpui_kit::px(0.));
        pagination.update(cx, |window, _| {
            let previous = window.find("previous").bounds();
            let next = window.find("next").bounds();
            assert!(previous.origin.x < window.find(page(1)).bounds().origin.x);
            assert!(window.find(page(5)).bounds().origin.x < next.origin.x);
        });
    }

    #[gpui_kit::test]
    fn test_pagination_click(cx: &mut TestAppContext) {
        let pagination = Fixture::new(
            cx,
            "Pagination",
            json!({ "value": 3, "totalPages": 5, "previousLabel": "Previous", "nextLabel": "Next" }),
        );
        pagination.update(cx, |window, cx| window.click(page(5), cx));
        assert_eq!(pagination.take_events(), change(5));
        pagination.update(cx, |window, cx| window.click("previous", cx));
        assert_eq!(pagination.take_events(), change(4));
        pagination.update(cx, |window, cx| window.click("next", cx));
        assert_eq!(pagination.take_events(), change(5));

        // The current page is not a change.
        pagination.update(cx, |window, cx| window.click(page(5), cx));
        assert!(pagination.take_events().is_empty());
    }

    #[gpui_kit::test]
    fn test_pagination_boundaries(cx: &mut TestAppContext) {
        let pagination = Fixture::new(
            cx,
            "Pagination",
            json!({ "value": 1, "totalPages": 3, "previousLabel": "Previous", "nextLabel": "Next" }),
        );
        pagination.update(cx, |window, cx| window.click("previous", cx));
        assert!(pagination.take_events().is_empty());
        pagination.update(cx, |window, cx| window.click("next", cx));
        assert_eq!(pagination.take_events(), change(2));

        pagination.set_prop(cx, "value", Some(json!(3)));
        pagination.update(cx, |window, cx| window.click("next", cx));
        assert!(pagination.take_events().is_empty());
        pagination.update(cx, |window, cx| window.click("previous", cx));
        assert_eq!(pagination.take_events(), change(2));
    }

    #[gpui_kit::test]
    fn test_pagination_ellipsis(cx: &mut TestAppContext) {
        let pagination = Fixture::new(cx, "Pagination", json!({ "value": 10, "totalPages": 20 }));
        pagination.update(cx, |window, _| {
            for ix in [1, 9, 10, 11, 20] {
                assert!(window.try_find(page(ix)).is_some(), "page {ix} is shown");
            }
            for ix in [2, 8, 12, 19] {
                assert!(window.try_find(page(ix)).is_none(), "page {ix} is hidden");
            }
            assert!(window.try_find("previous").is_none());
            assert!(window.try_find("next").is_none());
        });
    }

    #[gpui_kit::test]
    fn test_pagination_controlled(cx: &mut TestAppContext) {
        let pagination = Fixture::new(cx, "Pagination", json!({ "value": 1, "totalPages": 20 }));
        pagination.update(cx, |window, _| {
            assert!(window.try_find(page(3)).is_some());
            assert!(window.try_find(page(10)).is_none());
        });

        pagination.set_prop(cx, "value", Some(json!(10)));
        pagination.update(cx, |window, cx| {
            assert!(window.try_find(page(3)).is_none());
            window.click(page(10), cx);
        });
        assert!(pagination.take_events().is_empty());

        pagination.set_prop(cx, "totalPages", Some(json!(4)));
        pagination.update(cx, |window, _| {
            assert!(window.try_find(page(3)).is_some());
            assert!(window.try_find(page(5)).is_none());
        });
    }

    #[gpui_kit::test]
    fn test_pagination_disabled(cx: &mut TestAppContext) {
        let pagination = Fixture::new(
            cx,
            "Pagination",
            json!({ "value": 2, "totalPages": 3, "disabled": true, "nextLabel": "Next" }),
        );
        pagination.update(cx, |window, cx| {
            window.click(page(3), cx);
            window.click("next", cx);
        });
        assert!(pagination.take_events().is_empty());

        pagination.set_prop(cx, "disabled", None);
        pagination.update(cx, |window, cx| window.click(page(3), cx));
        assert_eq!(pagination.take_events(), change(3));
    }
}
