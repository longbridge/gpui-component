use gpui::{
    App, AppContext as _, Context, Entity, Focusable, IntoElement, ParentElement, Render,
    SharedString, Styled, Window,
};
use gpui_component::{
    ActiveTheme, IndexPath, h_flex,
    pagination::{Pagination, PaginationEvent},
    select::{Select, SelectEvent, SelectItem, SelectState},
    v_flex,
};

use crate::section;

/// Page size option for the select dropdown
#[derive(Clone, Debug, PartialEq)]
struct PageSizeOption {
    size: usize,
    label: SharedString,
}

impl PageSizeOption {
    fn new(size: usize) -> Self {
        Self {
            size,
            label: format!("{} 条/页", size).into(),
        }
    }
}

impl SelectItem for PageSizeOption {
    type Value = usize;

    fn title(&self) -> SharedString {
        self.label.clone()
    }

    fn value(&self) -> &Self::Value {
        &self.size
    }
}

pub struct PaginationStory {
    focus_handle: gpui::FocusHandle,
    current_page: usize,
    page_size: usize,
    total_items: usize,
    page_size_select: Entity<SelectState<Vec<PageSizeOption>>>,
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
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let page_sizes = vec![
            PageSizeOption::new(10),
            PageSizeOption::new(20),
            PageSizeOption::new(50),
            PageSizeOption::new(100),
        ];

        let page_size_select = cx.new(|cx| {
            SelectState::new(
                page_sizes,
                Some(IndexPath::default()), // Default to first option (10)
                window,
                cx,
            )
        });

        // Subscribe to select events
        cx.subscribe_in(&page_size_select, window, Self::on_page_size_change)
            .detach();

        Self {
            focus_handle: cx.focus_handle(),
            current_page: 1,
            page_size: 10,
            total_items: 500,
            page_size_select,
        }
    }

    fn on_page_size_change(
        &mut self,
        _: &Entity<SelectState<Vec<PageSizeOption>>>,
        event: &SelectEvent<Vec<PageSizeOption>>,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match event {
            SelectEvent::Confirm(value) => {
                if let Some(size) = value {
                    self.page_size = *size;
                    // Reset to first page when changing page size
                    self.current_page = 1;
                    cx.notify();
                }
            }
        }
    }

    fn total_pages(&self) -> usize {
        (self.total_items + self.page_size - 1) / self.page_size
    }
}

impl Focusable for PaginationStory {
    fn focus_handle(&self, _: &gpui::App) -> gpui::FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for PaginationStory {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .gap_y_6()
            .child(
                section("Basic Pagination").max_w_2xl().child(
                    v_flex()
                        .gap_4()
                        .items_center()
                        .child(
                            Pagination::new()
                                .current(self.current_page)
                                .total(10)
                                .on_change(cx.listener(|this, event: &PaginationEvent, _, _| {
                                    this.current_page = event.page;
                                })),
                        )
                        .child(
                            h_flex()
                                .gap_2()
                                .text_sm()
                                .child(format!("Page {} of 10", self.current_page)),
                        ),
                ),
            )
            .child(
                section("With Custom Labels").max_w_2xl().child(
                    v_flex().gap_4().items_center().child(
                        Pagination::new()
                            .current(self.current_page)
                            .total(10)
                            .previous_label("上一页")
                            .next_label("下一页")
                            .on_change(cx.listener(|this, event: &PaginationEvent, _, _| {
                                this.current_page = event.page;
                            })),
                    ),
                ),
            )
            .child(
                section("Icon Only (No Labels)").max_w_2xl().child(
                    v_flex().gap_4().items_center().child(
                        Pagination::new()
                            .current(self.current_page)
                            .total(10)
                            .show_labels(false)
                            .on_change(cx.listener(|this, event: &PaginationEvent, _, _| {
                                this.current_page = event.page;
                            })),
                    ),
                ),
            )
            .child(
                section("With Page Size Selector").max_w_2xl().child(
                    v_flex()
                        .gap_4()
                        .child(
                            h_flex()
                                .items_center()
                                .justify_between()
                                .gap_4()
                                .child(
                                    h_flex()
                                        .items_center()
                                        .gap_2()
                                        .text_sm()
                                        .text_color(cx.theme().muted_foreground)
                                        .child(format!("共 {} 条数据", self.total_items)),
                                )
                                .child(
                                    h_flex()
                                        .items_center()
                                        .gap_3()
                                        .child(
                                            Select::new(&self.page_size_select).w(gpui::px(120.)),
                                        )
                                        .child(
                                            Pagination::new()
                                                .current(self.current_page)
                                                .total_items(self.total_items)
                                                .page_size(self.page_size)
                                                .show_labels(false)
                                                .on_change(cx.listener(
                                                    |this, event: &PaginationEvent, _, _| {
                                                        this.current_page = event.page;
                                                    },
                                                )),
                                        ),
                                ),
                        )
                        .child(h_flex().gap_2().text_sm().child(format!(
                            "当前: 第 {} 页 / 共 {} 页 (每页 {} 条)",
                            self.current_page,
                            self.total_pages(),
                            self.page_size
                        ))),
                ),
            )
    }
}
