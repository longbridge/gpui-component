use gpui::{
    App, AppContext, Context, Entity, Focusable, IntoElement, ParentElement, Render, Styled, Window,
};
use gpui_component::{pagination::Pagination, v_flex};

pub struct PaginationStory {
    current_page: u32,
    focus_handle: gpui::FocusHandle,
}

impl super::Story for PaginationStory {
    fn title() -> &'static str {
        "Pagination"
    }

    fn description() -> &'static str {
        "A pagination component for navigating through pages of data."
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }
}

impl PaginationStory {
    pub fn view(_window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self {
            current_page: 1,
            focus_handle: cx.focus_handle(),
        })
    }
}

impl Focusable for PaginationStory {
    fn focus_handle(&self, _: &gpui::App) -> gpui::FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for PaginationStory {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .gap_6()
            .child(
                v_flex().gap_2().child("Basic Pagination").child(
                    Pagination::new("basic-pagination")
                        .current_page(self.current_page)
                        .total_pages(10)
                        .total_items(100)
                        .on_prev(cx.listener(|view, _, _, cx| {
                            if view.current_page > 1 {
                                view.current_page -= 1;
                                cx.notify();
                            }
                        }))
                        .on_next(cx.listener(|view, _, _, cx| {
                            if view.current_page < 10 {
                                view.current_page += 1;
                                cx.notify();
                            }
                        })),
                ),
            )
            .child(
                v_flex()
                    .gap_2()
                    .child("Pagination with Custom Info Text")
                    .child(
                        Pagination::new("custom-pagination")
                            .current_page(5)
                            .total_pages(20)
                            .total_items(200)
                            .info_text("Showing page 5 of 20")
                            .on_prev(|_, _, _| {})
                            .on_next(|_, _, _| {}),
                    ),
            )
            .child(
                v_flex().gap_2().child("Pagination without Info").child(
                    Pagination::new("no-info-pagination")
                        .current_page(3)
                        .total_pages(10)
                        .total_items(100)
                        .hide_info()
                        .on_prev(|_, _, _| {})
                        .on_next(|_, _, _| {}),
                ),
            )
            .child(
                v_flex().gap_2().child("Loading State").child(
                    Pagination::new("loading-pagination")
                        .current_page(1)
                        .total_pages(10)
                        .total_items(100)
                        .loading(true)
                        .on_prev(|_, _, _| {})
                        .on_next(|_, _, _| {}),
                ),
            )
    }
}
