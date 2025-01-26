use std::{cell::Cell, rc::Rc, sync::Arc};

use gpui::{
    div, prelude::FluentBuilder as _, rems, AnyElement, App, Div, ElementId,
    InteractiveElement as _, IntoElement, ParentElement, RenderOnce, SharedString,
    StatefulInteractiveElement as _, Styled, Window,
};

use crate::{h_flex, v_flex, ActiveTheme as _, Icon, IconName, Sizable, Size};

/// An AccordionGroup is a container for multiple Accordion elements.
#[derive(IntoElement)]
pub struct Accordion {
    id: ElementId,
    base: Div,
    multiple: bool,
    size: Size,
    bordered: bool,
    disabled: bool,
    children: Vec<AccordionItem>,
    on_toggle_click: Option<Arc<dyn Fn(&[usize], &mut Window, &mut App) + Send + Sync>>,
}

impl Accordion {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            base: v_flex().gap_1(),
            multiple: false,
            size: Size::default(),
            bordered: true,
            children: Vec::new(),
            disabled: false,
            on_toggle_click: None,
        }
    }

    pub fn multiple(mut self, multiple: bool) -> Self {
        self.multiple = multiple;
        self
    }

    pub fn bordered(mut self, bordered: bool) -> Self {
        self.bordered = bordered;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn item<F>(mut self, child: F) -> Self
    where
        F: FnOnce(AccordionItem) -> AccordionItem,
    {
        let item = child(AccordionItem::new());
        self.children.push(item);
        self
    }

    /// Sets the on_toggle_click callback for the AccordionGroup.
    ///
    /// The first argument `Vec<usize>` is the indices of the open accordions.
    pub fn on_toggle_click(
        mut self,
        on_toggle_click: impl Fn(&[usize], &mut Window, &mut App) + Send + Sync + 'static,
    ) -> Self {
        self.on_toggle_click = Some(Arc::new(on_toggle_click));
        self
    }
}

impl Sizable for Accordion {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }
}

impl RenderOnce for Accordion {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let mut open_ixs: Vec<usize> = Vec::new();
        let multiple = self.multiple;
        let state = Rc::new(Cell::new(None));

        self.children
            .iter()
            .enumerate()
            .for_each(|(ix, accordion)| {
                if accordion.open {
                    open_ixs.push(ix);
                }
            });

        self.base
            .id(self.id)
            .children(
                self.children
                    .into_iter()
                    .enumerate()
                    .map(|(ix, accordion)| {
                        let state = Rc::clone(&state);
                        accordion
                            .with_size(self.size)
                            .bordered(self.bordered)
                            .when(self.disabled, |this| this.disabled(true))
                            .on_toggle_click(move |_, _, _| {
                                state.set(Some(ix));
                            })
                    }),
            )
            .when_some(
                self.on_toggle_click.filter(|_| !self.disabled),
                move |this, on_toggle_click| {
                    this.on_click(move |_, window, cx| {
                        let mut open_ixs = open_ixs.clone();
                        if let Some(ix) = state.get() {
                            if multiple {
                                if let Some(pos) = open_ixs.iter().position(|&i| i == ix) {
                                    open_ixs.remove(pos);
                                } else {
                                    open_ixs.push(ix);
                                }
                            } else {
                                let was_open = open_ixs.iter().any(|&i| i == ix);
                                open_ixs.clear();
                                if !was_open {
                                    open_ixs.push(ix);
                                }
                            }
                        }

                        on_toggle_click(&open_ixs, window, cx);
                    })
                },
            )
    }
}

/// An Accordion is a vertically stacked list of items, each of which can be expanded to reveal the content associated with it.
#[derive(IntoElement)]
pub struct AccordionItem {
    icon: Option<Icon>,
    title: AnyElement,
    content: AnyElement,
    open: bool,
    size: Size,
    bordered: bool,
    disabled: bool,
    on_toggle_click: Option<Arc<dyn Fn(&bool, &mut Window, &mut App)>>,
}

impl AccordionItem {
    pub fn new() -> Self {
        Self {
            icon: None,
            title: SharedString::default().into_any_element(),
            content: SharedString::default().into_any_element(),
            open: false,
            disabled: false,
            on_toggle_click: None,
            size: Size::default(),
            bordered: true,
        }
    }

    pub fn icon(mut self, icon: impl Into<Icon>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    pub fn title(mut self, title: impl IntoElement) -> Self {
        self.title = title.into_any_element();
        self
    }

    pub fn content(mut self, content: impl IntoElement) -> Self {
        self.content = content.into_any_element();
        self
    }

    pub fn bordered(mut self, bordered: bool) -> Self {
        self.bordered = bordered;
        self
    }

    pub fn open(mut self, open: bool) -> Self {
        self.open = open;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    fn on_toggle_click(
        mut self,
        on_toggle_click: impl Fn(&bool, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_toggle_click = Some(Arc::new(on_toggle_click));
        self
    }
}

impl Sizable for AccordionItem {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }
}

impl RenderOnce for AccordionItem {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let text_size = match self.size {
            Size::XSmall => rems(0.875),
            Size::Small => rems(0.875),
            _ => rems(1.0),
        };

        v_flex()
            .bg(cx.theme().accordion)
            .overflow_hidden()
            .when(self.bordered, |this| {
                this.border_1().border_color(cx.theme().border).rounded_md()
            })
            .text_size(text_size)
            .child(
                h_flex()
                    .id("accordion-title")
                    .justify_between()
                    .map(|this| match self.size {
                        Size::XSmall => this.py_0().px_1p5(),
                        Size::Small => this.py_0p5().px_2(),
                        Size::Large => this.py_1p5().px_4(),
                        _ => this.py_1().px_3(),
                    })
                    .when(self.open, |this| {
                        this.when(self.bordered, |this| {
                            this.bg(cx.theme().accordion_active)
                                .text_color(cx.theme().foreground)
                                .border_b_1()
                                .border_color(cx.theme().border)
                        })
                    })
                    .child(
                        h_flex()
                            .items_center()
                            .map(|this| match self.size {
                                Size::XSmall => this.gap_1(),
                                Size::Small => this.gap_1(),
                                _ => this.gap_2(),
                            })
                            .when_some(self.icon, |this, icon| {
                                this.child(
                                    icon.with_size(self.size)
                                        .text_color(cx.theme().muted_foreground),
                                )
                            })
                            .child(self.title),
                    )
                    .when(!self.disabled, |this| {
                        this.cursor_pointer()
                            .hover(|this| this.bg(cx.theme().accordion_hover))
                            .child(
                                Icon::new(if self.open {
                                    IconName::ChevronUp
                                } else {
                                    IconName::ChevronDown
                                })
                                .xsmall()
                                .text_color(cx.theme().muted_foreground),
                            )
                    })
                    .when_some(
                        self.on_toggle_click.filter(|_| !self.disabled),
                        |this, on_toggle_click| {
                            this.on_click({
                                move |_, window, cx| {
                                    on_toggle_click(&!self.open, window, cx);
                                }
                            })
                        },
                    ),
            )
            .when(self.open, |this| {
                this.child(
                    div()
                        .map(|this| match self.size {
                            Size::XSmall => this.p_1p5(),
                            Size::Small => this.p_2(),
                            Size::Large => this.p_4(),
                            _ => this.p_3(),
                        })
                        .child(self.content),
                )
            })
    }
}
