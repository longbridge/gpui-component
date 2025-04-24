use crate::{h_flex, ActiveTheme, Disableable, Icon, IconName, Selectable, Sizable as _};
use gpui::{
    div, prelude::FluentBuilder as _, AnyElement, App, ClickEvent, Div, ElementId,
    InteractiveElement, IntoElement, MouseButton, MouseMoveEvent, ParentElement, RenderOnce,
    Stateful, StatefulInteractiveElement as _, Styled, Window,
};
use smallvec::SmallVec;

#[derive(IntoElement)]
pub struct ListItem {
    id: ElementId,
    base: Stateful<Div>,
    disabled: bool,
    selected: bool,
    confirmed: bool,
    check_icon: Option<Icon>,
    on_click: Option<Box<dyn Fn(&ClickEvent, &mut Window, &mut App) + 'static>>,
    on_mouse_enter: Option<Box<dyn Fn(&MouseMoveEvent, &mut Window, &mut App) + 'static>>,
    suffix: Option<Box<dyn Fn(&mut Window, &mut App) -> AnyElement + 'static>>,
    children: SmallVec<[AnyElement; 2]>,
}

impl ListItem {
    pub fn new(id: impl Into<ElementId>) -> Self {
        let id: ElementId = id.into();
        Self {
            id: id.clone(),
            base: h_flex().id(id).gap_x_1().py_1().px_2().text_base(),
            disabled: false,
            selected: false,
            confirmed: false,
            on_click: None,
            on_mouse_enter: None,
            check_icon: None,
            suffix: None,
            children: SmallVec::new(),
        }
    }

    /// Set to show check icon, default is None.
    pub fn check_icon(mut self, icon: IconName) -> Self {
        self.check_icon = Some(Icon::new(icon));
        self
    }

    /// Set ListItem as the selected item style.
    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    /// Set ListItem as the confirmed item style, it will show a check icon.
    pub fn confirmed(mut self, confirmed: bool) -> Self {
        self.confirmed = confirmed;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Set the suffix element of the input field, for example a clear button.
    pub fn suffix<F, E>(mut self, builder: F) -> Self
    where
        F: Fn(&mut Window, &mut App) -> E + 'static,
        E: IntoElement,
    {
        self.suffix = Some(Box::new(move |window, cx| {
            builder(window, cx).into_any_element()
        }));
        self
    }

    pub fn on_click(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_click = Some(Box::new(handler));
        self
    }

    pub fn on_mouse_enter(
        mut self,
        handler: impl Fn(&MouseMoveEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_mouse_enter = Some(Box::new(handler));
        self
    }
}

impl Disableable for ListItem {
    fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl Selectable for ListItem {
    fn element_id(&self) -> &ElementId {
        &self.id
    }

    fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }
}

impl Styled for ListItem {
    fn style(&mut self) -> &mut gpui::StyleRefinement {
        self.base.style()
    }
}

impl ParentElement for ListItem {
    fn extend(&mut self, elements: impl IntoIterator<Item = gpui::AnyElement>) {
        self.children.extend(elements);
    }
}

impl RenderOnce for ListItem {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let is_active = self.confirmed || self.selected;

        self.base
            .text_color(cx.theme().foreground)
            .relative()
            .items_center()
            .justify_between()
            .when(!self.disabled, |this| {
                this.when_some(self.on_click, |this, on_click| {
                    this.on_mouse_down(MouseButton::Left, move |_, _, cx| {
                        cx.stop_propagation();
                    })
                    .on_click(on_click)
                })
                .when_some(self.on_mouse_enter, |this, on_mouse_enter| {
                    this.on_mouse_move(move |ev, window, cx| (on_mouse_enter)(ev, window, cx))
                })
                .when(!is_active, |this| {
                    this.hover(|this| this.bg(cx.theme().list_hover))
                })
            })
            .when(self.disabled, |this| {
                this.text_color(cx.theme().muted_foreground)
            })
            .child(
                h_flex()
                    .w_full()
                    .items_center()
                    .justify_between()
                    .gap_x_1()
                    .child(div().w_full().children(self.children))
                    .when_some(self.check_icon, |this, icon| {
                        this.child(
                            div().w_5().items_center().justify_center().when(
                                self.confirmed,
                                |this| {
                                    this.child(icon.small().text_color(cx.theme().muted_foreground))
                                },
                            ),
                        )
                    }),
            )
            .when_some(self.suffix, |this, suffix| this.child(suffix(window, cx)))
    }
}
