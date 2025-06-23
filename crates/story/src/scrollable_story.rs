use std::rc::Rc;

use gpui::{
    div, px, size, App, AppContext, Axis, Context, Entity, Focusable, InteractiveElement,
    IntoElement, ParentElement, Pixels, Render, ScrollHandle, SharedString, Size, Styled, Window,
};
use gpui_component::{
    button::{Button, ButtonGroup},
    divider::Divider,
    gray_100, gray_800, h_flex,
    label::Label,
    scroll::{Scrollbar, ScrollbarAxis, ScrollbarState},
    v_flex, v_virtual_list, ActiveTheme as _, Selectable, StyledExt as _,
};

pub struct ScrollableStory {
    focus_handle: gpui::FocusHandle,
    scroll_handle: ScrollHandle,
    scroll_size: gpui::Size<Pixels>,
    scroll_state: ScrollbarState,
    items: Vec<String>,
    item_sizes: Rc<Vec<Size<Pixels>>>,
    test_width: Pixels,
    axis: ScrollbarAxis,
    size_mode: usize,
    message: SharedString,
}

const ITEM_HEIGHT: Pixels = px(30.);

impl ScrollableStory {
    fn new(_: &mut Window, cx: &mut Context<Self>) -> Self {
        let items = (0..5000).map(|i| format!("Item {}", i)).collect::<Vec<_>>();
        let test_width = px(3000.);
        let item_sizes = items
            .iter()
            .map(|_| size(test_width, ITEM_HEIGHT))
            .collect::<Vec<_>>();

        Self {
            focus_handle: cx.focus_handle(),
            scroll_handle: ScrollHandle::new(),
            scroll_state: ScrollbarState::default(),
            scroll_size: gpui::Size::default(),
            items,
            item_sizes: Rc::new(item_sizes),
            test_width,
            axis: ScrollbarAxis::Both,
            size_mode: 0,
            message: SharedString::default(),
        }
    }

    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    pub fn change_test_cases(&mut self, n: usize, cx: &mut Context<Self>) {
        self.size_mode = n;
        if n == 0 {
            self.items = (0..5000).map(|i| format!("Item {}", i)).collect::<Vec<_>>();
            self.test_width = px(3000.);
        } else if n == 1 {
            self.items = (0..100).map(|i| format!("Item {}", i)).collect::<Vec<_>>();
            self.test_width = px(10000.);
        } else if n == 2 {
            self.items = (0..500000)
                .map(|i| format!("Item {}", i))
                .collect::<Vec<_>>();
            self.test_width = px(10000.);
        } else {
            self.items = (0..5).map(|i| format!("Item {}", i)).collect::<Vec<_>>();
            self.test_width = px(10000.);
        }

        self.item_sizes = self
            .items
            .iter()
            .map(|_| size(self.test_width, ITEM_HEIGHT))
            .collect::<Vec<_>>()
            .into();
        self.scroll_state = ScrollbarState::default();
        cx.notify();
    }

    pub fn change_axis(&mut self, axis: ScrollbarAxis, cx: &mut Context<Self>) {
        self.axis = axis;
        cx.notify();
    }

    fn set_message(&mut self, msg: &str, cx: &mut Context<Self>) {
        self.message = SharedString::from(msg.to_string());
        cx.notify();
    }

    fn render_buttons(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        h_flex()
            .gap_2()
            .justify_between()
            .child(
                h_flex()
                    .gap_2()
                    .child(
                        ButtonGroup::new("test-cases")
                            .outline()
                            .compact()
                            .child(
                                Button::new("test-0")
                                    .label("Size 0")
                                    .selected(self.size_mode == 0),
                            )
                            .child(
                                Button::new("test-1")
                                    .label("Size 1")
                                    .selected(self.size_mode == 1),
                            )
                            .child(
                                Button::new("test-2")
                                    .label("Size 2")
                                    .selected(self.size_mode == 2),
                            )
                            .child(
                                Button::new("test-3")
                                    .label("Size 3")
                                    .selected(self.size_mode == 3),
                            )
                            .on_click(cx.listener(|view, clicks: &Vec<usize>, _, cx| {
                                if clicks.contains(&0) {
                                    view.change_test_cases(0, cx)
                                } else if clicks.contains(&1) {
                                    view.change_test_cases(1, cx)
                                } else if clicks.contains(&2) {
                                    view.change_test_cases(2, cx)
                                } else if clicks.contains(&3) {
                                    view.change_test_cases(3, cx)
                                }
                            })),
                    )
                    .child(Divider::vertical().px_2())
                    .child(
                        ButtonGroup::new("scrollbars")
                            .outline()
                            .compact()
                            .child(
                                Button::new("test-axis-both")
                                    .label("Both Scrollbar")
                                    .selected(self.axis.is_both()),
                            )
                            .child(
                                Button::new("test-axis-vertical")
                                    .label("Vertical")
                                    .selected(self.axis.is_vertical()),
                            )
                            .child(
                                Button::new("test-axis-horizontal")
                                    .label("Horizontal")
                                    .selected(self.axis.is_horizontal()),
                            )
                            .on_click(cx.listener(|view, clicks: &Vec<usize>, _, cx| {
                                if clicks.contains(&0) {
                                    view.change_axis(ScrollbarAxis::Both, cx)
                                } else if clicks.contains(&1) {
                                    view.change_axis(ScrollbarAxis::Vertical, cx)
                                } else if clicks.contains(&2) {
                                    view.change_axis(ScrollbarAxis::Horizontal, cx)
                                }
                            })),
                    ),
            )
            .child(Label::new(self.message.clone()))
    }
}

impl super::Story for ScrollableStory {
    fn title() -> &'static str {
        "Scrollable"
    }

    fn description() -> &'static str {
        "Add vertical or horizontal, or both scrollbars to a container, \
        and use `virtual_list` to render a large number of items."
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }
}

impl Focusable for ScrollableStory {
    fn focus_handle(&self, _: &gpui::App) -> gpui::FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for ScrollableStory {
    fn render(
        &mut self,
        _: &mut gpui::Window,
        cx: &mut gpui::Context<Self>,
    ) -> impl gpui::IntoElement {
        v_flex()
            .size_full()
            .gap_4()
            .child(self.render_buttons(cx))
            .child(
                div().w_full().flex_1().min_h_64().child(
                    div().relative().size_full().child(
                        v_flex()
                            .id("test-0")
                            .relative()
                            .size_full()
                            .child(
                                v_virtual_list(
                                    cx.entity().clone(),
                                    "items",
                                    self.item_sizes.clone(),
                                    move |story, visible_range, content_size, _, cx| {
                                        story.set_message(
                                            &format!("visible_range: {:?}", visible_range),
                                            cx,
                                        );
                                        story.scroll_size = content_size;
                                        visible_range
                                            .map(|ix| {
                                                h_flex()
                                                    .h(ITEM_HEIGHT)
                                                    .gap_1()
                                                    .children(
                                                        (0..(story.test_width.0 as i32 / 100))
                                                            .map(|i| {
                                                                div()
                                                                    .flex()
                                                                    .h_full()
                                                                    .items_center()
                                                                    .justify_center()
                                                                    .text_sm()
                                                                    .w(px(100.))
                                                                    .bg(
                                                                        if cx.theme().mode.is_dark()
                                                                        {
                                                                            gray_800()
                                                                        } else {
                                                                            gray_100()
                                                                        },
                                                                    )
                                                                    .child(if i == 0 {
                                                                        format!("{}", ix)
                                                                    } else {
                                                                        format!("{}", i)
                                                                    })
                                                            })
                                                            .collect::<Vec<_>>(),
                                                    )
                                                    .items_center()
                                            })
                                            .collect::<Vec<_>>()
                                    },
                                )
                                .track_scroll(&self.scroll_handle)
                                .p_4()
                                .border_1()
                                .border_color(cx.theme().border)
                                .v_flex()
                                .gap_1(),
                            )
                            .child({
                                div()
                                    .absolute()
                                    .top_0()
                                    .left_0()
                                    .right_0()
                                    .bottom_0()
                                    .child(
                                        Scrollbar::both(&self.scroll_state, &self.scroll_handle)
                                            .axis(self.axis),
                                    )
                            }),
                    ),
                ),
            )
            .child({
                div()
                    .relative()
                    .border_1()
                    .border_color(cx.theme().border)
                    .w_full()
                    .max_h(px(400.))
                    .min_h(px(200.))
                    .child(
                        v_flex()
                            .p_3()
                            .w(self.test_width)
                            .id("test-1")
                            .scrollable(Axis::Vertical)
                            .gap_1()
                            .child("Scrollable Example")
                            .children(self.items.iter().take(500).map(|item| {
                                div()
                                    .h(ITEM_HEIGHT)
                                    .bg(cx.theme().background)
                                    .items_center()
                                    .justify_center()
                                    .text_sm()
                                    .child(item.to_string())
                            })),
                    )
            })
    }
}
