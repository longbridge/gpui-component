use std::rc::Rc;

use gpui::{
    App, AppContext, Context, Entity, FocusHandle, Focusable, IntoElement, ParentElement, Pixels,
    Render, Size, Styled, UniformListScrollHandle, Window, div, px, size, uniform_list,
};
use gpui_component::{
    ActiveTheme as _, Selectable,
    button::{Button, ButtonGroup},
    h_flex,
    scroll::ScrollableElement,
    v_flex,
};

pub struct ScrollbarStory {
    focus_handle: FocusHandle,
    items: Vec<String>,
    item_sizes: Rc<Vec<Size<Pixels>>>,
    test_width: Pixels,
    size_mode: usize,
    scroll_handle: UniformListScrollHandle,
}

const ITEM_HEIGHT: Pixels = px(50.);

impl ScrollbarStory {
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
            scroll_handle: UniformListScrollHandle::new(),
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

impl super::Story for ScrollbarStory {
    fn title() -> &'static str {
        "Scrollbar"
    }

    fn description() -> &'static str {
        "Add scrollbar to a scrollable element."
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }
}

impl Focusable for ScrollbarStory {
    fn focus_handle(&self, _: &gpui::App) -> gpui::FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for ScrollbarStory {
    fn render(
        &mut self,
        _: &mut gpui::Window,
        cx: &mut gpui::Context<Self>,
    ) -> impl gpui::IntoElement {
        v_flex()
            .size_full()
            .gap_4()
            .child(self.render_buttons(cx))
            .child({
                div()
                    .relative()
                    .border_1()
                    .border_color(cx.theme().border)
                    .flex_1()
                    .child(
                        uniform_list("list", self.items.len(), {
                            let items = self.items.clone();
                            move |visible_range, _, cx| {
                                let mut elements = Vec::with_capacity(visible_range.len());
                                for ix in visible_range {
                                    let item = &items[ix];
                                    elements.push(
                                        div()
                                            .h(ITEM_HEIGHT)
                                            .pt_1()
                                            .items_center()
                                            .justify_center()
                                            .text_sm()
                                            .child(
                                                div()
                                                    .p_2()
                                                    .bg(cx.theme().secondary)
                                                    .child(item.to_string()),
                                            ),
                                    );
                                }
                                elements
                            }
                        })
                        .py_1()
                        .px_3()
                        .size_full()
                        .track_scroll(&self.scroll_handle),
                    )
                    .vertical_scrollbar(&self.scroll_handle)
            })
    }
}
