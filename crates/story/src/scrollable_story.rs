use std::cell::Cell;
use std::rc::Rc;

use gpui::{Window, ModelContext, AppContext, Model, 
    div, px, size, Entity, InteractiveElement, IntoElement, ParentElement, Pixels, Render,
    ScrollHandle, SharedString, Size, Styled,   VisualContext, 
};
use ui::button::Button;
use ui::divider::Divider;
use ui::label::Label;
use ui::scroll::{Scrollbar, ScrollbarAxis, ScrollbarState};
use ui::ActiveTheme;
use ui::{h_flex, v_flex, v_virtual_list, StyledExt as _};

pub struct ScrollableStory {
    focus_handle: gpui::FocusHandle,
    scroll_handle: ScrollHandle,
    scroll_size: gpui::Size<Pixels>,
    scroll_state: Rc<Cell<ScrollbarState>>,
    items: Vec<String>,
    item_sizes: Rc<Vec<Size<Pixels>>>,
    test_width: Pixels,
    axis: ScrollbarAxis,
    message: SharedString,
}

const ITEM_HEIGHT: Pixels = px(30.);

impl ScrollableStory {
    fn new(cx: &mut ViewContext<Self>) -> Self {
        let items = (0..5000).map(|i| format!("Item {}", i)).collect::<Vec<_>>();
        let test_width = px(3000.);
        let item_sizes = items
            .iter()
            .map(|_| size(test_width, ITEM_HEIGHT))
            .collect::<Vec<_>>();

        Self {
            focus_handle: cx.focus_handle(),
            scroll_handle: ScrollHandle::new(),
            scroll_state: Rc::new(Cell::new(ScrollbarState::default())),
            scroll_size: gpui::Size::default(),
            items,
            item_sizes: Rc::new(item_sizes),
            test_width,
            axis: ScrollbarAxis::Both,
            message: SharedString::default(),
        }
    }

    pub fn view(cx: &mut WindowContext) -> View<Self> {
        cx.new_view(Self::new)
    }

    pub fn change_test_cases(&mut self, n: usize, cx: &mut ViewContext<Self>) {
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
        self.scroll_state.set(ScrollbarState::default());
        cx.notify();
    }

    pub fn change_axis(&mut self, axis: ScrollbarAxis, cx: &mut ViewContext<Self>) {
        self.axis = axis;
        cx.notify();
    }

    fn set_message(&mut self, msg: &str, cx: &mut ViewContext<Self>) {
        self.message = SharedString::from(msg.to_string());
        cx.notify();
    }

    fn render_buttons(&mut self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        h_flex()
            .gap_2()
            .justify_between()
            .child(
                h_flex()
                    .gap_2()
                    .child(Button::new("test-0").label("Size 0").on_click(cx.listener(
                        |view, _, cx| {
                            view.change_test_cases(0, cx);
                        },
                    )))
                    .child(Button::new("test-1").label("Size 1").on_click(cx.listener(
                        |view, _, cx| {
                            view.change_test_cases(1, cx);
                        },
                    )))
                    .child(Button::new("test-2").label("Size 2").on_click(cx.listener(
                        |view, _, cx| {
                            view.change_test_cases(2, cx);
                        },
                    )))
                    .child(Button::new("test-3").label("Size 3").on_click(cx.listener(
                        |view, _, cx| {
                            view.change_test_cases(3, cx);
                        },
                    )))
                    .child(Divider::vertical().px_2())
                    .child(
                        Button::new("test-axis-both")
                            .label("Both Scrollbar")
                            .on_click(
                                cx.listener(|view, _, cx| {
                                    view.change_axis(ScrollbarAxis::Both, cx)
                                }),
                            ),
                    )
                    .child(
                        Button::new("test-axis-vertical")
                            .label("Vertical")
                            .on_click(cx.listener(|view, _, cx| {
                                view.change_axis(ScrollbarAxis::Vertical, cx)
                            })),
                    )
                    .child(
                        Button::new("test-axis-horizontal")
                            .label("Horizontal")
                            .on_click(cx.listener(|view, _, cx| {
                                view.change_axis(ScrollbarAxis::Horizontal, cx)
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

    fn new_view(cx: &mut WindowContext) -> View<impl gpui::FocusableView> {
        Self::view(cx)
    }
}

impl gpui::FocusableView for ScrollableStory {
    fn focus_handle(&self, _: &gpui::AppContext) -> gpui::FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for ScrollableStory {
    fn render(&mut self, cx: &mut gpui::ViewContext<Self>) -> impl gpui::IntoElement {
        let view = cx.view().clone();

        v_flex()
            .size_full()
            .gap_4()
            .child(self.render_buttons(cx))
            .child(
                div().w_full().child(
                    div().relative().w_full().h(px(350.)).child(
                        v_flex()
                            .id("test-0")
                            .relative()
                            .size_full()
                            .child(
                                v_virtual_list(
                                    cx.view().clone(),
                                    "items",
                                    self.item_sizes.clone(),
                                    move |story, visible_range, content_size, cx| {
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
                                                                            ui::gray_800()
                                                                        } else {
                                                                            ui::gray_100()
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
                                        Scrollbar::both(
                                            view.entity_id(),
                                            self.scroll_state.clone(),
                                            self.scroll_handle.clone(),
                                            self.scroll_size,
                                        )
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
                    .overflow_hidden()
                    .max_h(px(400.))
                    .child(
                        v_flex()
                            .p_3()
                            .w(self.test_width)
                            .id("test-1")
                            .scrollable(cx.view().entity_id(), ScrollbarAxis::Vertical)
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
