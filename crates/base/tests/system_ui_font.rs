use std::{cell::RefCell, rc::Rc};

use gpui::{
    Context, IntoElement, ParentElement as _, Render, Styled as _, TestAppContext, Window, div,
    prelude::FluentBuilder as _,
};
use gpui_base::{Button, ElementExt as _};

struct FontHarness {
    application_font: Option<&'static str>,
    observed: Rc<RefCell<Option<String>>>,
}

impl Render for FontHarness {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let observed = self.observed.clone();
        div()
            .when_some(self.application_font, |root, font| root.font_family(font))
            .child(Button::new("probe").child(
                div().on_prepaint(move |_, window, _| {
                    *observed.borrow_mut() = Some(window.text_style().font_family.to_string());
                }),
            ))
    }
}

fn rendered_button_font(
    cx: &mut TestAppContext,
    application_font: Option<&'static str>,
) -> String {
    let observed = Rc::new(RefCell::new(None));
    let result = observed.clone();
    let (_, cx) = cx.add_window_view(move |_, _| FontHarness {
        application_font,
        observed,
    });

    cx.update(|window, cx| window.draw(cx).clear(cx));

    result
        .borrow_mut()
        .take()
        .expect("the button child should be prepainted")
}

#[gpui::test]
fn base_controls_inherit_the_system_ui_font_by_default(cx: &mut TestAppContext) {
    assert_eq!(rendered_button_font(cx, None), ".SystemUIFont");
}

#[gpui::test]
fn application_ancestor_can_override_the_base_font(cx: &mut TestAppContext) {
    assert_eq!(
        rendered_button_font(cx, Some("Application Sans")),
        "Application Sans"
    );
}
