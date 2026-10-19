//! Controls that display a value: Progress, Avatar, SelectableText.

use gpui_kit::{
    IntoElement as _, ParentElement as _, StyleRefinement, Styled as _,
    base::{
        Avatar, AvatarFallback, AvatarImage, Progress, ProgressIndicator, ProgressTrack,
        SelectableText,
    },
    prelude::FluentBuilder as _,
    relative,
};

use crate::{Control, PropType};

pub(super) fn controls() -> Vec<Control> {
    vec![progress(), avatar(), selectable_text()]
}

fn progress() -> Control {
    Control::new("Progress", |props, _, _, cx| {
        let indeterminate = props.boolean("indeterminate");
        let value = props.number("value") as f32;
        props
            .styled(Progress::new(props.id()), cx)
            .value(value)
            .indeterminate(indeterminate)
            .when_some(props.string("accessibilityLabel"), |this, label| {
                this.accessibility_label(label)
            })
            .child(
                props
                    .styled_by("trackClass", ProgressTrack::new(), cx)
                    .child(
                        props
                            .styled_by("indicatorClass", ProgressIndicator::new(), cx)
                            .when(!indeterminate, |this| {
                                this.w(relative(value.clamp(0., 100.) / 100.))
                            }),
                    ),
            )
            .into_any_element()
    })
    .documentation(
        "GPUI Base's Progress: a percentage (0 to 100) or an indeterminate state, \
         announced as a progress indicator. The indicator's width is the value's \
         share of the track; when indeterminate, `indicatorClass` sizes it.",
    )
    .prop("value", PropType::Number)
    .prop("indeterminate", PropType::Boolean)
    .prop("accessibilityLabel", PropType::String)
    .prop("rootClass", PropType::String)
    .prop("trackClass", PropType::String)
    .prop("indicatorClass", PropType::String)
}

fn avatar() -> Control {
    Control::new("Avatar", |props, _, _, cx| {
        props
            .styled(Avatar::new(), cx)
            .when_some(props.string("src"), |this, src| {
                this.image(props.styled_by("imageClass", AvatarImage::new(src), cx))
            })
            .when_some(props.string("fallback"), |this, fallback| {
                this.fallback(
                    props
                        .styled_by("fallbackClass", AvatarFallback::new(), cx)
                        .child(fallback),
                )
            })
            .into_any_element()
    })
    .documentation(
        "GPUI Base's Avatar: the image at `src` (a URL, or an asset path), or the \
         `fallback` text (initials) without one.",
    )
    .prop("src", PropType::String)
    .prop("fallback", PropType::String)
    .prop("rootClass", PropType::String)
    .prop("imageClass", PropType::String)
    .prop("fallbackClass", PropType::String)
}

fn selectable_text() -> Control {
    Control::new("SelectableText", |props, _, _, cx| {
        // SelectableText is a text run, not a box: it takes the text styles
        // of its class (`text_sm font_semibold text_muted_foreground`).
        let style = props.styled(StyleRefinement::default(), cx);
        SelectableText::new(props.id(), props.string("text").unwrap_or_default())
            .text_style(style.text)
            .into_any_element()
    })
    .documentation(
        "GPUI Base's SelectableText: plain text the pointer selects and the copy \
         shortcut copies, sharing the window's text selection. `rootClass` takes text \
         styles only.",
    )
    .prop("text", PropType::String)
    .prop("rootClass", PropType::String)
}

#[cfg(test)]
mod tests {
    use gpui_kit::{Role, TestAppContext, base::TextSelection, point, px, size};
    use serde_json::json;

    use crate::testing::{Fixture, TestWindowExt as _};

    #[gpui_kit::test]
    fn test_progress_builder(cx: &mut TestAppContext) {
        let progress = Fixture::new(
            cx,
            "Progress",
            json!({
                "value": 40,
                "accessibilityLabel": "Uploading",
                "rootClass": "w_64 h_2",
                "trackClass": "size_full bg_muted",
                "indicatorClass": "h_full bg_primary",
            }),
        );
        assert_eq!(progress.measure(cx), size(px(256.), px(8.)));
        progress.update(cx, |window, _| {
            let root = window.find(progress.id.clone());
            assert_eq!(root.role(), Some(Role::ProgressIndicator));
            assert_eq!(root.label(), Some("Uploading"));
            assert_eq!(root.bounds().size, size(px(256.), px(8.)));
        });
    }

    #[gpui_kit::test]
    fn test_progress_props(cx: &mut TestAppContext) {
        let progress = Fixture::new(
            cx,
            "Progress",
            json!({ "value": 120, "rootClass": "w_64 h_2", "indicatorClass": "h_full" }),
        );
        progress.set_prop(cx, "indeterminate", Some(json!(true)));
        progress.set_prop(cx, "accessibilityLabel", Some(json!("Loading")));
        progress.update(cx, |window, _| {
            assert_eq!(window.find(progress.id.clone()).label(), Some("Loading"));
        });
        progress.set_prop(cx, "rootClass", Some(json!("w_32 h_1")));
        assert_eq!(progress.measure(cx), size(px(128.), px(4.)));
    }

    #[gpui_kit::test]
    fn test_avatar_fallback(cx: &mut TestAppContext) {
        let avatar = Fixture::new(cx, "Avatar", json!({ "fallback": "JL" }));
        let text = avatar.measure(cx);
        assert!(text.width > px(0.) && text.height > px(0.));
        avatar.set_prop(cx, "fallbackClass", Some(json!("size_10")));
        assert_eq!(avatar.measure(cx), size(px(40.), px(40.)));
        avatar.set_prop(cx, "fallback", None);
        assert_eq!(avatar.measure(cx), size(px(0.), px(0.)));
    }

    #[gpui_kit::test]
    fn test_avatar_image_takes_precedence(cx: &mut TestAppContext) {
        let avatar = Fixture::new(
            cx,
            "Avatar",
            json!({
                "src": "https://example.com/avatar.png",
                "fallback": "JL",
                "rootClass": "rounded_full overflow_hidden",
                "imageClass": "size_6",
                "fallbackClass": "size_10",
            }),
        );
        assert_eq!(avatar.measure(cx), size(px(24.), px(24.)));
        avatar.set_prop(cx, "src", None);
        assert_eq!(avatar.measure(cx), size(px(40.), px(40.)));
    }

    #[gpui_kit::test]
    fn test_selectable_text_selects_by_drag(cx: &mut TestAppContext) {
        let text = Fixture::new(
            cx,
            "SelectableText",
            json!({ "text": "alpha beta", "rootClass": "text_sm" }),
        );
        let natural = text.measure(cx);
        assert!(natural.width > px(0.) && natural.height > px(0.));
        // The host draws the control at its 16px padding.
        let y = px(16.) + natural.height / 2.;
        text.update(cx, |window, cx| {
            window.drag(
                point(px(17.), y),
                point(px(16.) + natural.width - px(1.), y),
                cx,
            );
            assert_eq!(TextSelection::selected_text(window, cx), "alpha beta");
        });
    }

    #[gpui_kit::test]
    fn test_selectable_text_props(cx: &mut TestAppContext) {
        let text = Fixture::new(cx, "SelectableText", json!({ "text": "alpha" }));
        let short = text.measure(cx);
        text.set_prop(cx, "text", Some(json!("alpha beta gamma")));
        let long = text.measure(cx);
        assert!(long.width > short.width);
        text.set_prop(cx, "rootClass", Some(json!("text_3xl")));
        assert!(text.measure(cx).height > long.height);
    }
}
