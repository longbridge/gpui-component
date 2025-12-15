use std::rc::Rc;

use gpui::{
    AnyElement, App, Axis, ClickEvent, ElementId, InteractiveElement as _, IntoElement,
    ParentElement, RenderOnce, SharedString, StatefulInteractiveElement as _, StyleRefinement,
    Styled, Window, div, prelude::FluentBuilder as _, px,
};

use crate::{ActiveTheme as _, AxisExt, Icon, IconName, Sizable, Size, StyledExt as _};

/// A step-by-step progress for users to navigate through a series of steps or stages.
#[derive(IntoElement)]
pub struct Stepper {
    id: ElementId,
    style: StyleRefinement,
    items: Vec<StepperItem>,
    step: usize,
    layout: Axis,
    disabled: bool,
    size: Size,
    on_click: Rc<dyn Fn(&usize, &mut Window, &mut App) + 'static>,
}

impl Stepper {
    /// Creates a new stepper with the given ID.
    ///
    /// Default use is horizontal layout with step 0 selected.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            style: StyleRefinement::default(),
            items: Vec::new(),
            step: 0,
            layout: Axis::Horizontal,
            disabled: false,
            size: Size::default(),
            on_click: Rc::new(|_, _, _| {}),
        }
    }

    /// Set the layout of the stepper, default is horizontal.
    pub fn layout(mut self, layout: Axis) -> Self {
        self.layout = layout;
        self
    }

    /// Sets the selected index of the stepper.
    pub fn step(mut self, step_index: usize) -> Self {
        self.step = step_index;
        self
    }

    /// Adds a stepper item to the stepper.
    pub fn item(mut self, item: StepperItem) -> Self {
        self.items.push(item);
        self
    }

    /// Add multiple stepper items to the stepper.
    pub fn items(mut self, items: impl IntoIterator<Item = StepperItem>) -> Self {
        self.items.extend(items);
        self
    }

    /// Set the disabled state of the stepper, default is false.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Add an on_click handler for when a step is clicked.
    ///
    /// The first parameter is the `step` of currently clicked item.
    pub fn on_click<F>(mut self, f: F) -> Self
    where
        F: Fn(&usize, &mut Window, &mut App) + 'static,
    {
        self.on_click = Rc::new(f);
        self
    }
}

impl Sizable for Stepper {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }
}

impl Styled for Stepper {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

/// A step item within a [`Stepper`].
#[derive(IntoElement)]
pub struct StepperItem {
    step: usize,
    checked_step: usize,
    icon: Option<Icon>,
    label: Option<SharedString>,
    description: Option<AnyElement>,
    layout: Axis,
    disabled: bool,
    size: Size,
    is_last: bool,
    on_click: Box<dyn Fn(&ClickEvent, &mut Window, &mut App) + 'static>,
}

impl StepperItem {
    pub fn new() -> Self {
        Self {
            step: 0,
            checked_step: 0,
            icon: None,
            label: None,
            description: None,
            layout: Axis::Horizontal,
            disabled: false,
            size: Size::default(),
            is_last: false,
            on_click: Box::new(|_, _, _| {}),
        }
    }

    /// Set the icon of the stepper item.
    pub fn icon(mut self, icon: impl Into<Icon>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    /// Set the label of the stepper item.
    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Set the description of the stepper item.
    pub fn description(mut self, description: impl IntoElement) -> Self {
        self.description = Some(description.into_any_element());
        self
    }

    /// Set disabled state of the stepper item.
    ///
    /// Will override the stepper's disabled state if set to true.
    ///
    /// Default is false.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    fn step(mut self, ix: usize) -> Self {
        self.step = ix;
        self
    }

    fn checked_step(mut self, checked_step: usize) -> Self {
        self.checked_step = checked_step;
        self
    }

    fn layout(mut self, layout: Axis) -> Self {
        self.layout = layout;
        self
    }

    fn is_last(mut self, is_last: bool) -> Self {
        self.is_last = is_last;
        self
    }

    fn on_click<F>(mut self, f: F) -> Self
    where
        F: Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    {
        self.on_click = Box::new(f);
        self
    }
}

impl Sizable for StepperItem {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }
}

impl RenderOnce for StepperItem {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let icon_size = match self.size {
            Size::Small => px(18.),
            Size::Large => px(32.),
            _ => px(24.),
        };
        let separator_size = match self.size {
            Size::Small => px(1.),
            Size::Large => px(3.),
            _ => px(2.),
        };
        let is_checked = self.step <= self.checked_step;
        let is_passed = self.step < self.checked_step;

        div()
            .id(("stepper-item", self.step))
            .relative()
            .when(self.layout.is_horizontal(), |this| this.h_flex())
            .when(self.layout.is_vertical(), |this| this.v_flex())
            .when(!self.is_last, |this| this.flex_1())
            .items_start()
            .child(
                div()
                    .id("stepper-tab")
                    .when(self.layout.is_horizontal(), |this| this.v_flex())
                    .when(self.layout.is_vertical(), |this| this.h_flex())
                    .gap_1()
                    .child(
                        div()
                            .id(self.step)
                            .size(icon_size)
                            .overflow_hidden()
                            .flex()
                            .rounded_full()
                            .items_center()
                            .justify_center()
                            .bg(cx.theme().secondary)
                            .when(!self.disabled && !is_checked, |this| {
                                this.hover(|this| this.bg(cx.theme().secondary_hover))
                            })
                            .text_color(cx.theme().secondary_foreground)
                            .text_xs()
                            .when(is_checked, |this| {
                                this.bg(cx.theme().primary)
                                    .text_color(cx.theme().primary_foreground)
                            })
                            .map(|this| {
                                if is_passed {
                                    this.child(IconName::Check)
                                } else {
                                    this.child(if let Some(icon) = self.icon {
                                        icon.into_any_element()
                                    } else {
                                        div().child(format!("{}", self.step + 1)).into_any_element()
                                    })
                                }
                            }),
                    )
                    .when_some(self.label, |this, label| {
                        this.child(
                            div()
                                .mr_3()
                                .text_sm()
                                .text_color(cx.theme().foreground)
                                .child(label),
                        )
                        .when_some(
                            self.description,
                            |this, description| {
                                this.child(
                                    div()
                                        .mr_3()
                                        .text_xs()
                                        .text_color(cx.theme().muted_foreground)
                                        .child(description),
                                )
                            },
                        )
                    })
                    .on_click(move |event, window, cx| {
                        (self.on_click)(event, window, cx);
                    }),
            )
            .when(!self.is_last, |this| {
                this.child(
                    StepperSeparator::new()
                        .absolute()
                        .when(self.layout.is_horizontal(), |this| {
                            this.h(separator_size)
                                .left(icon_size + px(4.))
                                .top((icon_size - separator_size) / 2.)
                                .right(px(4.))
                        })
                        .when(self.layout.is_vertical(), |this| {
                            this.w(separator_size)
                                .top(icon_size + px(4.))
                                .left((icon_size - separator_size) / 2.)
                                .bottom(px(4.))
                        })
                        .layout(self.layout)
                        .checked(is_passed),
                )
            })
    }
}

/// A separator between stepper items.
#[derive(IntoElement)]
struct StepperSeparator {
    layout: Axis,
    checked: bool,
    style: StyleRefinement,
}

impl StepperSeparator {
    fn new() -> Self {
        Self {
            layout: Axis::Horizontal,
            checked: false,
            style: StyleRefinement::default(),
        }
    }

    fn layout(mut self, layout: Axis) -> Self {
        self.layout = layout;
        self
    }

    fn checked(mut self, checked: bool) -> Self {
        self.checked = checked;
        self
    }
}

impl Styled for StepperSeparator {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for StepperSeparator {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        div()
            .flex_1()
            .refine_style(&self.style)
            .bg(cx.theme().border)
            .when(self.checked, |this| this.bg(cx.theme().primary))
    }
}

impl RenderOnce for Stepper {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let total_items = self.items.len();
        div()
            .id(self.id)
            .when(self.layout.is_horizontal(), |this| this.h_flex())
            .when(self.layout.is_vertical(), |this| this.v_flex())
            .refine_style(&self.style)
            .children(self.items.into_iter().enumerate().map(|(step, item)| {
                let is_last = step + 1 == total_items;
                item.step(step)
                    .checked_step(self.step)
                    .layout(self.layout)
                    .when(self.disabled, |this| this.disabled(true))
                    .is_last(is_last)
                    .on_click({
                        let on_click = self.on_click.clone();
                        move |_, window, cx| {
                            on_click(&step, window, cx);
                        }
                    })
            }))
    }
}
