use std::{cell::RefCell, rc::Rc};

use gpui::{
    AppContext as _, Context, IntoElement, Render, SharedString, Styled as _, TestAppContext,
    Window, div,
};
use gpui_base::{Root, Theme};

struct FontProbe {
    observed: Rc<RefCell<Vec<SharedString>>>,
}

impl Render for FontProbe {
    fn render(&mut self, window: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        self.observed
            .borrow_mut()
            .push(window.text_style().font_family);
        div()
    }
}

fn draw_and_observe(
    cx: &mut TestAppContext,
    configure_root: impl FnOnce(Root) -> Root + 'static,
) -> (Rc<RefCell<Vec<SharedString>>>, &mut gpui::VisualTestContext) {
    let observed = Rc::new(RefCell::new(Vec::new()));
    let result = observed.clone();
    let (_, window) = cx.add_window_view(move |_, cx| {
        let probe = cx.new(|_| FontProbe { observed });
        configure_root(Root::new(probe, cx))
    });
    window.update(|window, cx| window.draw(cx).clear(cx));
    (result, window)
}

#[gpui::test]
fn root_children_inherit_the_system_ui_font_by_default(cx: &mut TestAppContext) {
    cx.update(gpui_base::init);

    let (observed, _) = draw_and_observe(cx, |root| root);

    assert_eq!(observed.borrow().last().unwrap(), ".SystemUIFont");
}

#[gpui::test]
fn root_children_inherit_the_base_theme_font(cx: &mut TestAppContext) {
    cx.update(|cx| {
        gpui_base::init(cx);
        Theme::global_mut(cx).tokens.typography.sans = "Application Sans".into();
    });

    let (observed, _) = draw_and_observe(cx, |root| root);

    assert_eq!(observed.borrow().last().unwrap(), "Application Sans");
}

#[gpui::test]
fn root_style_overrides_the_base_theme_font(cx: &mut TestAppContext) {
    cx.update(|cx| {
        gpui_base::init(cx);
        Theme::global_mut(cx).tokens.typography.sans = "Application Sans".into();
    });

    let (observed, _) = draw_and_observe(cx, |root| root.font_family("Window Sans"));

    assert_eq!(observed.borrow().last().unwrap(), "Window Sans");
}

#[gpui::test]
fn root_reacts_to_runtime_base_theme_changes(cx: &mut TestAppContext) {
    cx.update(gpui_base::init);
    let (observed, window) = draw_and_observe(cx, |root| root);
    observed.borrow_mut().clear();

    window.update(|_, cx| {
        Theme::global_mut(cx).tokens.typography.sans = "Runtime Sans".into();
    });
    window.run_until_parked();

    assert_eq!(observed.borrow().last().unwrap(), "Runtime Sans");
}

#[gpui::test]
fn closing_the_window_releases_root_and_its_theme_observer(cx: &mut TestAppContext) {
    cx.update(gpui_base::init);
    let observed = Rc::new(RefCell::new(Vec::new()));
    let (root, window) = cx.add_window_view({
        let observed = observed.clone();
        move |_, cx| {
            let probe = cx.new(|_| FontProbe { observed });
            Root::new(probe, cx)
        }
    });
    let weak_root = root.downgrade();
    drop(root);

    window.update(|window, _| window.remove_window());
    window.run_until_parked();

    assert!(weak_root.upgrade().is_none());

    window.cx.update(|cx| {
        Theme::global_mut(cx).tokens.typography.sans = "After Close Sans".into();
    });
    window.run_until_parked();

    assert!(weak_root.upgrade().is_none());
}
