use std::rc::Rc;

use gpui::{
    App, Div, ElementId, InteractiveElement, IntoElement, ParentElement, RenderOnce, SharedString,
    Stateful, StyleRefinement, Styled, Window, div, prelude::FluentBuilder,
};

use crate::{
    ActiveTheme, Disableable, StyledExt,
    button::{Button, ButtonVariants},
    h_flex,
    icon::IconName,
    v_flex,
};

/// Pagination component for navigating through pages of data.
///
/// This component displays current page information and provides previous/next
/// navigation buttons with page numbers. It's commonly used with tables or lists
/// that display paginated data.
///
/// # Examples
///
/// Basic usage:
///
/// ```ignore
/// use gpui_component::pagination::Pagination;
///
/// Pagination::new("my-pagination")
///     .current_page(1)
///     .total_pages(10)
///     .on_page_change(|page, _, cx| {
///         // Handle page change
///     })
/// ```
///
/// With custom info display:
///
/// ```ignore
/// Pagination::new("my-pagination")
///     .current_page(1)
///     .total_pages(10)
///     .child(div().text_sm().child("Page 1 of 10"))
///     .on_page_change(|page, _, cx| {
///         // Handle page change
///     })
/// ```
#[derive(IntoElement)]
pub struct Pagination {
    base: Stateful<Div>,
    style: StyleRefinement,
    current_page: u32,
    total_pages: u32,
    loading: bool,
    info: Option<gpui::AnyElement>,
    show_page_numbers: bool,
    max_visible_pages: usize,
    on_page_change: Option<Rc<dyn Fn(&u32, &mut Window, &mut App)>>,
}

impl Pagination {
    /// Create a new Pagination component with the given ID.
    pub fn new(id: impl Into<ElementId>) -> Self {
        let id = id.into();
        Self {
            base: div().id(id),
            style: StyleRefinement::default(),
            current_page: 1,
            total_pages: 1,
            loading: false,
            info: None,
            show_page_numbers: true,
            max_visible_pages: 7,
            on_page_change: None,
        }
    }

    /// Set the current page number (1-based).
    ///
    /// The value will be clamped between 1 and total_pages when total_pages is set.
    pub fn current_page(mut self, page: u32) -> Self {
        self.current_page = page.max(1);
        self
    }

    /// Set the total number of pages.
    pub fn total_pages(mut self, pages: u32) -> Self {
        self.total_pages = pages.max(1);
        if self.current_page > self.total_pages {
            self.current_page = self.total_pages;
        }
        self
    }

    /// Set the loading state. When true, buttons will be disabled.
    pub fn loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    /// Set custom info element to display page information.
    ///
    /// If not set, no info text will be displayed.
    pub fn child(mut self, info: impl IntoElement) -> Self {
        self.info = Some(info.into_any_element());
        self
    }

    /// Set the handler for page change (when clicking on page numbers, prev, or next).
    ///
    /// This handler receives the new page number to navigate to.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// Pagination::new("my-pagination")
    ///     .current_page(current_page)
    ///     .total_pages(total_pages)
    ///     .on_page_change(|page, _, cx| {
    ///         // Handle page change
    ///     })
    /// ```
    pub fn on_page_change(
        mut self,
        handler: impl Fn(&u32, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_page_change = Some(Rc::new(handler));
        self
    }

    /// Hide page numbers display.
    pub fn hide_page_numbers(mut self) -> Self {
        self.show_page_numbers = false;
        self
    }

    /// Set maximum number of visible page numbers (default: 7).
    pub fn max_visible_pages(mut self, max: usize) -> Self {
        self.max_visible_pages = max;
        self
    }
}

impl Styled for Pagination {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for Pagination {
    fn render(mut self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let can_prev = self.current_page > 1 && !self.loading;
        let can_next = self.current_page < self.total_pages && !self.loading;
        let base_id = self.base.interactivity().element_id.clone();

        let page_numbers = if self.show_page_numbers {
            calculate_page_range(self.current_page, self.total_pages, self.max_visible_pages)
        } else {
            vec![]
        };

        let has_info = self.info.is_some();
        let layout = if has_info { v_flex().gap_3() } else { v_flex() };

        let current_page = self.current_page;
        let loading = self.loading;
        let on_page_change = self.on_page_change;

        // Helper to create navigation button
        let nav_button = |id_suffix: &str, icon: IconName, page: u32, enabled: bool| {
            let handler = on_page_change.clone();
            Button::new(SharedString::from(format!("{:?}-{}", base_id, id_suffix)))
                .icon(icon)
                .compact()
                .disabled(!enabled)
                .when_some(handler, |this, handler| {
                    this.on_click(move |_, window, cx| handler(&page, window, cx))
                })
        };

        self.base
            .flex()
            .flex_shrink_0()
            .bg(cx.theme().background)
            .px_2()
            .py_2()
            .rounded_lg()
            .items_center()
            .refine_style(&self.style)
            .child(
                layout
                    .when_some(self.info, |this, info| this.child(info))
                    .child(
                        h_flex()
                            .gap_1()
                            .items_center()
                            .child(nav_button(
                                "prev",
                                IconName::ChevronLeft,
                                current_page.saturating_sub(1),
                                can_prev,
                            ))
                            .when(self.show_page_numbers, |this| {
                                page_numbers.iter().fold(this, |this, page_item| match page_item {
                                    PageItem::Page(page) => {
                                        let is_current = *page == current_page;
                                        let page_num = *page;
                                        let handler = on_page_change.clone();

                                        let mut button = Button::new(SharedString::from(format!(
                                            "{:?}-page-{}",
                                            base_id, page
                                        )))
                                        .label(page.to_string())
                                        .compact()
                                        .disabled(loading);

                                        if is_current {
                                            button = button.primary();
                                        }

                                        if let Some(handler) = handler {
                                            if !is_current && !loading {
                                                button = button.on_click(move |_, window, cx| {
                                                    handler(&page_num, window, cx);
                                                });
                                            }
                                        }

                                        this.child(button)
                                    }
                                    PageItem::Ellipsis(idx) => this.child(
                                        div()
                                            .id(SharedString::from(format!(
                                                "{:?}-ellipsis-{}",
                                                base_id, idx
                                            )))
                                            .flex()
                                            .items_center()
                                            .justify_center()
                                            .w_8()
                                            .h_8()
                                            .text_sm()
                                            .text_color(cx.theme().muted_foreground)
                                            .child("..."),
                                    ),
                                })
                            })
                            .child(nav_button(
                                "next",
                                IconName::ChevronRight,
                                current_page + 1,
                                can_next,
                            )),
                    ),
            )
    }
}

#[derive(Debug, Clone)]
enum PageItem {
    Page(u32),
    Ellipsis(usize),
}

fn calculate_page_range(current: u32, total: u32, max_visible: usize) -> Vec<PageItem> {
    if total <= 1 {
        return vec![];
    }

    let max_visible = max_visible.max(5);

    if total as usize <= max_visible {
        return (1..=total).map(PageItem::Page).collect();
    }

    let mut pages = vec![];
    let side_pages = (max_visible - 3) / 2;

    pages.push(PageItem::Page(1));

    let start = if current <= side_pages as u32 + 1 {
        2
    } else if current > total - side_pages as u32 - 1 {
        total - side_pages as u32 - 1
    } else {
        current - side_pages as u32
    };

    if start > 2 {
        pages.push(PageItem::Ellipsis(0));
    }

    let end = if current >= total - side_pages as u32 {
        total - 1
    } else if current <= side_pages as u32 + 1 {
        side_pages as u32 + 2
    } else {
        current + side_pages as u32
    };

    for page in start..=end {
        pages.push(PageItem::Page(page));
    }

    if end < total - 1 {
        pages.push(PageItem::Ellipsis(1));
    }

    pages.push(PageItem::Page(total));

    pages
}
