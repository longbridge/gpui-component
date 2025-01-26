use std::rc::Rc;

use crate::{h_flex, v_flex, ActiveTheme, AxisExt, IconName};
use gpui::{
    div, prelude::FluentBuilder, relative, svg, App, Axis, ElementId, InteractiveElement,
    IntoElement, ParentElement, RenderOnce, SharedString, StatefulInteractiveElement, Styled,
    Window,
};

/// A Radio element.
///
/// This is not included the Radio group implementation, you can manage the group by yourself.
#[derive(IntoElement)]
pub struct Radio {
    id: ElementId,
    label: Option<SharedString>,
    checked: bool,
    disabled: bool,
    on_click: Option<Box<dyn Fn(&bool, &mut Window, &mut App) + 'static>>,
}

impl Radio {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            label: None,
            checked: false,
            disabled: false,
            on_click: None,
        }
    }

    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn checked(mut self, checked: bool) -> Self {
        self.checked = checked;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn on_click(mut self, handler: impl Fn(&bool, &mut Window, &mut App) + 'static) -> Self {
        self.on_click = Some(Box::new(handler));
        self
    }
}

impl RenderOnce for Radio {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let color = if self.disabled {
            cx.theme().primary.opacity(0.5)
        } else {
            cx.theme().primary
        };

        // wrap a flex to patch for let Radio display inline
        h_flex().child(
            h_flex()
                .id(self.id)
                .gap_x_2()
                .text_color(cx.theme().foreground)
                .items_center()
                .line_height(relative(1.))
                .child(
                    div()
                        .relative()
                        .size_4()
                        .flex_shrink_0()
                        .rounded_full()
                        .border_1()
                        .border_color(color)
                        .when(self.checked, |this| this.bg(color))
                        .child(
                            svg()
                                .absolute()
                                .top_px()
                                .left_px()
                                .size_3()
                                .text_color(color)
                                .when(self.checked, |this| {
                                    this.text_color(cx.theme().primary_foreground)
                                })
                                .map(|this| match self.checked {
                                    true => this.path(IconName::Check.path()),
                                    false => this,
                                }),
                        ),
                )
                .when_some(self.label, |this, label| {
                    this.child(
                        div()
                            .size_full()
                            .overflow_x_hidden()
                            .text_ellipsis()
                            .line_height(relative(1.))
                            .child(label),
                    )
                })
                .when_some(
                    self.on_click.filter(|_| !self.disabled),
                    |this, on_click| {
                        this.on_click(move |_event, window, cx| {
                            on_click(&!self.checked, window, cx);
                        })
                    },
                ),
        )
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
        Self::new(SharedString::from(label.clone())).label(SharedString::from(label))
    }
}

/// A Radio group element.
#[derive(IntoElement)]
pub struct RadioGroup {
    radios: Vec<Radio>,
    layout: Axis,
    selected_index: Option<usize>,
    disabled: bool,
    on_change: Option<Rc<dyn Fn(&usize, &mut Window, &mut App) + 'static>>,
}

impl RadioGroup {
    fn new() -> Self {
        Self {
            on_change: None,
            layout: Axis::Vertical,
            selected_index: None,
            disabled: false,
            radios: vec![],
        }
    }

    /// Create a new Radio group with default Vertical layout.
    pub fn vertical() -> Self {
        Self::new()
    }

    /// Create a new Radio group with Horizontal layout.
    pub fn horizontal() -> Self {
        Self::new().layout(Axis::Horizontal)
    }

    /// Set the layout of the Radio group. Default is `Axis::Vertical`.
    pub fn layout(mut self, layout: Axis) -> Self {
        self.layout = layout;
        self
    }

    /// Listen to the change event.
    pub fn on_change(mut self, handler: impl Fn(&usize, &mut Window, &mut App) + 'static) -> Self {
        self.on_change = Some(Rc::new(handler));
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

impl RenderOnce for RadioGroup {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let on_change = self.on_change;
        let disabled = self.disabled;
        let selected_ix = self.selected_index;

        let base = if self.layout.is_vertical() {
            v_flex()
        } else {
            h_flex().flex_wrap()
        };

        div().flex().child(
            base.gap_3()
                .children(self.radios.into_iter().enumerate().map(|(ix, radio)| {
                    let checked = selected_ix == Some(ix);

                    radio.disabled(disabled).checked(checked).when_some(
                        on_change.clone(),
                        |this, on_change| {
                            this.on_click(move |_, window, cx| {
                                on_change(&ix, window, cx);
                            })
                        },
                    )
                })),
        )
    }
}
