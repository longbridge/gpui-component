//! Controls activated by a press: Button, Toggle, Link.

use gpui_kit::{
    IntoElement as _, ParentElement as _, Role,
    base::{Button, Link, RoleOverride, Toggle},
    prelude::FluentBuilder as _,
};
use serde_json::json;

use crate::{Control, PropType, Props};

pub(super) fn controls() -> Vec<Control> {
    vec![button(), toggle(), link()]
}

/// The values of Button's `role` prop; the first keeps Base's implicit role.
const BUTTON_ROLES: &[&str] = &[
    "button",
    "presentation",
    "link",
    "menuItem",
    "tab",
    "listBoxOption",
    "treeItem",
];

/// The [`RoleOverride`] Button's `role` prop names.
fn button_role(props: &Props) -> RoleOverride {
    match props.string("role").as_deref() {
        Some("presentation") => RoleOverride::Presentational,
        Some("link") => Role::Link.into(),
        Some("menuItem") => Role::MenuItem.into(),
        Some("tab") => Role::Tab.into(),
        Some("listBoxOption") => Role::ListBoxOption.into(),
        Some("treeItem") => Role::TreeItem.into(),
        _ => RoleOverride::Implicit,
    }
}

/// The focus traversal index of the `tabIndex` prop, when the app set it.
fn tab_index(props: &Props) -> Option<isize> {
    props
        .has("tabIndex")
        .then(|| props.number("tabIndex") as isize)
}

/// A boolean prop whose Base default is `true`, when the app set it.
fn flag(props: &Props, name: &str) -> Option<bool> {
    props.has(name).then(|| props.boolean(name))
}

fn button() -> Control {
    Control::new("Button", |props, events, _, cx| {
        props
            .styled(Button::new(props.id()), cx)
            .disabled(props.boolean("disabled"))
            .selected(props.boolean("selected"))
            .role(button_role(props))
            .styles(|styles| {
                styles
                    .selected(|style| props.styled_by("selectedClass", style, cx))
                    .disabled(|style| props.styled_by("disabledClass", style, cx))
            })
            .when_some(props.string("accessibilityLabel"), |this, label| {
                this.accessibility_label(label)
            })
            .when_some(tab_index(props), |this, index| this.tab_index(index))
            .when_some(flag(props, "tabStop"), |this, tab_stop| {
                this.tab_stop(tab_stop)
            })
            .when_some(flag(props, "focusable"), |this, focusable| {
                this.focusable(focusable)
            })
            .when_some(props.string("label"), |this, label| this.child(label))
            .on_click(events.handler("click", |_| json!({})))
            .into_any_element()
    })
    .documentation("GPUI Base's Button: activated by a click, Enter or Space.")
    .prop("label", PropType::String)
    .prop("disabled", PropType::Boolean)
    .prop("selected", PropType::Boolean)
    .prop("role", PropType::Enum(BUTTON_ROLES))
    .prop("accessibilityLabel", PropType::String)
    .prop("tabIndex", PropType::Number)
    .prop("tabStop", PropType::Boolean)
    .prop("focusable", PropType::Boolean)
    .prop("rootClass", PropType::String)
    .prop("selectedClass", PropType::String)
    .prop("disabledClass", PropType::String)
    .event("click", &[])
}

fn toggle() -> Control {
    Control::new("Toggle", |props, events, _, cx| {
        let change = events.clone();
        props
            .styled(Toggle::new(props.id()), cx)
            .pressed(props.boolean("pressed"))
            .disabled(props.boolean("disabled"))
            .styles(|styles| {
                styles
                    .pressed(|style| props.styled_by("pressedClass", style, cx))
                    .disabled(|style| props.styled_by("disabledClass", style, cx))
            })
            .when_some(props.string("accessibilityLabel"), |this, label| {
                this.accessibility_label(label)
            })
            .when_some(tab_index(props), |this, index| this.tab_index(index))
            .when_some(flag(props, "tabStop"), |this, tab_stop| {
                this.tab_stop(tab_stop)
            })
            .when_some(props.string("label"), |this, label| this.child(label))
            .on_change(move |pressed, _, _, _| change.emit("change", json!({ "pressed": pressed })))
            .into_any_element()
    })
    .documentation(
        "GPUI Base's Toggle: a controlled two-state button; a click, Enter or Space \
         asks to flip `pressed`.",
    )
    .prop("label", PropType::String)
    .prop("pressed", PropType::Boolean)
    .prop("disabled", PropType::Boolean)
    .prop("accessibilityLabel", PropType::String)
    .prop("tabIndex", PropType::Number)
    .prop("tabStop", PropType::Boolean)
    .prop("rootClass", PropType::String)
    .prop("pressedClass", PropType::String)
    .prop("disabledClass", PropType::String)
    .event("change", &[("pressed", PropType::Boolean)])
}

/// Base's Link never opens `href` itself: it only calls the strategy an app
/// injects with `open_with`. Opening is the app's decision, so the control
/// injects none and sends `activate` with the target for JS to open, route
/// or ignore.
fn link() -> Control {
    Control::new("Link", |props, events, _, cx| {
        let href = props.string("href");
        let activate = events.clone();
        props
            .styled(Link::new(props.id()), cx)
            .disabled(props.boolean("disabled"))
            .styles(|styles| styles.disabled(|style| props.styled_by("disabledClass", style, cx)))
            .when_some(href.clone(), |this, href| this.href(href))
            .when_some(props.string("accessibilityLabel"), |this, label| {
                this.accessibility_label(label)
            })
            .when_some(tab_index(props), |this, index| this.tab_index(index))
            .when_some(flag(props, "tabStop"), |this, tab_stop| {
                this.tab_stop(tab_stop)
            })
            .when_some(props.string("label"), |this, label| this.child(label))
            .on_activate(move |_, _, _| {
                activate.emit(
                    "activate",
                    json!({ "href": href.as_deref().unwrap_or_default() }),
                )
            })
            .into_any_element()
    })
    .documentation(
        "GPUI Base's Link: activated by a click or the keyboard. It never opens `href` \
         itself; `activate` carries `href` for the app to open.",
    )
    .prop("label", PropType::String)
    .prop("href", PropType::String)
    .prop("disabled", PropType::Boolean)
    .prop("accessibilityLabel", PropType::String)
    .prop("tabIndex", PropType::Number)
    .prop("tabStop", PropType::Boolean)
    .prop("rootClass", PropType::String)
    .prop("disabledClass", PropType::String)
    .event("activate", &[("href", PropType::String)])
}

#[cfg(test)]
mod tests {
    use gpui_kit::{
        App, InputEvent as _, Modifiers, MouseButton, MouseDownEvent, MouseUpEvent, Role,
        TestAppContext, Window, point, px,
    };
    use serde_json::json;

    use crate::testing::{Fixture, TestWindowExt as _};

    /// Clicks the window's control at its top-left corner: Base's Link is not
    /// observed under its id, so `TestWindowExt::click` cannot find it.
    fn click_control(window: &mut Window, cx: &mut App) {
        let position = point(px(20.), px(20.));
        window.dispatch_event(
            MouseDownEvent {
                button: MouseButton::Left,
                position,
                modifiers: Modifiers::default(),
                click_count: 1,
                first_mouse: false,
            }
            .to_platform_input(),
            cx,
        );
        window.dispatch_event(
            MouseUpEvent {
                button: MouseButton::Left,
                position,
                modifiers: Modifiers::default(),
                click_count: 1,
            }
            .to_platform_input(),
            cx,
        );
    }

    #[gpui_kit::test]
    fn test_button_click(cx: &mut TestAppContext) {
        let button = Fixture::new(
            cx,
            "Button",
            json!({ "label": "Save", "rootClass": "px_3 py_1 rounded_md bg_primary" }),
        );
        let size = button.measure(cx);
        assert!(size.width > px(0.) && size.height > px(0.));
        button.update(cx, |window, cx| window.click(button.id.clone(), cx));
        assert_eq!(button.take_events(), vec![("click".into(), json!({}))]);
    }

    #[gpui_kit::test]
    fn test_button_builder(cx: &mut TestAppContext) {
        let button = Fixture::new(
            cx,
            "Button",
            json!({
                "label": "Bold",
                "selected": true,
                "role": "menuItem",
                "accessibilityLabel": "Bold text",
                "tabIndex": 1,
                "tabStop": true,
                "focusable": true,
                "rootClass": "px_3 py_1",
                "selectedClass": "bg_accent",
                "disabledClass": "opacity_50",
            }),
        );
        let size = button.measure(cx);
        assert!(size.width > px(0.) && size.height > px(0.));
        button.update(cx, |window, _| {
            let found = window.find(button.id.clone());
            assert_eq!(found.role(), Some(Role::MenuItem));
            assert_eq!(found.label(), Some("Bold text"));
        });
        button.set_prop(cx, "role", Some(json!("presentation")));
        button.update(cx, |window, _| {
            assert_eq!(window.find(button.id.clone()).role(), None);
        });
    }

    #[gpui_kit::test]
    fn test_button_keyboard(cx: &mut TestAppContext) {
        let button = Fixture::new(cx, "Button", json!({ "label": "Save" }));
        button.update(cx, |window, cx| window.click(button.id.clone(), cx));
        button.update(cx, |window, cx| {
            assert_eq!(window.find(button.id.clone()).focused(), Some(true));
            window.press("enter", cx);
            window.press("space", cx);
        });
        assert_eq!(button.take_events().len(), 3);
    }

    #[gpui_kit::test]
    fn test_button_disabled(cx: &mut TestAppContext) {
        let button = Fixture::new(cx, "Button", json!({ "label": "Save", "disabled": true }));
        button.update(cx, |window, cx| window.click(button.id.clone(), cx));
        assert!(button.take_events().is_empty());
        button.set_prop(cx, "disabled", None);
        button.update(cx, |window, cx| window.click(button.id.clone(), cx));
        assert_eq!(button.take_events().len(), 1);
    }

    #[gpui_kit::test]
    fn test_toggle_builder(cx: &mut TestAppContext) {
        let toggle = Fixture::new(
            cx,
            "Toggle",
            json!({
                "label": "Bold",
                "pressed": true,
                "accessibilityLabel": "Bold text",
                "tabIndex": 0,
                "tabStop": true,
                "rootClass": "px_2 py_1 rounded_md",
                "pressedClass": "bg_accent",
                "disabledClass": "opacity_50",
            }),
        );
        let size = toggle.measure(cx);
        assert!(size.width > px(0.) && size.height > px(0.));
        toggle.update(cx, |window, _| {
            let found = window.find(toggle.id.clone());
            assert_eq!(found.role(), Some(Role::Button));
            assert_eq!(found.checked(), Some(true));
            assert_eq!(found.label(), Some("Bold text"));
        });
    }

    #[gpui_kit::test]
    fn test_toggle_change(cx: &mut TestAppContext) {
        let toggle = Fixture::new(cx, "Toggle", json!({ "label": "Bold" }));
        toggle.update(cx, |window, cx| window.click(toggle.id.clone(), cx));
        assert_eq!(
            toggle.take_events(),
            vec![("change".into(), json!({ "pressed": true }))]
        );
        toggle.update(cx, |window, _| {
            assert_eq!(window.find(toggle.id.clone()).checked(), Some(true));
        });

        toggle.update(cx, |window, cx| window.press("space", cx));
        assert_eq!(
            toggle.take_events(),
            vec![("change".into(), json!({ "pressed": false }))]
        );
    }

    #[gpui_kit::test]
    fn test_toggle_controlled(cx: &mut TestAppContext) {
        let toggle = Fixture::new(cx, "Toggle", json!({ "label": "Bold" }));
        toggle.set_prop(cx, "pressed", Some(json!(true)));
        toggle.update(cx, |window, _| {
            assert_eq!(window.find(toggle.id.clone()).checked(), Some(true));
        });
        toggle.set_prop(cx, "pressed", Some(json!(false)));
        toggle.update(cx, |window, _| {
            assert_eq!(window.find(toggle.id.clone()).checked(), Some(false));
        });
    }

    #[gpui_kit::test]
    fn test_toggle_disabled(cx: &mut TestAppContext) {
        let toggle = Fixture::new(cx, "Toggle", json!({ "label": "Bold", "disabled": true }));
        toggle.update(cx, |window, cx| {
            assert_eq!(window.find(toggle.id.clone()).disabled(), Some(true));
            window.click(toggle.id.clone(), cx);
        });
        assert!(toggle.take_events().is_empty());
        toggle.set_prop(cx, "disabled", None);
        toggle.update(cx, |window, cx| window.click(toggle.id.clone(), cx));
        assert_eq!(toggle.take_events().len(), 1);
    }

    #[gpui_kit::test]
    fn test_link_activate(cx: &mut TestAppContext) {
        let link = Fixture::new(
            cx,
            "Link",
            json!({
                "label": "GPUI Kit",
                "href": "https://gpui-kit.com",
                "accessibilityLabel": "Open GPUI Kit",
                "tabIndex": 0,
                "tabStop": true,
                "rootClass": "text_primary",
                "disabledClass": "text_muted_foreground",
            }),
        );
        let size = link.measure(cx);
        assert!(size.width > px(0.) && size.height > px(0.));
        link.update(cx, click_control);
        let activated = ("activate".into(), json!({ "href": "https://gpui-kit.com" }));
        assert_eq!(link.take_events(), vec![activated.clone()]);

        // The click focused the link.
        link.update(cx, |window, cx| window.press("enter", cx));
        assert_eq!(link.take_events(), vec![activated]);
    }

    #[gpui_kit::test]
    fn test_link_controlled_href(cx: &mut TestAppContext) {
        let link = Fixture::new(cx, "Link", json!({ "label": "Docs" }));
        link.update(cx, click_control);
        assert_eq!(
            link.take_events(),
            vec![("activate".into(), json!({ "href": "" }))]
        );
        link.set_prop(cx, "href", Some(json!("/docs")));
        link.update(cx, click_control);
        assert_eq!(
            link.take_events(),
            vec![("activate".into(), json!({ "href": "/docs" }))]
        );
    }

    #[gpui_kit::test]
    fn test_link_disabled(cx: &mut TestAppContext) {
        let link = Fixture::new(
            cx,
            "Link",
            json!({ "label": "Docs", "href": "/docs", "disabled": true }),
        );
        link.update(cx, click_control);
        assert!(link.take_events().is_empty());
        link.set_prop(cx, "disabled", None);
        link.update(cx, click_control);
        assert_eq!(link.take_events().len(), 1);
    }
}
