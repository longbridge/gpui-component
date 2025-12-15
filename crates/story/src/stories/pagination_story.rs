use gpui::{
    App, AppContext, Context, Entity, Focusable, IntoElement, ParentElement, Render, Styled, Window,
};
use gpui_component::{pagination::Pagination, v_flex};

pub struct PaginationStory {
    basic_page: u32,
    many_pages_page: u32,
    legacy_page: u32,
    loading_page: u32,
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
            basic_page: 1,
            many_pages_page: 1,
            legacy_page: 1,
            loading_page: 1,
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
        let entity = cx.entity();

        v_flex()
            .gap_6()
            .child(
                v_flex()
                    .gap_2()
                    .child("Basic Pagination with Page Numbers")
                    .child(
                        Pagination::new("basic-pagination")
                            .current_page(self.basic_page)
                            .total_pages(10)
                            .on_page_change({
                                let entity = entity.clone();
                                move |page, _, cx| {
                                    entity.update(cx, |this, cx| {
                                        this.basic_page = *page;
                                        cx.notify();
                                    });
                                }
                            }),
                    ),
            )
            .child(
                v_flex().gap_2().child("Pagination with Many Pages").child(
                    Pagination::new("many-pages-pagination")
                        .current_page(self.many_pages_page)
                        .total_pages(50)
                        .on_page_change({
                            let entity = entity.clone();
                            move |page, _, cx| {
                                entity.update(cx, |this, cx| {
                                    this.many_pages_page = *page;
                                    cx.notify();
                                });
                            }
                        }),
                ),
            )
            .child(
                v_flex()
                    .gap_2()
                    .child("Pagination without Page Numbers (Minimal Style)")
                    .child(
                        Pagination::new("legacy-pagination")
                            .current_page(self.legacy_page)
                            .total_pages(10)
                            .hide_page_numbers()
                            .on_page_change({
                                let entity = entity.clone();
                                move |page, _, cx| {
                                    entity.update(cx, |this, cx| {
                                        this.legacy_page = *page;
                                        cx.notify();
                                    });
                                }
                            }),
                    ),
            )
            .child(
                v_flex().gap_2().child("Loading State").child(
                    Pagination::new("loading-pagination")
                        .current_page(self.loading_page)
                        .total_pages(10)
                        .loading(true)
                        .on_page_change(|_, _, _| {}),
                ),
            )
    }
}
