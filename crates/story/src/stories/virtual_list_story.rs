use std::{ops::Range, rc::Rc};

use gpui::{
    App, AppContext, Context, Div, Entity, FocusHandle, Focusable, InteractiveElement, IntoElement,
    ParentElement, Pixels, Render, ScrollStrategy, Size, Styled, Window, div, px, size,
};
use gpui_component::{
    ActiveTheme as _, Selectable, Sizable, VirtualListScrollHandle,
    button::{Button, ButtonGroup},
    divider::Divider,
    h_flex,
    scroll::{ScrollableElement, ScrollbarAxis},
    v_flex, v_virtual_list,
};

pub struct VirtualListStory {
    focus_handle: FocusHandle,
    scroll_handle: VirtualListScrollHandle,
    items: Vec<String>,
    item_sizes: Rc<Vec<Size<Pixels>>>,
    columns_count: usize,
    axis: ScrollbarAxis,
    size_mode: usize,
    visible_range: Range<usize>,
}

const ITEM_SIZE: Size<Pixels> = size(px(100.), px(30.));

impl VirtualListStory {
    fn new(_: &mut Window, cx: &mut Context<Self>) -> Self {
        let items = (0..5000).map(|i| format!("Item {}", i)).collect::<Vec<_>>();
        let item_sizes = items.iter().map(|_| ITEM_SIZE).collect::<Vec<_>>();

        Self {
            focus_handle: cx.focus_handle(),
            scroll_handle: VirtualListScrollHandle::new(),
            items,
            item_sizes: Rc::new(item_sizes),
            columns_count: 100,
            axis: ScrollbarAxis::Both,
            size_mode: 0,
            visible_range: (0..0),
        }
    }

    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    pub fn change_test_cases(&mut self, n: usize, cx: &mut Context<Self>) {
        self.size_mode = n;
        if n == 0 {
            self.items = (0..5000).map(|i| format!("Item {}", i)).collect::<Vec<_>>();
            self.columns_count = 30;
        } else if n == 1 {
            self.items = (0..100).map(|i| format!("Item {}", i)).collect::<Vec<_>>();
            self.columns_count = 100;
        } else if n == 2 {
            self.items = (0..500000)
                .map(|i| format!("Item {}", i))
                .collect::<Vec<_>>();
            self.columns_count = 100;
        } else {
            self.items = (0..5).map(|i| format!("Item {}", i)).collect::<Vec<_>>();
            self.columns_count = 10;
        }

        self.item_sizes = Rc::new(self.items.iter().map(|_| ITEM_SIZE).collect());
        cx.notify();
    }

    pub fn change_axis(&mut self, axis: ScrollbarAxis, cx: &mut Context<Self>) {
        self.axis = axis;
        cx.notify();
    }

    fn render_buttons(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .gap_2()
            .child(
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
                    .child(format!("visible_range: {:?}", self.visible_range)),
            )
            .child(
                h_flex()
                    .gap_2()
                    .child(
                        Button::new("scroll-to0")
                            .small()
                            .outline()
                            .label("Scroll to Top")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.scroll_handle.scroll_to_item(0, ScrollStrategy::Top);
                                cx.notify();
                            })),
                    )
                    .child(
                        Button::new("scroll-to1")
                            .small()
                            .outline()
                            .label("Scroll to 50")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.scroll_handle.scroll_to_item(50, ScrollStrategy::Top);
                                cx.notify();
                            })),
                    )
                    .child(
                        Button::new("scroll-to2")
                            .small()
                            .outline()
                            .label("Scroll to 25 (center)")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.scroll_handle
                                    .scroll_to_item(25, ScrollStrategy::Center);
                                cx.notify();
                            })),
                    )
                    .child(
                        Button::new("scroll-to-bottom")
                            .small()
                            .outline()
                            .label("Scroll to Bottom")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.scroll_handle.scroll_to_bottom();
                                cx.notify();
                            })),
                    ),
            )
    }
}

impl super::Story for VirtualListStory {
    fn title() -> &'static str {
        "VirtualList"
    }

    fn description() -> &'static str {
        "Add vertical or horizontal, or both scrollbars to a container, \
        and use `virtual_list` to render a large number of items."
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }
}

impl Focusable for VirtualListStory {
    fn focus_handle(&self, _: &gpui::App) -> gpui::FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for VirtualListStory {
    fn render(
        &mut self,
        _: &mut gpui::Window,
        cx: &mut gpui::Context<Self>,
    ) -> impl gpui::IntoElement {
        let columns_count = self.columns_count;

        fn render_item(cx: &App) -> Div {
            div()
                .flex()
                .h_full()
                .items_center()
                .justify_center()
                .text_sm()
                .w(ITEM_SIZE.width)
                .h(ITEM_SIZE.height)
                .bg(cx.theme().secondary)
        }

        v_flex()
            .size_full()
            .gap_4()
            .child(self.render_buttons(cx))
            .child(
                div().w_full().flex_1().min_h_64().child(
                    div().relative().size_full().child(
                        v_flex()
                            .id("list")
                            .relative()
                            .size_full()
                            .child(
                                v_virtual_list(
                                    cx.entity().clone(),
                                    "items",
                                    self.item_sizes.clone(),
                                    move |story, visible_range, _, cx| {
                                        story.visible_range = visible_range.clone();

                                        visible_range
                                            .map(|ix| {
                                                h_flex().gap_1().items_center().children(
                                                    (0..columns_count).map(|i| {
                                                        render_item(cx).child(if i == 0 {
                                                            format!("row: {}", ix)
                                                        } else {
                                                            format!("{}", i)
                                                        })
                                                    }),
                                                )
                                            })
                                            .collect()
                                    },
                                )
                                .track_scroll(&self.scroll_handle)
                                .p_4()
                                .border_1()
                                .border_color(cx.theme().border)
                                .gap_1(),
                            )
                            .scrollbar(&self.scroll_handle, self.axis),
                    ),
                ),
            )
    }
}
