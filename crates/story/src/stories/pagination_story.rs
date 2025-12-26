use gpui::{
    App, AppContext, Context, Entity, FocusHandle, Focusable, IntoElement, ParentElement, Render,
    Styled, Window,
};
use gpui_component::{
    Disableable, Selectable as _, Sizable, Size,
    button::{Button, ButtonGroup},
    pagination::Pagination,
    v_flex,
};

use crate::section;

pub struct PaginationStory {
    basic_page: usize,
    many_pages_page: usize,
    compact_page: usize,
    focus_handle: FocusHandle,
    size: Size,
}

impl super::Story for PaginationStory {
    fn title() -> &'static str {
        "Pagination"
    }

    fn description() -> &'static str {
        "Pagination with page navigation, next and previous links."
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }
}

impl PaginationStory {
    pub fn view(_window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self {
            basic_page: 5,
            many_pages_page: 1,
            compact_page: 3,
            focus_handle: cx.focus_handle(),
            size: Size::default(),
        })
    }

    fn set_size(&mut self, size: Size, _: &mut Window, cx: &mut Context<Self>) {
        self.size = size;
        cx.notify();
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
                ButtonGroup::new("toggle-size")
                    .outline()
                    .compact()
                    .child(
                        Button::new("xsmall")
                            .label("XSmall")
                            .selected(self.size == Size::XSmall),
                    )
                    .child(
                        Button::new("small")
                            .label("Small")
                            .selected(self.size == Size::Small),
                    )
                    .child(
                        Button::new("medium")
                            .label("Medium")
                            .selected(self.size == Size::Medium),
                    )
                    .child(
                        Button::new("large")
                            .label("Large")
                            .selected(self.size == Size::Large),
                    )
                    .on_click(cx.listener(|this, selecteds: &Vec<usize>, window, cx| {
                        let size = match selecteds[0] {
                            0 => Size::XSmall,
                            1 => Size::Small,
                            2 => Size::Medium,
                            3 => Size::Large,
                            _ => Size::Medium,
                        };
                        this.set_size(size, window, cx);
                    })),
            )
            .child(
                section("Basic").child(
                    Pagination::new("basic-pagination")
                        .current_page(self.basic_page)
                        .total_pages(10)
                        .with_size(self.size)
                        .on_click({
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
                section("Pagination with 10 visible pages").child(
                    Pagination::new("many-pages-pagination")
                        .current_page(self.many_pages_page)
                        .total_pages(50)
                        .visible_pages(10)
                        .with_size(self.size)
                        .on_click({
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
                section("Compact Style").child(
                    Pagination::new("compact-pagination")
                        .compact()
                        .current_page(self.compact_page)
                        .total_pages(10)
                        .with_size(self.size)
                        .on_click({
                            let entity = entity.clone();
                            move |page, _, cx| {
                                entity.update(cx, |this, cx| {
                                    this.compact_page = *page;
                                    cx.notify();
                                });
                            }
                        }),
                ),
            )
            .child(
                section("Disabled").child(
                    Pagination::new("disabled-pagination")
                        .current_page(4)
                        .total_pages(10)
                        .with_size(self.size)
                        .disabled(true)
                        .on_click(|_, _, _| {}),
                ),
            )
    }
}
