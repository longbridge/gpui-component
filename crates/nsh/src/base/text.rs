//! Controls showing rich text: TextView.

use std::sync::Arc;

use gpui_kit::{
    AppContext as _, ElementId, Entity, IntoElement as _, SharedString, base::text::TextView,
    prelude::FluentBuilder as _,
};
use serde_json::json;

use crate::{Control, Events, PropType};

pub(super) fn controls() -> Vec<Control> {
    vec![text_view()]
}

fn text_view() -> Control {
    // The link handler must be `Send`, so it reaches the events through an
    // entity rather than holding them.
    Control::stateful(
        "TextView",
        |_, events, _, cx| cx.new(|_| events.clone()),
        |sink: &mut Entity<Events>, props, _, _, cx| {
            let text = props.string("text").unwrap_or_default();
            let format = props.string("format").unwrap_or("markdown".into());
            // Base keeps a view's parsed document under its id, so each
            // format gets its own.
            let id = ElementId::NamedChild(Arc::new(props.id()), format.clone());
            let sink = sink.clone();
            props
                .styled(
                    if format == "html" {
                        TextView::html(id, text)
                    } else {
                        TextView::markdown(id, text)
                    },
                    cx,
                )
                .when(props.has("selectable"), |this| {
                    this.selectable(props.boolean("selectable"))
                })
                .on_link_click(move |href: &SharedString, event, _, cx| {
                    if !event.is_right_click() {
                        sink.read(cx)
                            .emit("linkClick", json!({ "href": href.as_ref() }));
                    }
                })
                .into_any_element()
        },
    )
    .documentation(
        "GPUI Base's TextView: Markdown or simple HTML, selectable unless `selectable` is false. \
         Links send `linkClick` instead of opening.",
    )
    .prop("text", PropType::String)
    .prop("format", PropType::Enum(&["markdown", "html"]))
    .prop("selectable", PropType::Boolean)
    .prop("rootClass", PropType::String)
    .event("linkClick", &[("href", PropType::String)])
}

#[cfg(test)]
mod tests {
    use gpui_kit::{
        MouseButton, Pixels, Point, TestAppContext, base::TextSelection, point, px,
        test::TestEventExt as _,
    };
    use serde_json::json;

    use crate::testing::{Fixture, TestWindowExt as _};

    /// A point on the first line of text: the test host pads by 16px.
    fn first_line() -> Point<Pixels> {
        point(px(22.), px(24.))
    }

    fn click(fixture: &Fixture, cx: &mut TestAppContext, button: MouseButton) {
        fixture.update(cx, |window, cx| {
            window.pointer_move(first_line(), None, cx);
            window.pointer_down(first_line(), button, cx);
            window.pointer_up(first_line(), button, cx);
        });
    }

    #[gpui_kit::test]
    fn test_text_view_builder(cx: &mut TestAppContext) {
        let view = Fixture::new(
            cx,
            "TextView",
            json!({
                "text": "# Title\n\nSee [docs](https://example.com/docs). The quick brown fox jumps over the lazy dog many times.",
                "format": "markdown",
                "selectable": true,
                "rootClass": "text_sm text_muted_foreground",
            }),
        );
        let size = view.measure(cx);
        assert!(size.width > px(0.) && size.height > px(0.));
    }

    #[gpui_kit::test]
    fn test_text_view_markdown_heading(cx: &mut TestAppContext) {
        let view = Fixture::new(cx, "TextView", json!({ "text": "Title" }));
        let paragraph = view.measure(cx);
        view.set_prop(cx, "text", Some(json!("# Title")));
        let heading = view.measure(cx);
        assert!(heading.height > paragraph.height);
        view.set_prop(cx, "text", Some(json!("Title\n\nMore\n\nAnd more")));
        assert!(view.measure(cx).height > paragraph.height);
    }

    #[gpui_kit::test]
    fn test_text_view_wraps_in_box_width(cx: &mut TestAppContext) {
        let view = Fixture::new(
            cx,
            "TextView",
            json!({ "text": "The quick brown fox jumps over the lazy dog many times." }),
        );
        let unwrapped = view.measure_at(cx, None);
        let wrapped = view.measure_at(cx, Some(px(200.)));
        assert_eq!(wrapped.width, px(200.));
        assert!(wrapped.height > unwrapped.height);
    }

    #[gpui_kit::test]
    fn test_text_view_html(cx: &mut TestAppContext) {
        let view = Fixture::new(
            cx,
            "TextView",
            json!({ "text": "<p>Title</p>", "format": "html" }),
        );
        let paragraph = view.measure(cx);
        assert!(paragraph.height > px(0.));
        view.set_prop(cx, "text", Some(json!("<h1>Title</h1>")));
        assert!(view.measure(cx).height > paragraph.height);

        view.set_prop(
            cx,
            "text",
            Some(json!(r#"<a href="https://example.com/html">docs</a>"#)),
        );
        click(&view, cx, MouseButton::Left);
        assert_eq!(
            view.take_events(),
            vec![(
                "linkClick".into(),
                json!({ "href": "https://example.com/html" })
            )]
        );
    }

    #[gpui_kit::test]
    fn test_text_view_link_click(cx: &mut TestAppContext) {
        let view = Fixture::new(
            cx,
            "TextView",
            json!({ "text": "[docs](https://example.com/docs)" }),
        );
        click(&view, cx, MouseButton::Left);
        assert_eq!(
            view.take_events(),
            vec![(
                "linkClick".into(),
                json!({ "href": "https://example.com/docs" })
            )]
        );
        click(&view, cx, MouseButton::Right);
        assert!(view.take_events().is_empty());

        view.set_prop(cx, "text", Some(json!("[blog](https://example.com/blog)")));
        click(&view, cx, MouseButton::Left);
        assert_eq!(
            view.take_events(),
            vec![(
                "linkClick".into(),
                json!({ "href": "https://example.com/blog" })
            )]
        );
    }

    #[gpui_kit::test]
    fn test_text_view_selectable(cx: &mut TestAppContext) {
        let view = Fixture::new(cx, "TextView", json!({ "text": "Hello world" }));
        let select = |view: &Fixture, cx: &mut TestAppContext| {
            view.update(cx, |window, cx| {
                window.drag(point(px(17.), px(24.)), point(px(200.), px(24.)), cx);
                TextSelection::selected_text(window, cx)
            })
        };
        assert_eq!(select(&view, cx).trim(), "Hello world");

        view.set_prop(cx, "selectable", Some(json!(false)));
        view.update(cx, |window, cx| {
            window.pointer_down(point(px(400.), px(400.)), MouseButton::Left, cx);
            window.pointer_up(point(px(400.), px(400.)), MouseButton::Left, cx);
        });
        assert_eq!(select(&view, cx).trim(), "");
    }
}
