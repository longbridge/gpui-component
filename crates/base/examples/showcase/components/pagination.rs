use gpui::{
    Context, IntoElement, ParentElement as _, Styled as _, div, prelude::FluentBuilder as _, px,
    rgb,
};
use gpui_base::{Button, Pagination, PaginationItem, PaginationState};

use super::super::BaseShowcase;

impl BaseShowcase {
    pub(in super::super) fn pagination(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let entity = cx.entity().downgrade();
        let state = PaginationState::new(self.page, 8).on_change(move |page, _, cx| {
            _ = entity.update(cx, |this, cx| {
                this.page = page;
                cx.notify();
            });
        });
        let items = state.items();
        Pagination::new("example-pagination", state.clone())
            .flex()
            .items_center()
            .gap_2()
            .text_xs()
            .children(items.into_iter().map(move |item| {
                match item {
                    PaginationItem::Page(page) => {
                        let state = state.clone();
                        Button::new(("page", page))
                            .size_7()
                            .p_0()
                            .flex()
                            .items_center()
                            .justify_center()
                            .border_1()
                            .border_color(rgb(0xd4d4d4))
                            .when(page == state.current_page(), |this| {
                                this.bg(rgb(0x171717)).text_color(rgb(0xffffff))
                            })
                            .on_click(move |_, window, cx| state.request_page(page, window, cx))
                            .child(page.to_string())
                            .into_any_element()
                    }
                    PaginationItem::Ellipsis(_) => div()
                        .w(px(20.))
                        .h_7()
                        .flex()
                        .items_center()
                        .justify_center()
                        .child("…")
                        .into_any_element(),
                }
            }))
    }
}
