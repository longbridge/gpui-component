use std::rc::Rc;

use crate::{
    actions::Confirm, text::Text, v_flex, ActiveTheme, Disableable, FocusableExt, IconName,
    Selectable, Sizable, Size, StyledExt as _,
};
use gpui::{
    div, prelude::FluentBuilder as _, px, relative, rems, svg, AnyElement, App, Div, ElementId,
    Entity, InteractiveElement, IntoElement, KeyBinding, ParentElement, RenderOnce,
    StatefulInteractiveElement, StyleRefinement, Styled, Window,
};

const KEY_CONTENT: &str = "Checkbox";
pub(super) fn init(cx: &mut App) {
    cx.bind_keys(vec![
        // Add key bindings for button actions if needed
        KeyBinding::new("enter", Confirm { secondary: false }, Some(KEY_CONTENT)),
        KeyBinding::new("space", Confirm { secondary: false }, Some(KEY_CONTENT)),
    ]);
}

/// A Checkbox element.
#[derive(IntoElement)]
pub struct Checkbox {
    id: ElementId,
    base: Div,
    style: StyleRefinement,
    label: Option<Text>,
    children: Vec<AnyElement>,
    default_checked: bool,
    disabled: bool,
    size: Size,
    tab_stop: bool,
    tab_index: isize,
    on_click: Option<Rc<dyn Fn(&bool, &mut Window, &mut App) + 'static>>,
}

impl Checkbox {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            base: div(),
            style: StyleRefinement::default(),
            label: None,
            children: Vec::new(),
            default_checked: false,
            disabled: false,
            size: Size::default(),
            on_click: None,
            tab_stop: true,
            tab_index: 0,
        }
    }

    pub fn label(mut self, label: impl Into<Text>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn checked(mut self, checked: bool) -> Self {
        self.default_checked = checked;
        self
    }

    pub fn on_click(mut self, handler: impl Fn(&bool, &mut Window, &mut App) + 'static) -> Self {
        self.on_click = Some(Rc::new(handler));
        self
    }

    /// Set the tab stop for the checkbox, default is true.
    pub fn tab_stop(mut self, tab_stop: bool) -> Self {
        self.tab_stop = tab_stop;
        self
    }

    /// Set the tab index for the checkbox, default is 0.
    pub fn tab_index(mut self, tab_index: isize) -> Self {
        self.tab_index = tab_index;
        self
    }

    fn handle_click(
        on_click: &Option<Rc<dyn Fn(&bool, &mut Window, &mut App) + 'static>>,
        checked_state: &Entity<bool>,
        window: &mut Window,
        cx: &mut App,
    ) {
        let new_checked = !*checked_state.read(cx);
        checked_state.update(cx, |checked, cx| {
            *checked = new_checked;
            cx.notify();
        });

        if let Some(f) = on_click {
            (f)(&new_checked, window, cx);
        }
    }
}

impl InteractiveElement for Checkbox {
    fn interactivity(&mut self) -> &mut gpui::Interactivity {
        self.base.interactivity()
    }
}
impl StatefulInteractiveElement for Checkbox {}

impl Styled for Checkbox {
    fn style(&mut self) -> &mut gpui::StyleRefinement {
        &mut self.style
    }
}

impl Disableable for Checkbox {
    fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl Selectable for Checkbox {
    fn element_id(&self) -> &ElementId {
        &self.id
    }

    fn selected(self, selected: bool) -> Self {
        self.checked(selected)
    }

    fn is_selected(&self) -> bool {
        self.default_checked
    }
}

impl ParentElement for Checkbox {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl Sizable for Checkbox {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }
}

impl RenderOnce for Checkbox {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let default_checked = self.default_checked;

        let focus_handle = window
            .use_keyed_state(self.id.clone(), cx, |_, cx| cx.focus_handle())
            .read(cx)
            .clone();

        let checked_state = window.use_keyed_state(
            ElementId::Name(format!("{}:checked", self.id.clone()).into()),
            cx,
            |_, _| default_checked,
        );

        let checked = *checked_state.read(cx);

        let border_color = if checked {
            cx.theme().primary
        } else {
            cx.theme().input
        };
        let (color, icon_color) = if self.disabled {
            (
                border_color.opacity(0.5),
                cx.theme().primary_foreground.opacity(0.5),
            )
        } else {
            (border_color, cx.theme().primary_foreground)
        };
        let radius = cx.theme().radius.min(px(4.));

        let is_focused = focus_handle.is_focused(window);

        div().child(
            self.base
                .id(self.id)
                .key_context(KEY_CONTENT)
                .track_focus(
                    &focus_handle
                        .tab_stop(self.tab_stop)
                        .tab_index(self.tab_index),
                )
                .when(!self.disabled, |this| {
                    this.on_action({
                        let checked_state = checked_state.clone();
                        let on_click = self.on_click.clone();
                        move |_: &Confirm, window, cx| {
                            Self::handle_click(&on_click, &checked_state, window, cx);
                        }
                    })
                })
                .h_flex()
                .gap_2()
                .items_start()
                .line_height(relative(1.))
                .text_color(cx.theme().foreground)
                .map(|this| match self.size {
                    Size::XSmall => this.text_xs(),
                    Size::Small => this.text_sm(),
                    Size::Medium => this.text_base(),
                    Size::Large => this.text_lg(),
                    _ => this,
                })
                .when(self.disabled, |this| {
                    this.text_color(cx.theme().muted_foreground)
                })
                .rounded(cx.theme().radius * 0.5)
                .focus_ring(is_focused, px(2.), window, cx)
                .refine_style(&self.style)
                .child(
                    div()
                        .relative()
                        .map(|this| match self.size {
                            Size::XSmall => this.size_3(),
                            Size::Small => this.size_3p5(),
                            Size::Medium => this.size_4(),
                            Size::Large => this.size(rems(1.125)),
                            _ => this.size_4(),
                        })
                        .flex_shrink_0()
                        .border_1()
                        .border_color(color)
                        .rounded(radius)
                        .when(cx.theme().shadow && !self.disabled, |this| this.shadow_xs())
                        .map(|this| match checked {
                            false => this.bg(cx.theme().background),
                            _ => this.bg(color),
                        })
                        .child(
                            svg()
                                .absolute()
                                .top_px()
                                .left_px()
                                .map(|this| match self.size {
                                    Size::XSmall => this.size_2(),
                                    Size::Small => this.size_2p5(),
                                    Size::Medium => this.size_3(),
                                    Size::Large => this.size_3p5(),
                                    _ => this.size_3(),
                                })
                                .text_color(icon_color)
                                .map(|this| match checked {
                                    true => this.path(IconName::Check.path()),
                                    _ => this,
                                }),
                        ),
                )
                .when(self.label.is_some() || !self.children.is_empty(), |this| {
                    this.child(
                        v_flex()
                            .w_full()
                            .line_height(relative(1.2))
                            .gap_1()
                            .map(|this| {
                                if let Some(label) = self.label {
                                    this.child(
                                        div()
                                            .size_full()
                                            .text_color(cx.theme().foreground)
                                            .when(self.disabled, |this| {
                                                this.text_color(cx.theme().muted_foreground)
                                            })
                                            .line_height(relative(1.))
                                            .child(label),
                                    )
                                } else {
                                    this
                                }
                            })
                            .children(self.children),
                    )
                })
                .when(!self.disabled, |this| {
                    this.on_click({
                        let on_click = self.on_click.clone();
                        move |_, window, cx| {
                            cx.stop_propagation();

                            Self::handle_click(&on_click, &checked_state, window, cx);
                        }
                    })
                }),
        )
    }
}
