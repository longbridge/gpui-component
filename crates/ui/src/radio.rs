use std::rc::Rc;

use crate::{
    ActiveTheme, AxisExt, FocusableExt as _, Sizable, Size, StyledExt,
    checkbox::checkbox_check_icon, h_flex, text::Text, v_flex,
};
use gpui::{
    AnyElement, App, Axis, Div, ElementId, InteractiveElement, IntoElement, ParentElement,
    RenderOnce, SharedString, StatefulInteractiveElement, StyleRefinement, Styled, Window, div,
    prelude::FluentBuilder, px, relative, rems,
};

/// A Radio element.
///
/// This is not included the Radio group implementation, you can manage the group by yourself.
#[derive(IntoElement)]
pub struct Radio {
    base: Div,
    style: StyleRefinement,
    id: ElementId,
    label: Option<Text>,
    children: Vec<AnyElement>,
    checked: bool,
    disabled: bool,
    tab_stop: bool,
    tab_index: isize,
    size: Size,
    on_click: Option<Rc<dyn Fn(&bool, &mut Window, &mut App) + 'static>>,
}

impl Radio {
    /// Create a new Radio element with the given id.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            base: div(),
            style: StyleRefinement::default(),
            label: None,
            children: Vec::new(),
            checked: false,
            disabled: false,
            tab_index: 0,
            tab_stop: true,
            size: Size::default(),
            on_click: None,
        }
    }

    /// Set the label of the Radio element.
    pub fn label(mut self, label: impl Into<Text>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Set the checked state of the Radio element, default is `false`.
    pub fn checked(mut self, checked: bool) -> Self {
        self.checked = checked;
        self
    }

    /// Set the disabled state of the Radio element, default is `false`.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Set the tab index for the Radio element, default is `0`.
    pub fn tab_index(mut self, tab_index: isize) -> Self {
        self.tab_index = tab_index;
        self
    }

    /// Set the tab stop for the Radio element, default is `true`.
    pub fn tab_stop(mut self, tab_stop: bool) -> Self {
        self.tab_stop = tab_stop;
        self
    }

    /// Add on_click handler when the Radio is clicked.
    ///
    /// The `&bool` parameter is the **new checked state**.
    pub fn on_click(mut self, handler: impl Fn(&bool, &mut Window, &mut App) + 'static) -> Self {
        self.on_click = Some(Rc::new(handler));
        self
    }

    fn handle_click(
        on_click: &Option<Rc<dyn Fn(&bool, &mut Window, &mut App) + 'static>>,
        checked: bool,
        window: &mut Window,
        cx: &mut App,
    ) {
        let new_checked = !checked;
        if let Some(f) = on_click {
            (f)(&new_checked, window, cx);
        }
    }
}

impl Sizable for Radio {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }
}

impl Styled for Radio {
    fn style(&mut self) -> &mut gpui::StyleRefinement {
        &mut self.style
    }
}

impl InteractiveElement for Radio {
    fn interactivity(&mut self) -> &mut gpui::Interactivity {
        self.base.interactivity()
    }
}

impl StatefulInteractiveElement for Radio {}

impl ParentElement for Radio {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl RenderOnce for Radio {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let checked = self.checked;
        let focus_handle = window
            .use_keyed_state(self.id.clone(), cx, |_, cx| cx.focus_handle())
            .read(cx)
            .clone();
        let is_focused = focus_handle.is_focused(window);
        let disabled = self.disabled;

        let (border_color, bg) = if checked {
            (cx.theme().primary, cx.theme().primary)
        } else {
            (cx.theme().input, cx.theme().input.opacity(0.3))
        };
        let (border_color, bg) = if disabled {
            (border_color.opacity(0.5), bg.opacity(0.5))
        } else {
            (border_color, bg)
        };

        // wrap a flex to patch for let Radio display inline
        div().child(
            self.base
                .id(self.id.clone())
                .when(!self.disabled, |this| {
                    this.track_focus(
                        &focus_handle
                            .tab_stop(self.tab_stop)
                            .tab_index(self.tab_index),
                    )
                })
                .h_flex()
                .gap_x_2()
                .text_color(cx.theme().foreground)
                .items_start()
                .line_height(relative(1.))
                .rounded(cx.theme().radius * 0.5)
                .focus_ring(is_focused, px(2.), window, cx)
                .map(|this| match self.size {
                    Size::XSmall => this.text_xs(),
                    Size::Small => this.text_sm(),
                    Size::Medium => this.text_base(),
                    Size::Large => this.text_lg(),
                    _ => this,
                })
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
                        .rounded_full()
                        .border_1()
                        .border_color(border_color)
                        .when(cx.theme().shadow && !disabled, |this| this.shadow_xs())
                        .map(|this| match self.checked {
                            false => this.bg(cx.theme().background),
                            _ => this.bg(bg),
                        })
                        .child(checkbox_check_icon(
                            self.id, self.size, checked, disabled, window, cx,
                        )),
                )
                .when(!self.children.is_empty() || self.label.is_some(), |this| {
                    this.child(
                        v_flex()
                            .w_full()
                            .line_height(relative(1.2))
                            .gap_1()
                            .when_some(self.label, |this, label| {
                                this.child(
                                    div()
                                        .size_full()
                                        .overflow_hidden()
                                        .line_height(relative(1.))
                                        .when(self.disabled, |this| {
                                            this.text_color(cx.theme().muted_foreground)
                                        })
                                        .child(label),
                                )
                            })
                            .children(self.children),
                    )
                })
                .on_mouse_down(gpui::MouseButton::Left, |_, window, _| {
                    // Avoid focus on mouse down.
                    window.prevent_default();
                })
                .when(!self.disabled, |this| {
                    this.on_click({
                        let on_click = self.on_click.clone();
                        move |_, window, cx| {
                            window.prevent_default();
                            Self::handle_click(&on_click, checked, window, cx);
                        }
                    })
                }),
        )
    }
}

/// A Radio group element.
#[derive(IntoElement)]
pub struct RadioGroup {
    id: ElementId,
    style: StyleRefinement,
    radios: Vec<Radio>,
    layout: Axis,
    selected_index: Option<usize>,
    disabled: bool,
    on_click: Option<Rc<dyn Fn(&usize, &mut Window, &mut App) + 'static>>,
}

impl RadioGroup {
    fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            style: StyleRefinement::default().flex_1(),
            on_click: None,
            layout: Axis::Vertical,
            selected_index: None,
            disabled: false,
            radios: vec![],
        }
    }

    /// Create a new Radio group with default Vertical layout.
    pub fn vertical(id: impl Into<ElementId>) -> Self {
        Self::new(id)
    }

    /// Create a new Radio group with Horizontal layout.
    pub fn horizontal(id: impl Into<ElementId>) -> Self {
        Self::new(id).layout(Axis::Horizontal)
    }

    /// Set the layout of the Radio group. Default is `Axis::Vertical`.
    pub fn layout(mut self, layout: Axis) -> Self {
        self.layout = layout;
        self
    }

    // Add on_click handler when selected index changes.
    //
    // The `&usize` parameter is the selected index.
    pub fn on_click(mut self, handler: impl Fn(&usize, &mut Window, &mut App) + 'static) -> Self {
        self.on_click = Some(Rc::new(handler));
        self
    }

    /// Set the selected index.
    pub fn selected_index(mut self, index: Option<usize>) -> Self {
        self.selected_index = index;
        self
    }

    /// Set the disabled state.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Add a child Radio element.
    pub fn child(mut self, child: impl Into<Radio>) -> Self {
        self.radios.push(child.into());
        self
    }

    /// Add multiple child Radio elements.
    pub fn children(mut self, children: impl IntoIterator<Item = impl Into<Radio>>) -> Self {
        self.radios.extend(children.into_iter().map(Into::into));
        self
    }
}

impl Styled for RadioGroup {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl From<&'static str> for Radio {
    fn from(label: &'static str) -> Self {
        Self::new(label).label(label)
    }
}

impl From<SharedString> for Radio {
    fn from(label: SharedString) -> Self {
        Self::new(label.clone()).label(label)
    }
}

impl From<String> for Radio {
    fn from(label: String) -> Self {
        Self::new(label.clone()).label(label)
    }
}

impl RenderOnce for RadioGroup {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let on_click = self.on_click;
        let disabled = self.disabled;
        let selected_ix = self.selected_index;

        let base = if self.layout.is_vertical() {
            v_flex()
        } else {
            h_flex().w_full().flex_wrap()
        };

        let mut container = div().id(self.id);
        *container.style() = self.style;

        container.child(
            base.gap_3()
                .children(self.radios.into_iter().enumerate().map(|(ix, mut radio)| {
                    let checked = selected_ix == Some(ix);

                    radio.id = ix.into();
                    radio.disabled(disabled).checked(checked).when_some(
                        on_click.clone(),
                        |this, on_click| {
                            this.on_click(move |_, window, cx| {
                                on_click(&ix, window, cx);
                            })
                        },
                    )
                })),
        )
    }
}
