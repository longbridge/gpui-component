use gpui::{
    AnyView, Context, IntoElement, ParentElement as _, Render, StyleRefinement, Styled,
    Subscription, Window, div,
};

use crate::{StyledExt as _, Theme};

/// Establishes Base theme inheritance for a window's application view.
///
/// Place one `Root` at the first level of each window. Its child inherits the
/// active Base typography font, while styles applied directly to `Root` take
/// precedence over that theme default.
pub struct Root {
    style: StyleRefinement,
    view: AnyView,
    _theme_subscription: Subscription,
}

impl Root {
    pub fn new(view: impl Into<AnyView>, cx: &mut Context<Self>) -> Self {
        let theme_subscription = cx.observe_global::<Theme>(|_, cx| cx.notify());
        Self {
            style: StyleRefinement::default(),
            view: view.into(),
            _theme_subscription: theme_subscription,
        }
    }
}

impl Styled for Root {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl Render for Root {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .font_family(Theme::global(cx).tokens.typography.sans)
            .refine_style(&self.style)
            .child(self.view.clone())
    }
}

#[cfg(test)]
mod tests {
    use std::{cell::Cell, rc::Rc};

    use super::*;
    use gpui::{AppContext as _, TestAppContext};

    struct EmptyView;

    impl Render for EmptyView {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div()
        }
    }

    #[gpui::test]
    fn releasing_root_drops_its_theme_subscription(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let subscription_dropped = Rc::new(Cell::new(false));
        let (root, window) = cx.add_window_view(|_, cx| {
            let child = cx.new(|_| EmptyView);
            Root::new(child, cx)
        });
        window.update({
            let subscription_dropped = subscription_dropped.clone();
            let root = root.clone();
            move |_, cx| {
                root.update(cx, |root, _| {
                    root._theme_subscription = Subscription::new(move || {
                        subscription_dropped.set(true);
                    });
                });
            }
        });
        assert!(!subscription_dropped.get());
        let weak_root = root.downgrade();
        drop(root);

        window.update(|window, _| window.remove_window());
        window.run_until_parked();

        assert!(weak_root.upgrade().is_none());
        assert!(subscription_dropped.get());
    }
}
