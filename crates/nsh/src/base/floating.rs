//! Controls whose content floats beside a trigger: Popover, HoverCard,
//! Tooltip.

use std::{cell::RefCell, rc::Rc, sync::Arc, time::Duration};

use gpui_kit::{
    Anchor, AnyView, App, AppContext as _, Bounds, Context, Div, ElementId, Entity,
    InteractiveElement as _, IntoElement, MouseButton, ParentElement, Pixels, Render, SharedString,
    Stateful, StatefulInteractiveElement as _, StyleRefinement, Styled as _, Window,
    base::{
        Button, ElementExt as _, HoverCard, ObservedElement, Placement, Popover, PopupArrow,
        StyledExt as _, TestSupportExt as _, Tooltip, TooltipOverlay, TooltipRequest,
    },
    div,
    prelude::FluentBuilder as _,
    px,
};
use serde_json::json;

use crate::{Control, Events, PropType, Props};

pub(super) fn controls() -> Vec<Control> {
    vec![popover(), hover_card(), tooltip()]
}

/// The id of a control's part named `name`.
fn part(props: &Props, name: &str) -> ElementId {
    ElementId::NamedChild(Arc::new(props.id()), SharedString::from(name.to_owned()))
}

/// The values of the `placement` prop: the trigger's side the content shows on.
const PLACEMENTS: &[&str] = &["bottom", "top", "left", "right"];

/// The values of the `align` prop: how the content lines up with the trigger
/// along that side.
const ALIGNS: &[&str] = &["center", "start", "end"];

/// Base's anchor for the `placement` and `align` props, `None` when the app
/// set neither (Base's default stands). A side placement is centered: Base
/// anchors a popup beside its trigger only at the middle.
fn anchor(props: &Props) -> Option<Anchor> {
    if !props.has("placement") && !props.has("align") {
        return None;
    }
    let placement = props.string("placement");
    let align = props.string("align");
    Some(match (placement.as_deref(), align.as_deref()) {
        (Some("top"), Some("start")) => Anchor::BottomLeft,
        (Some("top"), Some("end")) => Anchor::BottomRight,
        (Some("top"), _) => Anchor::BottomCenter,
        (Some("left"), _) => Anchor::RightCenter,
        (Some("right"), _) => Anchor::LeftCenter,
        (_, Some("start")) => Anchor::TopLeft,
        (_, Some("end")) => Anchor::TopRight,
        _ => Anchor::TopCenter,
    })
}

/// A duration prop in milliseconds, when the app set it.
fn millis(props: &Props, name: &str) -> Option<Duration> {
    props
        .has(name)
        .then(|| Duration::from_secs_f64(props.number(name).max(0.0) / 1000.0))
}

/// The `title` and `description` texts of a floating surface, each shown
/// only when set.
fn texts<E: ParentElement + IntoElement>(props: &Props, element: E, cx: &App) -> E {
    element
        .when_some(
            props.string("title").filter(|title| !title.is_empty()),
            |this, title| {
                this.child(props.styled_by(
                    "titleClass",
                    div().id(part(props, "title")).test_support().child(title),
                    cx,
                ))
            },
        )
        .when_some(
            props
                .string("description")
                .filter(|description| !description.is_empty()),
            |this, description| {
                this.child(
                    props.styled_by(
                        "descriptionClass",
                        div()
                            .id(part(props, "description"))
                            .test_support()
                            .child(description),
                        cx,
                    ),
                )
            },
        )
}

/// Reports Base's open changes as `openChange`.
fn on_open_change(events: &Events) -> impl Fn(&bool, &mut Window, &mut App) + 'static {
    events.handler("openChange", |open: &bool| json!({ "open": open }))
}

/// The arrow of the `arrowSize` prop (its depth), styled by `arrowClass`
/// (its background fills it, its border color strokes it), its base clear of
/// `arrowCornerRadius`; `None` when the app set no size.
fn arrow(props: &Props, cx: &App) -> Option<PopupArrow> {
    props.has("arrowSize").then(|| {
        props.styled_by(
            "arrowClass",
            PopupArrow::new(px(props.number("arrowSize") as f32))
                .corner_radius(px(props.number("arrowCornerRadius") as f32)),
            cx,
        )
    })
}

/// The interaction-state class props of a trigger.
const TRIGGER_STATES: [&str; 3] = [
    "triggerHoverClass",
    "triggerActiveClass",
    "triggerFocusClass",
];

/// A trigger that takes no focus: hover and press states only.
const POINTER_TRIGGER_STATES: [&str; 3] = ["triggerHoverClass", "triggerActiveClass", ""];

/// A trigger showing the `label` prop, which only the pointer drives.
fn label_trigger(props: &Props, cx: &App) -> ObservedElement<Stateful<Div>> {
    props
        .states(
            POINTER_TRIGGER_STATES,
            props.styled_by(
                "triggerClass",
                div().id(part(props, "trigger")).test_support(),
                cx,
            ),
            cx,
        )
        .when_some(props.string("label"), |this, label| this.child(label))
}

/// The control's root, styled by `rootClass`. It lays its trigger out at the
/// trigger's own size (a flex row), so Base anchors the content to the
/// trigger rather than to the width of the app's box.
fn root(props: &Props, cx: &App) -> Div {
    props.styled(div().flex(), cx)
}

fn popover() -> Control {
    Control::focusable("Popover", |focus, props, events, _, cx| {
        let open = props.boolean("open");
        let disabled = props.boolean("disabled");
        let trigger = props
            .states(
                TRIGGER_STATES,
                props.styled_by("triggerClass", Button::new(part(props, "trigger")), cx),
                cx,
            )
            .when(open, |this| props.styled_by("triggerOpenClass", this, cx))
            .track_focus(focus)
            .disabled(disabled)
            .styles(|styles| styles.disabled(|style| props.styled_by("disabledClass", style, cx)))
            .when_some(props.string("label"), |this, label| this.child(label));
        let content = texts(
            props,
            props.styled_by(
                "contentClass",
                div().id(part(props, "content")).test_support(),
                cx,
            ),
            cx,
        )
        .into_any_element();
        root(props, cx)
            .child(
                Popover::new(props.id())
                    .when_some(anchor(props), |this, anchor| this.anchor(anchor))
                    .when(props.has("offset"), |this| {
                        this.offset(px(props.number("offset") as f32))
                    })
                    .open(open)
                    .disabled(disabled)
                    .when_some(arrow(props, cx), |this, arrow| this.arrow(arrow))
                    .on_open_change(on_open_change(events))
                    .trigger(trigger)
                    .content(move |_, _, _| content),
            )
            .into_any_element()
    })
    .documentation(
        "GPUI Base's Popover: a trigger showing `label` that opens a non-modal surface with \
         `title` and `description` beside it. A click, Enter or Space on the trigger opens or \
         closes it; Escape or a click outside closes it and gives focus back to the trigger.",
    )
    .prop("label", PropType::String)
    .prop("title", PropType::String)
    .prop("description", PropType::String)
    .prop("open", PropType::Boolean)
    .prop("disabled", PropType::Boolean)
    .prop("placement", PropType::Enum(PLACEMENTS))
    .prop("align", PropType::Enum(ALIGNS))
    .prop("offset", PropType::Number)
    .prop("arrowSize", PropType::Number)
    .prop("arrowCornerRadius", PropType::Number)
    .prop("rootClass", PropType::String)
    .prop("triggerClass", PropType::String)
    .states(TRIGGER_STATES)
    .prop("triggerOpenClass", PropType::String)
    .prop("contentClass", PropType::String)
    .prop("titleClass", PropType::String)
    .prop("descriptionClass", PropType::String)
    .prop("arrowClass", PropType::String)
    .prop("disabledClass", PropType::String)
    .event("openChange", &[("open", PropType::Boolean)])
}

fn hover_card() -> Control {
    Control::new("HoverCard", |props, events, _, cx| {
        // Base's HoverCard hovers on a plain stateful div, which wraps the
        // styled surface.
        let content = div().id(part(props, "card")).child(texts(
            props,
            props.styled_by(
                "contentClass",
                div().id(part(props, "content")).test_support(),
                cx,
            ),
            cx,
        ));
        root(props, cx)
            .child(
                HoverCard::new(props.id())
                    .when_some(anchor(props), |this, anchor| this.anchor(anchor))
                    .when_some(millis(props, "openDelay"), |this, delay| {
                        this.open_delay(delay)
                    })
                    .when_some(millis(props, "closeDelay"), |this, delay| {
                        this.close_delay(delay)
                    })
                    .when(props.has("offset"), |this| {
                        this.offset(px(props.number("offset") as f32))
                    })
                    .when(props.has("open"), |this| this.open(props.boolean("open")))
                    .when_some(arrow(props, cx), |this, arrow| this.arrow(arrow))
                    .on_open_change(on_open_change(events))
                    .trigger(label_trigger(props, cx))
                    .content(move |_, _, _| content),
            )
            .into_any_element()
    })
    .documentation(
        "GPUI Base's HoverCard: a trigger showing `label` whose card of `title` and \
         `description` opens after the pointer rests on it for `openDelay` ms, and closes \
         `closeDelay` ms after the pointer leaves both. Setting `open` controls it: the card          asks with `openChange` and shows `open`.",
    )
    .prop("label", PropType::String)
    .prop("title", PropType::String)
    .prop("description", PropType::String)
    .prop("open", PropType::Boolean)
    .prop("placement", PropType::Enum(PLACEMENTS))
    .prop("align", PropType::Enum(ALIGNS))
    .prop("offset", PropType::Number)
    .prop("openDelay", PropType::Number)
    .prop("closeDelay", PropType::Number)
    .prop("arrowSize", PropType::Number)
    .prop("arrowCornerRadius", PropType::Number)
    .prop("rootClass", PropType::String)
    .prop("triggerClass", PropType::String)
    .states(POINTER_TRIGGER_STATES)
    .prop("contentClass", PropType::String)
    .prop("titleClass", PropType::String)
    .prop("descriptionClass", PropType::String)
    .prop("arrowClass", PropType::String)
    .event("openChange", &[("open", PropType::Boolean)])
}

/// What a tooltip shows, as of the last frame: its overlay builds the
/// popup from it whenever it draws.
#[derive(Clone, Default)]
struct TooltipText {
    id: Option<ElementId>,
    text: SharedString,
    style: StyleRefinement,
}

/// The tooltip popup: Base's `Tooltip` with the app's text and class.
struct TooltipView(TooltipText);

impl Render for TooltipView {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let TooltipText { id, text, style } = self.0.clone();
        // An unstyled wrapper, as large as the tooltip, observed by tests.
        div()
            .id(id.unwrap_or_else(|| "tooltip".into()))
            .test_support()
            .child(Tooltip::new("tooltip").refine_style(&style).child(text))
    }
}

/// A tooltip's overlay, Base's per-window tooltip host kept for this
/// trigger, and what it shows.
struct TooltipState {
    overlay: Entity<TooltipOverlay>,
    text: Rc<RefCell<TooltipText>>,
    trigger: Rc<RefCell<Bounds<Pixels>>>,
}

/// The `placement` prop as Base's [`Placement`], when the app set it.
fn placement(props: &Props) -> Option<Placement> {
    match props.string("placement")?.as_ref() {
        "top" => Some(Placement::Top),
        "left" => Some(Placement::Left),
        "right" => Some(Placement::Right),
        _ => Some(Placement::Bottom),
    }
}

fn tooltip() -> Control {
    Control::stateful(
        "Tooltip",
        |_, _, _, cx| TooltipState {
            overlay: cx.new(|_| TooltipOverlay::new()),
            text: Rc::default(),
            trigger: Rc::default(),
        },
        |state, props, events, _, cx| {
            *state.text.borrow_mut() = TooltipText {
                id: Some(part(props, "tooltip")),
                text: props.string("content").unwrap_or_default(),
                style: props.styled_by("contentClass", StyleRefinement::default(), cx),
            };
            let trigger_bounds = state.trigger.clone();
            let request = {
                let bounds = state.trigger.clone();
                let text = state.text.clone();
                let placement = placement(props);
                let delay = millis(props, "delay");
                let events = events.clone();
                move || {
                    let request = TooltipRequest::new(*bounds.borrow(), {
                        let text = text.clone();
                        move |_, cx: &mut App| -> AnyView {
                            let text = text.borrow().clone();
                            cx.new(|_| TooltipView(text)).into()
                        }
                    });
                    let request = match placement {
                        Some(placement) => request.with_placement(placement),
                        None => request,
                    };
                    let events = events.clone();
                    match delay {
                        Some(delay) => request.with_show_delay(delay),
                        None => request,
                    }
                    .with_on_open_change(move |open, _| {
                        events.emit("openChange", json!({ "open": open }))
                    })
                }
            };
            let hover = state.overlay.clone();
            let press = state.overlay.clone();
            root(props, cx)
                .child(
                    label_trigger(props, cx)
                        // The trigger's bounds, which the tooltip is placed beside.
                        .on_prepaint(move |bounds, _, _| *trigger_bounds.borrow_mut() = bounds)
                        .on_hover(move |hovered, window, cx| {
                            hover.update(cx, |overlay, cx| {
                                if *hovered {
                                    overlay.request_show(request(), window, cx);
                                } else {
                                    overlay.request_hide(window, cx);
                                }
                            })
                        })
                        // A press dismisses the tooltip, as GPUI Kit's do.
                        .on_mouse_down(MouseButton::Left, move |_, _, cx| {
                            press.update(cx, |overlay, cx| overlay.hide(cx))
                        }),
                )
                .child(state.overlay.clone())
                .into_any_element()
        },
    )
    .documentation(
        "GPUI Base's tooltip: a trigger showing `label` whose `content` shows on the \
         `placement` side (flipping when it does not fit) once the pointer rests on it for \
         `delay` ms (Base's default, 500 ms, when unset). It hides when the pointer leaves or \
         presses the trigger; `openChange` reports both.",
    )
    .prop("label", PropType::String)
    .prop("content", PropType::String)
    .prop("placement", PropType::Enum(PLACEMENTS))
    .prop("delay", PropType::Number)
    .prop("rootClass", PropType::String)
    .prop("triggerClass", PropType::String)
    .states(POINTER_TRIGGER_STATES)
    .prop("contentClass", PropType::String)
    .event("openChange", &[("open", PropType::Boolean)])
}

#[cfg(test)]
mod tests {
    use std::{sync::Arc, time::Duration};

    use gpui_kit::{
        App, Bounds, ElementId, InputEvent as _, Modifiers, MouseMoveEvent, Pixels, TestAppContext,
        Window, point, px,
    };
    use serde_json::{Value, json};

    use crate::testing::{Fixture, TestWindowExt as _};

    /// The id of the fixture control's part `name`.
    fn part(fixture: &Fixture, name: &str) -> ElementId {
        ElementId::NamedChild(Arc::new(fixture.id.clone()), name.to_owned().into())
    }

    fn event(name: &str, data: Value) -> (String, Value) {
        (name.into(), data)
    }

    fn open_change(open: bool) -> Vec<(String, Value)> {
        vec![event("openChange", json!({ "open": open }))]
    }

    /// The bounds of the fixture's part `name`, `None` when it is not drawn.
    fn bounds(fixture: &Fixture, cx: &mut TestAppContext, name: &str) -> Option<Bounds<Pixels>> {
        let id = part(fixture, name);
        fixture.update(cx, |window, _| {
            window.try_find(id).map(|part| part.bounds())
        })
    }

    /// Moves the pointer to the window's far corner, off every control.
    fn move_away(window: &mut Window, cx: &mut App) {
        window.dispatch_event(
            MouseMoveEvent {
                position: point(px(790.), px(590.)),
                pressed_button: None,
                modifiers: Modifiers::default(),
            }
            .to_platform_input(),
            cx,
        );
    }

    /// Lets `ms` pass, runs what became due and draws.
    fn wait(fixture: &Fixture, cx: &mut TestAppContext, ms: u64) {
        cx.executor().advance_clock(Duration::from_millis(ms));
        fixture.update(cx, |_, _| {});
    }

    /// Whether two coordinates agree to the half pixel Base rounds popups to.
    fn near(a: Pixels, b: Pixels) -> bool {
        (f32::from(a) - f32::from(b)).abs() <= 0.5
    }

    const SURFACE: &str = "p_2 rounded_md border_1 border_border bg_background";

    #[gpui_kit::test]
    fn test_popover_builder(cx: &mut TestAppContext) {
        let popover = Fixture::new(
            cx,
            "Popover",
            json!({
                "label": "Details",
                "title": "Dimensions",
                "description": "Set the layer's size.",
                "open": true,
                "placement": "bottom",
                "align": "start",
                "offset": 4,
                "arrowSize": 4,
                "arrowCornerRadius": 6,
                "arrowClass": "bg_background border_border",
                "rootClass": "gap_2",
                "triggerClass": "px_3 py_1",
                "triggerHoverClass": "bg_accent",
                "triggerActiveClass": "bg_muted",
                "triggerFocusClass": "border_ring",
                "triggerOpenClass": "bg_accent",
                "contentClass": SURFACE,
                "titleClass": "font_medium",
                "descriptionClass": "text_muted_foreground",
                "disabledClass": "opacity_50",
            }),
        );
        let size = popover.measure(cx);
        assert!(size.width > px(0.) && size.height > px(0.));
        let trigger = bounds(&popover, cx, "trigger").unwrap();
        let content = bounds(&popover, cx, "content").unwrap();
        assert!(bounds(&popover, cx, "title").is_some());
        assert!(bounds(&popover, cx, "description").is_some());
        // Below the trigger by the offset, lined up with its leading edge.
        assert_eq!(content.top(), trigger.bottom() + px(4.));
        assert_eq!(content.left(), trigger.left());
    }

    #[gpui_kit::test]
    fn test_popover_placement(cx: &mut TestAppContext) {
        let popover = Fixture::new(
            cx,
            "Popover",
            json!({
                "label": "Details",
                "description": "Below or above",
                "open": true,
                "rootClass": "mt_40 ml_80",
                "placement": "top",
                "align": "end",
            }),
        );
        let trigger = bounds(&popover, cx, "trigger").unwrap();
        let content = bounds(&popover, cx, "content").unwrap();
        assert_eq!(content.bottom(), trigger.top());
        assert_eq!(content.right(), trigger.right());

        popover.set_prop(cx, "placement", Some(json!("right")));
        let content = bounds(&popover, cx, "content").unwrap();
        assert_eq!(content.left(), trigger.right());
        assert!(near(content.center().y, trigger.center().y));
    }

    #[gpui_kit::test]
    fn test_popover_click(cx: &mut TestAppContext) {
        let popover = Fixture::new(
            cx,
            "Popover",
            json!({ "label": "Details", "description": "Text" }),
        );
        let trigger = part(&popover, "trigger");
        popover.update(cx, |window, cx| window.click(trigger.clone(), cx));
        assert_eq!(popover.take_events(), open_change(true));
        assert!(bounds(&popover, cx, "content").is_some());
        // A click inside the content keeps it open.
        let description = part(&popover, "description");
        popover.update(cx, |window, cx| window.click(description, cx));
        assert!(popover.take_events().is_empty());
        // A click outside closes it.
        popover.update(cx, |window, cx| {
            window.drag(point(px(700.), px(500.)), point(px(700.), px(500.)), cx)
        });
        assert_eq!(popover.take_events(), open_change(false));
        assert!(bounds(&popover, cx, "content").is_none());
        // A click on the trigger closes an open popover.
        popover.update(cx, |window, cx| window.click(trigger.clone(), cx));
        popover.update(cx, |window, cx| window.click(trigger.clone(), cx));
        assert_eq!(
            popover.take_events(),
            [open_change(true), open_change(false)].concat()
        );
    }

    #[gpui_kit::test]
    fn test_popover_keyboard(cx: &mut TestAppContext) {
        let popover = Fixture::new(
            cx,
            "Popover",
            json!({ "label": "Details", "description": "Text" }),
        );
        let focus = popover.focus_handle(cx).unwrap();
        popover.update(cx, |window, cx| focus.focus(window, cx));
        popover.update(cx, |window, cx| window.press("enter", cx));
        assert_eq!(popover.take_events(), open_change(true));
        // Opening moves focus into the content.
        popover.update(cx, |window, _| assert!(!focus.is_focused(window)));
        popover.update(cx, |window, cx| window.press("escape", cx));
        assert_eq!(popover.take_events(), open_change(false));
        popover.update(cx, |window, _| assert!(focus.is_focused(window)));
        popover.update(cx, |window, cx| window.press("space", cx));
        assert_eq!(popover.take_events(), open_change(true));
    }

    #[gpui_kit::test]
    fn test_popover_controlled(cx: &mut TestAppContext) {
        let popover = Fixture::new(
            cx,
            "Popover",
            json!({ "label": "Details", "description": "Text" }),
        );
        assert!(bounds(&popover, cx, "content").is_none());
        popover.set_prop(cx, "open", Some(json!(true)));
        assert!(bounds(&popover, cx, "content").is_some());
        popover.set_prop(cx, "title", Some(json!("Title")));
        assert!(bounds(&popover, cx, "title").is_some());
        popover.set_prop(cx, "open", Some(json!(false)));
        assert!(bounds(&popover, cx, "content").is_none());
        // JS's own changes are not reported back.
        assert!(popover.take_events().is_empty());
    }

    #[gpui_kit::test]
    fn test_popover_disabled(cx: &mut TestAppContext) {
        let popover = Fixture::new(
            cx,
            "Popover",
            json!({ "label": "Details", "description": "Text", "disabled": true }),
        );
        assert!(popover.focus_handle(cx).is_none());
        // Base keeps a disabled popover closed, even when asked to open.
        popover.set_prop(cx, "open", Some(json!(true)));
        assert!(bounds(&popover, cx, "content").is_none());
        popover.update(cx, |window, cx| {
            assert!(window.find(part(&popover, "trigger")).disabled() == Some(true));
            window.click(part(&popover, "trigger"), cx);
        });
        assert!(popover.take_events().is_empty());
        assert!(bounds(&popover, cx, "content").is_none());
    }

    #[gpui_kit::test]
    fn test_hover_card_builder(cx: &mut TestAppContext) {
        let card = Fixture::new(
            cx,
            "HoverCard",
            json!({
                "label": "@gpui",
                "title": "GPUI",
                "description": "A GPU-accelerated UI framework.",
                "placement": "bottom",
                "align": "center",
                "openDelay": 100,
                "closeDelay": 50,
                "rootClass": "mt_40 ml_80",
                "triggerClass": "text_primary",
                "triggerHoverClass": "underline",
                "triggerActiveClass": "opacity_80",
                "contentClass": SURFACE,
                "titleClass": "font_semibold",
                "descriptionClass": "text_muted_foreground",
            }),
        );
        let size = card.measure(cx);
        assert!(size.width > px(0.) && size.height > px(0.));
        let trigger = part(&card, "trigger");
        card.update(cx, |window, cx| window.hover(trigger, cx));
        wait(&card, cx, 100);
        let trigger = bounds(&card, cx, "trigger").unwrap();
        let content = bounds(&card, cx, "content").unwrap();
        assert!(bounds(&card, cx, "title").is_some());
        // Below the trigger, centered on it.
        assert_eq!(content.top(), trigger.bottom());
        assert!(near(content.center().x, trigger.center().x));
    }

    #[gpui_kit::test]
    fn test_hover_card_hover(cx: &mut TestAppContext) {
        let card = Fixture::new(
            cx,
            "HoverCard",
            json!({
                "label": "@gpui",
                "description": "Card",
                "openDelay": 100,
                "closeDelay": 50,
            }),
        );
        let trigger = part(&card, "trigger");
        card.update(cx, |window, cx| window.hover(trigger.clone(), cx));
        wait(&card, cx, 99);
        assert!(card.take_events().is_empty());
        assert!(bounds(&card, cx, "content").is_none());
        wait(&card, cx, 1);
        assert_eq!(card.take_events(), open_change(true));
        assert!(bounds(&card, cx, "content").is_some());

        // The pointer may cross from the trigger into the card.
        let content = part(&card, "content");
        card.update(cx, |window, cx| window.hover(content, cx));
        wait(&card, cx, 100);
        assert!(card.take_events().is_empty());

        card.update(cx, move_away);
        wait(&card, cx, 49);
        assert!(card.take_events().is_empty());
        wait(&card, cx, 1);
        assert_eq!(card.take_events(), open_change(false));
        assert!(bounds(&card, cx, "content").is_none());
    }

    #[gpui_kit::test]
    fn test_hover_card_controlled(cx: &mut TestAppContext) {
        let card = Fixture::new(
            cx,
            "HoverCard",
            json!({ "label": "@gpui", "description": "Card", "rootClass": "mt_40 ml_80" }),
        );
        let trigger = part(&card, "trigger");
        card.update(cx, |window, cx| window.hover(trigger, cx));
        // Base's default delay.
        wait(&card, cx, 600);
        assert_eq!(card.take_events(), open_change(true));
        card.set_prop(cx, "placement", Some(json!("top")));
        card.set_prop(cx, "title", Some(json!("GPUI")));
        let trigger = bounds(&card, cx, "trigger").unwrap();
        let content = bounds(&card, cx, "content").unwrap();
        assert!(bounds(&card, cx, "title").is_some());
        assert_eq!(content.bottom(), trigger.top());
        assert!(card.take_events().is_empty());
    }

    #[gpui_kit::test]
    fn test_hover_card_open(cx: &mut TestAppContext) {
        let card = Fixture::new(
            cx,
            "HoverCard",
            json!({ "label": "@gpui", "description": "Card", "open": true }),
        );
        // Open without a hover, and closed by the app.
        assert!(bounds(&card, cx, "content").is_some());
        card.set_prop(cx, "open", Some(json!(false)));
        assert!(bounds(&card, cx, "content").is_none());
        assert!(card.take_events().is_empty());

        // A hover asks to open; the app's answer is shown.
        let trigger = part(&card, "trigger");
        card.update(cx, |window, cx| window.hover(trigger, cx));
        wait(&card, cx, 600);
        assert_eq!(card.take_events(), open_change(true));
        assert!(bounds(&card, cx, "content").is_some());
    }

    #[gpui_kit::test]
    fn test_arrow_offset(cx: &mut TestAppContext) {
        let arrow = json!({
            "label": "Details",
            "description": "Text",
            "open": true,
            "rootClass": "mt_40 ml_80",
            "offset": 6,
            "arrowSize": 6,
            "arrowCornerRadius": 4,
            "arrowClass": "bg_background border_border",
        });
        for name in ["Popover", "HoverCard"] {
            let control = Fixture::new(cx, name, arrow.clone());
            let trigger = bounds(&control, cx, "trigger").unwrap();
            let content = bounds(&control, cx, "content").unwrap();
            // The offset leaves the arrow its depth.
            assert_eq!(content.top(), trigger.bottom() + px(6.), "{name}");
        }
    }

    #[gpui_kit::test]
    fn test_tooltip_builder(cx: &mut TestAppContext) {
        let tooltip = Fixture::new(
            cx,
            "Tooltip",
            json!({
                "label": "Save",
                "content": "Save the document",
                "placement": "bottom",
                "delay": 0,
                "rootClass": "mt_40 ml_80",
                "triggerClass": "px_2 py_1",
                "triggerHoverClass": "bg_accent",
                "triggerActiveClass": "bg_muted",
                "contentClass": "px_2 py_1 rounded_md bg_primary text_primary_foreground",
            }),
        );
        let size = tooltip.measure(cx);
        assert!(size.width > px(0.) && size.height > px(0.));
        let trigger = part(&tooltip, "trigger");
        tooltip.update(cx, |window, cx| window.hover(trigger, cx));
        let trigger = bounds(&tooltip, cx, "trigger").unwrap();
        let popup = bounds(&tooltip, cx, "tooltip").unwrap();
        assert!(popup.size.width > px(0.));
        // Below the trigger, centered on it.
        assert_eq!(popup.top(), trigger.bottom());
        assert!(near(popup.center().x, trigger.center().x));
    }

    #[gpui_kit::test]
    fn test_tooltip_hover(cx: &mut TestAppContext) {
        let tooltip = Fixture::new(
            cx,
            "Tooltip",
            json!({
                "label": "Save",
                "content": "Save the document",
                "delay": 100,
                "rootClass": "mt_40 ml_80",
            }),
        );
        let trigger = part(&tooltip, "trigger");
        tooltip.update(cx, |window, cx| window.hover(trigger.clone(), cx));
        wait(&tooltip, cx, 99);
        assert!(bounds(&tooltip, cx, "tooltip").is_none());
        assert!(tooltip.take_events().is_empty());
        wait(&tooltip, cx, 1);
        assert_eq!(tooltip.take_events(), open_change(true));
        let popup = bounds(&tooltip, cx, "tooltip").unwrap();
        // Base prefers the top side.
        let trigger_bounds = bounds(&tooltip, cx, "trigger").unwrap();
        assert_eq!(popup.bottom(), trigger_bounds.top());

        // Leaving hides it after Base's grace period.
        tooltip.update(cx, move_away);
        assert!(bounds(&tooltip, cx, "tooltip").is_some());
        assert!(tooltip.take_events().is_empty());
        wait(&tooltip, cx, 300);
        assert!(bounds(&tooltip, cx, "tooltip").is_none());
        assert_eq!(tooltip.take_events(), open_change(false));

        // A press hides it at once.
        tooltip.update(cx, |window, cx| window.hover(trigger.clone(), cx));
        wait(&tooltip, cx, 100);
        assert!(bounds(&tooltip, cx, "tooltip").is_some());
        tooltip.update(cx, |window, cx| window.click(trigger.clone(), cx));
        assert!(bounds(&tooltip, cx, "tooltip").is_none());
        assert_eq!(
            tooltip.take_events(),
            [open_change(true), open_change(false)].concat()
        );
    }

    #[gpui_kit::test]
    fn test_tooltip_controlled(cx: &mut TestAppContext) {
        let tooltip = Fixture::new(
            cx,
            "Tooltip",
            json!({ "label": "Save", "content": "Save", "delay": 0, "rootClass": "mt_40 ml_80" }),
        );
        tooltip.set_prop(cx, "placement", Some(json!("right")));
        let trigger = part(&tooltip, "trigger");
        tooltip.update(cx, |window, cx| window.hover(trigger, cx));
        let trigger = bounds(&tooltip, cx, "trigger").unwrap();
        let popup = bounds(&tooltip, cx, "tooltip").unwrap();
        assert_eq!(popup.left(), trigger.right());
        assert!(near(popup.center().y, trigger.center().y));
        // A longer text, set while it shows, widens the shown tooltip.
        tooltip.set_prop(cx, "content", Some(json!("Save the document to disk")));
        let wider = bounds(&tooltip, cx, "tooltip").unwrap();
        assert!(wider.size.width > popup.size.width);
    }
}
