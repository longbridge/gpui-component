//! Pickers opening a popup from a trigger: DatePicker, ColorPicker.

use std::{cell::Cell, rc::Rc, sync::Arc};

use gpui_kit::{
    App, AppContext as _, ElementId, Entity, FocusHandle, Focusable as _, Hsla,
    InteractiveElement as _, IntoElement as _, MouseButton, ParentElement as _, Rgba,
    StatefulInteractiveElement as _, Styled as _, Subscription, Window,
    base::{
        Button, CalendarEvent, CalendarState, ColorPicker, ColorPickerEvent, ColorPickerState,
        ColorSwatch, DatePicker, Popup, TestSupportExt as _, input::InputBase,
    },
    div,
    prelude::FluentBuilder as _,
};
use serde_json::json;

use super::time::{calendar_element, calendar_props, date_value, sync_calendar};
use crate::{Control, PropType, Props};

pub(super) fn controls() -> Vec<Control> {
    vec![date_picker(), color_picker()]
}

/// The id of part `name` of the control `props` belong to.
fn part_id(props: &Props, name: &'static str) -> ElementId {
    ElementId::NamedChild(Arc::new(props.id()), name.into())
}

/// A date picker's calendar and focus, kept across frames.
struct DateField {
    focus: FocusHandle,
    calendar: Entity<CalendarState>,
    _selected: Subscription,
}

fn date_picker() -> Control {
    calendar_props(
        Control::stateful(
            "DatePicker",
            |_, events, window, cx| {
                let calendar = cx.new(|cx| CalendarState::new(window, cx));
                let events = events.clone();
                let selected =
                    cx.subscribe(&calendar, move |_, CalendarEvent::Selected(date), _| {
                        events.emit("change", json!({ "value": date_value(*date) }));
                    });
                DateField {
                    focus: cx.focus_handle(),
                    calendar,
                    _selected: selected,
                }
            },
            |field, props, events, _, cx| {
                sync_calendar(&field.calendar, props, false, cx);
                let value = props.string("value").filter(|value| !value.is_empty());
                let open = props.boolean("open");
                let disabled = props.boolean("disabled");
                let label = props
                    .string("label")
                    .or_else(|| value.clone())
                    .or_else(|| props.string("placeholder"))
                    .unwrap_or_default();
                // The root is the picker's one Tab stop: the trigger leaves focus
                // with it, so Enter and Escape reach Base's handlers.
                let trigger = props
                    .styled_by("triggerClass", Button::new(part_id(props, "trigger")), cx)
                    .tab_stop(false)
                    .focusable(false)
                    .disabled(disabled)
                    .styles(|styles| {
                        styles.disabled(|style| props.styled_by("disabledClass", style, cx))
                    })
                    .on_click(events.handler("openChange", move |_| json!({ "open": !open })))
                    .child(label);
                let popup = Popup::new(part_id(props, "popup"), trigger).when(open, |this| {
                    this.content(
                        props
                            .styled_by(
                                "popupClass",
                                div().id(part_id(props, "content")).test_support(),
                                cx,
                            )
                            .child(calendar_element(
                                part_id(props, "calendar"),
                                &field.calendar,
                                props,
                                cx,
                            )),
                    )
                });
                props
                    .styled(DatePicker::new(props.id(), &field.focus), cx)
                    .open(open)
                    .disabled(disabled)
                    .when_some(props.string("accessibilityLabel"), |this, label| {
                        this.aria_label(label)
                    })
                    .when_some(value, |this, value| this.aria_value(value))
                    .on_open_change({
                        let events = events.clone();
                        move |open, _, _| events.emit("openChange", json!({ "open": open }))
                    })
                    .child(popup)
                    .into_any_element()
            },
        )
        .focus(|field: &DateField, _| field.focus.clone()),
    )
    .documentation(
        "GPUI Base's DatePicker: a trigger opening a popup with Base's Calendar. \
         `value`, `min` and `max` are ISO dates (`2026-08-12`). Clicking the trigger \
         sends `openChange`; choosing a day sends `change` and leaves the popup open \
         until the app sets `open` to false.",
    )
    .prop("open", PropType::Boolean)
    .prop("disabled", PropType::Boolean)
    .prop("label", PropType::String)
    .prop("placeholder", PropType::String)
    .prop("accessibilityLabel", PropType::String)
    .prop("rootClass", PropType::String)
    .prop("triggerClass", PropType::String)
    .prop("disabledClass", PropType::String)
    .prop("popupClass", PropType::String)
    .event("change", &[("value", PropType::String)])
    .event("openChange", &[("open", PropType::Boolean)])
}

/// A hex color, `#rgb`, `#rgba`, `#rrggbb` or `#rrggbbaa`.
fn parse_color(value: &str) -> Option<Hsla> {
    Rgba::try_from(value).ok().map(Hsla::from)
}

/// A color picker's state, kept across frames with the open state last
/// sent to or taken from JS.
struct ColorField {
    state: Entity<ColorPickerState>,
    open: Rc<Cell<bool>>,
    _subscriptions: [Subscription; 2],
}

impl ColorField {
    /// Brings the state up to date with `value` and `open`.
    fn update(&self, props: &Props, window: &mut Window, cx: &mut App) {
        if props.changed("value") {
            let value = props.string("value");
            self.state.update(cx, |state, cx| {
                match value.as_deref().and_then(parse_color) {
                    Some(color) => state.set_value(color, window, cx),
                    None => state.clear_value(window, cx),
                }
            });
        }
        let open = props.boolean("open");
        if props.changed("open") && open != self.open.get() {
            self.open.set(open);
            self.state.update(cx, |state, cx| state.set_open(open, cx));
        }
    }
}

fn color_picker() -> Control {
    Control::stateful(
        "ColorPicker",
        |_, events, window, cx| {
            let state = cx.new(|cx| ColorPickerState::new(window, cx));
            let open = Rc::new(Cell::new(false));
            let change = cx.subscribe(&state, {
                let events = events.clone();
                move |state, _: &ColorPickerEvent, cx| {
                    // Base writes the committed color to its hex field before
                    // it sends the change, in Base's own hex format.
                    let hex = state.read(cx).hex_input().read(cx).value();
                    events.emit("change", json!({ "value": hex }));
                }
            });
            // The open state is Base's: a palette choice closes the picker.
            let open_change = cx.observe(&state, {
                let events = events.clone();
                let open = open.clone();
                move |state, cx| {
                    let is_open = state.read(cx).is_open();
                    if is_open != open.get() {
                        open.set(is_open);
                        events.emit("openChange", json!({ "open": is_open }));
                    }
                }
            });
            ColorField {
                state,
                open,
                _subscriptions: [change, open_change],
            }
        },
        |field, props, _, window, cx| {
            field.update(props, window, cx);
            let state = field.state.clone();
            let picker = state.read(cx);
            let open = picker.is_open();
            let disabled = props.boolean("disabled");
            let committed = picker.value();
            let displayed = picker.displayed_color();
            let hex = picker.hex_input().read(cx).value();
            let label = if hex.is_empty() {
                props.string("placeholder").unwrap_or_default()
            } else {
                hex
            };
            let hex_input = picker.hex_input().clone();
            let hex_focused = hex_input.focus_handle(cx).is_focused(window);
            let focus = picker.focus_handle(cx);

            let trigger = props
                .styled_by("triggerClass", Button::new(part_id(props, "trigger")), cx)
                .tab_stop(false)
                .focusable(false)
                .disabled(disabled)
                .styles(|styles| {
                    styles.disabled(|style| props.styled_by("disabledClass", style, cx))
                })
                .on_click({
                    let state = state.clone();
                    move |_, _, cx| state.update(cx, |state, cx| state.toggle_open(cx))
                })
                .child(
                    props
                        .styled_by("previewClass", div(), cx)
                        .when_some(displayed, |this, color| this.bg(color)),
                )
                .child(label);

            let swatches = props
                .strings("swatches")
                .iter()
                .filter_map(|value| parse_color(value))
                .enumerate()
                .map(|(index, color)| {
                    let selected = committed == Some(color);
                    let hover_state = state.clone();
                    let click_state = state.clone();
                    props
                        .styled_by(
                            "swatchClass",
                            ColorSwatch::new(
                                ElementId::NamedChild(
                                    Arc::new(props.id()),
                                    format!("swatch-{index}").into(),
                                ),
                                color,
                            ),
                            cx,
                        )
                        .selected(selected)
                        // The swatch shows its own color, which no class names.
                        .bg(color)
                        .when(selected, |this| {
                            props.styled_by("swatchSelectedClass", this, cx)
                        })
                        .on_hover(move |color, entered, window, cx| {
                            hover_state.update(cx, |state, cx| {
                                if entered {
                                    state.preview_color(color, window, cx);
                                } else {
                                    state.clear_preview(window, cx);
                                }
                            });
                        })
                        .on_click(move |color, _, window, cx| {
                            click_state
                                .update(cx, |state, cx| state.select_color(color, window, cx));
                        })
                })
                .collect::<Vec<_>>();

            let input = props
                .styled_by("inputClass", InputBase::new(part_id(props, "input")), cx)
                .focused(hex_focused)
                .styles(|styles| {
                    styles.focused(|style| props.styled_by("inputFocusedClass", style, cx))
                })
                .on_mouse_down(MouseButton::Left, {
                    let hex_input = hex_input.clone();
                    move |_, window, cx| {
                        hex_input.update(cx, |input, cx| input.focus(window, cx));
                    }
                })
                .child(hex_input);

            let popup = Popup::new(part_id(props, "popup"), trigger).when(open, |this| {
                this.content(
                    props
                        .styled_by(
                            "popupClass",
                            div().id(part_id(props, "content")).test_support(),
                            cx,
                        )
                        .child(
                            props
                                .styled_by("swatchesClass", div(), cx)
                                .children(swatches),
                        )
                        .child(input),
                )
            });

            props
                .styled(ColorPicker::new(props.id()), cx)
                .open(open)
                .disabled(disabled)
                .track_focus(&focus)
                .when_some(props.string("accessibilityLabel"), |this, label| {
                    this.accessibility_label(label)
                })
                .on_open_change(move |open, _, cx| {
                    state.update(cx, |state, cx| state.set_open(open, cx));
                })
                .child(popup)
                .into_any_element()
        },
    )
    .focus(|field: &ColorField, cx| field.state.focus_handle(cx))
    .documentation(
        "GPUI Base's ColorPicker: a trigger opening a popup of `swatches` and a hex \
         field. `value` and `swatches` are hex colors; `change` sends Base's \
         `#RRGGBB` or `#RRGGBBAA`. A swatch or Enter in the hex field commits and closes.",
    )
    .prop("value", PropType::String)
    .prop("swatches", PropType::Strings)
    .prop("open", PropType::Boolean)
    .prop("disabled", PropType::Boolean)
    .prop("placeholder", PropType::String)
    .prop("accessibilityLabel", PropType::String)
    .prop("rootClass", PropType::String)
    .prop("triggerClass", PropType::String)
    .prop("disabledClass", PropType::String)
    .prop("previewClass", PropType::String)
    .prop("popupClass", PropType::String)
    .prop("swatchesClass", PropType::String)
    .prop("swatchClass", PropType::String)
    .prop("swatchSelectedClass", PropType::String)
    .prop("inputClass", PropType::String)
    .prop("inputFocusedClass", PropType::String)
    .event("change", &[("value", PropType::String)])
    .event("openChange", &[("open", PropType::Boolean)])
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use gpui_kit::{ElementId, TestAppContext, point, px};
    use serde_json::json;

    use crate::testing::{Fixture, TestWindowExt as _};

    fn part(fixture: &Fixture, name: &'static str) -> ElementId {
        ElementId::NamedChild(Arc::new(fixture.id.clone()), name.into())
    }

    /// The id Base's Calendar gives the day `date` in week `week` of the
    /// first month shown.
    fn day(date: &str, week: usize) -> ElementId {
        ElementId::Name(format!("calendar-{date}-0-{week}").into())
    }

    #[gpui_kit::test]
    fn test_date_picker_builder(cx: &mut TestAppContext) {
        let picker = Fixture::new(
            cx,
            "DatePicker",
            json!({
                "value": "2026-08-12",
                "min": "2026-08-01",
                "max": "2026-08-31",
                "open": true,
                "label": "Aug 12, 2026",
                "placeholder": "Pick a date",
                "monthNames": ["Jan", "Feb", "Mar", "Apr", "May", "Jun",
                               "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"],
                "weekdayNames": ["Su", "Mo", "Tu", "We", "Th", "Fr", "Sa"],
                "accessibilityLabel": "Start date",
                "rootClass": "w_64",
                "triggerClass": "h_7 px_3 flex items_center border_1 border_border",
                "disabledClass": "opacity_50",
                "popupClass": "p_3 bg_popover border_1 border_border rounded_md",
                "calendarClass": "gap_2",
                "navClass": "size_7",
                "headingClass": "px_1 h_7",
                "weekdayClass": "size_8 text_muted_foreground",
                "dayClass": "size_8 flex items_center justify_center",
                "cellClass": "h_7",
                "activeClass": "bg_primary text_primary_foreground",
                "todayClass": "border_1",
                "mutedClass": "text_muted_foreground",
                "inRangeClass": "bg_accent",
                "itemDisabledClass": "opacity_50",
                "hoverClass": "bg_accent",
            }),
        );
        let size = picker.measure(cx);
        assert!(size.width > px(0.) && size.height > px(0.));
        picker.update(cx, |window, _| {
            let root = window.find(picker.id.clone());
            assert_eq!(root.label(), Some("Start date"));
            assert_eq!(root.value(), Some("2026-08-12"));
            assert_eq!(root.expanded(), Some(true));
            // The popup opens below the trigger, in the window's overlay.
            let trigger = window.find(part(&picker, "trigger")).bounds();
            let content = window.find(part(&picker, "content")).bounds();
            assert!(content.origin.y >= trigger.bottom());
            assert_eq!(content.origin.x, trigger.origin.x);
            assert_eq!(
                window.find(day("2026-08-12", 2)).label(),
                Some("2026-08-12")
            );
        });
    }

    #[gpui_kit::test]
    fn test_date_picker_choose(cx: &mut TestAppContext) {
        let picker = Fixture::new(cx, "DatePicker", json!({ "value": "2026-08-12" }));
        picker.update(cx, |window, cx| window.click(part(&picker, "trigger"), cx));
        assert_eq!(
            picker.take_events(),
            vec![("openChange".into(), json!({ "open": true }))]
        );
        picker.update(cx, |window, cx| window.click(day("2026-08-20", 3), cx));
        assert_eq!(
            picker.take_events(),
            vec![("change".into(), json!({ "value": "2026-08-20" }))]
        );
        picker.update(cx, |window, cx| window.click(part(&picker, "trigger"), cx));
        assert_eq!(
            picker.take_events(),
            vec![("openChange".into(), json!({ "open": false }))]
        );
        picker.update(cx, |window, _| {
            assert!(window.try_find(part(&picker, "content")).is_none());
        });
    }

    #[gpui_kit::test]
    fn test_date_picker_controlled(cx: &mut TestAppContext) {
        let picker = Fixture::new(
            cx,
            "DatePicker",
            json!({ "value": "2026-08-12", "open": true }),
        );
        picker.set_prop(cx, "value", Some(json!("2026-08-03")));
        picker.update(cx, |window, _| {
            assert_eq!(window.find(picker.id.clone()).value(), Some("2026-08-03"));
            assert!(window.try_find(day("2026-08-03", 1)).is_some());
        });
        // Days outside `min`..`max` cannot be chosen.
        picker.set_prop(cx, "min", Some(json!("2026-08-10")));
        picker.update(cx, |window, cx| window.click(day("2026-08-03", 1), cx));
        assert!(picker.take_events().is_empty());
        picker.set_prop(cx, "open", Some(json!(false)));
        picker.update(cx, |window, _| {
            assert!(window.try_find(part(&picker, "content")).is_none());
        });
    }

    #[gpui_kit::test]
    fn test_date_picker_disabled(cx: &mut TestAppContext) {
        let picker = Fixture::new(cx, "DatePicker", json!({ "disabled": true }));
        picker.update(cx, |window, cx| window.click(part(&picker, "trigger"), cx));
        assert!(picker.take_events().is_empty());
        picker.set_prop(cx, "disabled", None);
        picker.update(cx, |window, cx| window.click(part(&picker, "trigger"), cx));
        assert_eq!(picker.take_events().len(), 1);
    }

    /// Swatches 24px wide in a row at the popup's top left.
    const SWATCHES: &str = "flex";
    const SWATCH: &str = "size_6";

    fn swatch(index: usize) -> gpui_kit::Point<gpui_kit::Pixels> {
        point(px(12. + 24. * index as f32), px(12.))
    }

    #[gpui_kit::test]
    fn test_color_picker_builder(cx: &mut TestAppContext) {
        let picker = Fixture::new(
            cx,
            "ColorPicker",
            json!({
                "value": "#2563eb",
                "swatches": ["#dc2626", "#2563eb"],
                "open": true,
                "placeholder": "No color",
                "accessibilityLabel": "Brand color",
                "rootClass": "w_56",
                "triggerClass": "h_7 px_2 flex items_center gap_2 border_1 border_border",
                "disabledClass": "opacity_50",
                "previewClass": "size_4 border_1 border_border",
                "popupClass": "flex flex_col gap_2 bg_popover",
                "swatchesClass": SWATCHES,
                "swatchClass": SWATCH,
                "swatchSelectedClass": "border_2 border_ring",
                "inputClass": "h_7 px_2 border_1 border_input",
                "inputFocusedClass": "border_ring",
            }),
        );
        let size = picker.measure(cx);
        assert!(size.width > px(0.) && size.height > px(0.));
        picker.update(cx, |window, _| {
            let trigger = window.find(part(&picker, "trigger")).bounds();
            let content = window.find(part(&picker, "content")).bounds();
            assert!(content.origin.y >= trigger.bottom());
            assert_eq!(content.origin.x, trigger.origin.x);
            assert_eq!(
                content.size.height,
                px(24.) + px(8.) + window.find(part(&picker, "input")).bounds().size.height
            );
        });
    }

    #[gpui_kit::test]
    fn test_color_picker_choose(cx: &mut TestAppContext) {
        let picker = Fixture::new(
            cx,
            "ColorPicker",
            json!({
                "swatches": ["#dc2626", "#2563eb"],
                "swatchesClass": SWATCHES,
                "swatchClass": SWATCH,
            }),
        );
        picker.update(cx, |window, cx| window.click(part(&picker, "trigger"), cx));
        assert_eq!(
            picker.take_events(),
            vec![("openChange".into(), json!({ "open": true }))]
        );
        picker.update(cx, |window, cx| {
            window.click_at(part(&picker, "content"), swatch(1), cx)
        });
        // A palette choice commits and closes, as Base's state does.
        assert_eq!(
            picker.take_events(),
            vec![
                ("openChange".into(), json!({ "open": false })),
                ("change".into(), json!({ "value": "#2563EB" })),
            ]
        );
        picker.update(cx, |window, _| {
            assert!(window.try_find(part(&picker, "content")).is_none());
        });
    }

    /// A hex field wide enough to click.
    const INPUT: &str = "w_32 h_7";

    #[gpui_kit::test]
    fn test_color_picker_hex_field(cx: &mut TestAppContext) {
        let picker = Fixture::new(
            cx,
            "ColorPicker",
            json!({ "open": true, "inputClass": INPUT }),
        );
        picker.update(cx, |window, cx| window.click(part(&picker, "input"), cx));
        picker.update(cx, |window, cx| {
            window.input("#00ff00", cx);
            window.press("enter", cx);
        });
        assert_eq!(
            picker.take_events(),
            vec![
                ("openChange".into(), json!({ "open": false })),
                ("change".into(), json!({ "value": "#00FF00" })),
            ]
        );
    }

    #[gpui_kit::test]
    fn test_color_picker_controlled(cx: &mut TestAppContext) {
        let picker = Fixture::new(
            cx,
            "ColorPicker",
            json!({ "value": "#dc2626", "open": true, "inputClass": INPUT }),
        );
        picker.set_prop(cx, "value", Some(json!("#2563eb")));
        // Props set from JS are not sent back.
        assert!(picker.take_events().is_empty());
        // The hex field shows the value set, which Enter commits.
        picker.update(cx, |window, cx| window.click(part(&picker, "input"), cx));
        picker.update(cx, |window, cx| window.press("enter", cx));
        assert_eq!(
            picker.take_events(),
            vec![
                ("openChange".into(), json!({ "open": false })),
                ("change".into(), json!({ "value": "#2563EB" })),
            ]
        );
        picker.set_prop(cx, "open", Some(json!(true)));
        picker.update(cx, |window, _| {
            assert!(window.try_find(part(&picker, "content")).is_some());
        });
        picker.set_prop(cx, "open", Some(json!(false)));
        picker.update(cx, |window, _| {
            assert!(window.try_find(part(&picker, "content")).is_none());
        });
        assert!(picker.take_events().is_empty());
    }

    #[gpui_kit::test]
    fn test_color_picker_keyboard(cx: &mut TestAppContext) {
        let picker = Fixture::new(cx, "ColorPicker", json!({}));
        let focus = picker.focus_handle(cx).expect("a Tab stop");
        picker.update(cx, |window, cx| {
            focus.focus(window, cx);
            window.press("enter", cx);
        });
        assert_eq!(
            picker.take_events(),
            vec![("openChange".into(), json!({ "open": true }))]
        );
        picker.update(cx, |window, cx| window.press("escape", cx));
        assert_eq!(
            picker.take_events(),
            vec![("openChange".into(), json!({ "open": false }))]
        );
    }

    #[gpui_kit::test]
    fn test_color_picker_disabled(cx: &mut TestAppContext) {
        let picker = Fixture::new(cx, "ColorPicker", json!({ "disabled": true }));
        picker.update(cx, |window, cx| window.click(part(&picker, "trigger"), cx));
        assert!(picker.take_events().is_empty());
        picker.set_prop(cx, "disabled", None);
        picker.update(cx, |window, cx| window.click(part(&picker, "trigger"), cx));
        assert_eq!(
            picker.take_events(),
            vec![("openChange".into(), json!({ "open": true }))]
        );
    }
}
