use std::rc::Rc;

use gpui::{
    App, AppContext, Axis, Context, Entity, FocusHandle, Focusable, InteractiveElement,
    IntoElement, ParentElement, Pixels, Render, Size, Styled, Window, div, px, size,
};
use gpui_component::{
    ActiveTheme as _, Selectable, StyledExt as _,
    button::{Button, ButtonGroup},
    h_flex, v_flex,
};

pub struct ScrollableStory {
    focus_handle: FocusHandle,
    items: Vec<String>,
    item_sizes: Rc<Vec<Size<Pixels>>>,
    test_width: Pixels,
    size_mode: usize,
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
            items,
            item_sizes: Rc::new(item_sizes),
            test_width,
            size_mode: 0,
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
        cx.notify();
    }

    fn render_buttons(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        h_flex().gap_2().justify_between().child(
            h_flex().gap_2().child(
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
            ),
        )
    }
}

impl super::Story for ScrollableStory {
    fn title() -> &'static str {
        "Scrollable"
    }

    fn description() -> &'static str {
        "A scrollable container."
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
        let test_width = self.test_width;
        v_flex()
            .size_full()
            .gap_4()
            .child(self.render_buttons(cx))
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
                            .w(test_width)
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
