use std::rc::Rc;

use gpui::{
    App, ElementId, InteractiveElement as _, IntoElement, ParentElement, RenderOnce, SharedString,
    StatefulInteractiveElement, StyleRefinement, Styled, Window, div, prelude::FluentBuilder as _,
};

use crate::{ActiveTheme, Icon, IconName, StyledExt, h_flex};

/// Pagination change event data.
#[derive(Clone, Debug, Default)]
pub struct PaginationEvent {
    /// The new page number (1-indexed).
    pub page: usize,
}

/// A pagination navigation element.
///
/// Pagination component with page navigation, next and previous links.
///
/// # Example
///
/// ```rust,ignore
/// use gpui_component::pagination::Pagination;
///
/// // Basic usage with custom labels
/// Pagination::new()
///     .current(self.current_page)
///     .total(self.total_pages)
///     .previous_label("上一页")
///     .next_label("下一页")
///     .on_change(cx.listener(|this, event: &PaginationEvent, _, _| {
///         this.current_page = event.page;
///     }))
///
/// // With total_items and page_size, total pages will be calculated automatically
/// Pagination::new()
///     .current(self.current_page)
///     .total_items(100)
///     .page_size(10)  // Results in 10 total pages
///     .on_change(cx.listener(|this, event: &PaginationEvent, _, _| {
///         this.current_page = event.page;
///     }))
/// ```
///
/// # Page Size Selector
///
/// To add a page size selector, combine with a Select component in your UI:
///
/// ```rust,ignore
/// h_flex()
///     .justify_between()
///     .child(
///         Select::new(&self.page_size_select)
///             .placeholder("10 条/页")
///     )
///     .child(
///         Pagination::new()
///             .current(self.current_page)
///             .total_items(self.total_items)
///             .page_size(self.page_size)
///             .on_change(cx.listener(|this, event: &PaginationEvent, _, _| {
///                 this.current_page = event.page;
///             }))
///     )
/// ```
#[derive(IntoElement)]
pub struct Pagination {
    id: ElementId,
    style: StyleRefinement,
    /// Current page number (1-indexed)
    current: usize,
    /// Total number of pages (used when total_items is not set)
    total: usize,
    /// Total number of items (optional, used with page_size to calculate total pages)
    total_items: Option<usize>,
    /// Number of items per page (default: 10)
    page_size: usize,
    /// Number of sibling pages to show on each side of current page
    siblings: usize,
    /// Label for previous button
    previous_label: SharedString,
    /// Label for next button
    next_label: SharedString,
    /// Whether to show the previous/next labels (default: true)
    show_labels: bool,
    /// Callback when page changes
    on_change: Option<Rc<dyn Fn(&PaginationEvent, &mut Window, &mut App)>>,
}

impl Pagination {
    /// Create a new pagination component.
    pub fn new() -> Self {
        Self {
            id: ElementId::Name("pagination".into()),
            style: StyleRefinement::default(),
            current: 1,
            total: 1,
            total_items: None,
            page_size: 10,
            siblings: 1,
            previous_label: "Previous".into(),
            next_label: "Next".into(),
            show_labels: true,
            on_change: None,
        }
    }

    /// Set the element ID.
    pub fn id(mut self, id: impl Into<ElementId>) -> Self {
        self.id = id.into();
        self
    }

    /// Set the current page number (1-indexed).
    /// The page will be clamped to the valid range (1 to total pages) during rendering.
    pub fn current(mut self, page: usize) -> Self {
        self.current = page.max(1);
        self
    }

    /// Set the total number of pages.
    /// Note: If `total_items` is set, total pages will be calculated from it.
    pub fn total(mut self, total: usize) -> Self {
        self.total = total.max(1);
        self
    }

    /// Set the total number of items.
    /// When set, total pages will be calculated as `ceil(total_items / page_size)`.
    pub fn total_items(mut self, total_items: usize) -> Self {
        self.total_items = Some(total_items);
        self
    }

    /// Set the number of items per page. Default is 10.
    /// Used together with `total_items` to calculate total pages.
    pub fn page_size(mut self, page_size: usize) -> Self {
        self.page_size = page_size.max(1);
        self
    }

    /// Set the number of sibling pages to show on each side of current page.
    /// Default is 1.
    pub fn siblings(mut self, siblings: usize) -> Self {
        self.siblings = siblings;
        self
    }

    /// Set the label for the previous button.
    /// Default is "Previous".
    pub fn previous_label(mut self, label: impl Into<SharedString>) -> Self {
        self.previous_label = label.into();
        self
    }

    /// Set the label for the next button.
    /// Default is "Next".
    pub fn next_label(mut self, label: impl Into<SharedString>) -> Self {
        self.next_label = label.into();
        self
    }

    /// Set whether to show the previous/next labels.
    /// If false, only the icons will be shown.
    pub fn show_labels(mut self, show: bool) -> Self {
        self.show_labels = show;
        self
    }

    /// Set the callback when page changes.
    pub fn on_change(
        mut self,
        on_change: impl Fn(&PaginationEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_change = Some(Rc::new(on_change));
        self
    }

    /// Calculate total pages based on total_items and page_size, or use total directly
    pub fn get_total_pages(&self) -> usize {
        if let Some(total_items) = self.total_items {
            // Calculate total pages from total_items and page_size
            total_items.div_ceil(self.page_size)
        } else {
            self.total
        }
        .max(1)
    }

    /// Calculate which page numbers to display
    fn get_page_numbers(&self) -> Vec<PageItem> {
        let total = self.get_total_pages();
        // Clamp current page to valid range
        let current = self.current.max(1).min(total);
        let siblings = self.siblings;

        if total <= 1 {
            return vec![PageItem::Page(1)];
        }

        let mut items = Vec::new();

        // Always show first page
        items.push(PageItem::Page(1));

        // Calculate range around current page
        let left_sibling = current.saturating_sub(siblings).max(2);
        let right_sibling = (current + siblings).min(total - 1);

        // Add left ellipsis if needed
        if left_sibling > 2 {
            items.push(PageItem::Ellipsis);
        } else if left_sibling == 2 {
            items.push(PageItem::Page(2));
        }

        // Add pages around current
        for page in left_sibling..=right_sibling {
            if page > 1 && page < total {
                items.push(PageItem::Page(page));
            }
        }

        // Add right ellipsis if needed
        if right_sibling < total - 1 {
            items.push(PageItem::Ellipsis);
        } else if right_sibling == total - 1 && total - 1 > 1 {
            // Only add total-1 page if it's greater than 1 (to avoid re-adding page 1)
            items.push(PageItem::Page(total - 1));
        }

        // Always show last page if different from first
        if total > 1 {
            items.push(PageItem::Page(total));
        }

        // Deduplicate consecutive items
        items.dedup();

        items
    }
}

#[derive(Clone, PartialEq)]
enum PageItem {
    Page(usize),
    Ellipsis,
}

impl Default for Pagination {
    fn default() -> Self {
        Self::new()
    }
}

impl Styled for Pagination {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for Pagination {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let total = self.get_total_pages();
        // Clamp current page to valid range
        let current = self.current.max(1).min(total);
        let on_change = self.on_change.clone();
        let page_numbers = self.get_page_numbers();
        let previous_label = self.previous_label.clone();
        let next_label = self.next_label.clone();
        let show_labels = self.show_labels;

        h_flex()
            .id(self.id)
            .items_center()
            .gap_1()
            .refine_style(&self.style)
            // Previous button
            .child({
                let on_change = on_change.clone();
                let disabled = current <= 1;
                div()
                    .id("pagination-previous")
                    .flex()
                    .items_center()
                    .gap_1()
                    .h_9()
                    .when(show_labels, |this| this.px_2p5())
                    .when(!show_labels, |this| this.w_9().justify_center())
                    .text_sm()
                    .rounded_md()
                    .cursor_pointer()
                    .when(!disabled, |this| {
                        this.hover(|style| {
                            style.bg(theme.accent).text_color(theme.accent_foreground)
                        })
                    })
                    .when(disabled, |this| {
                        this.cursor_not_allowed().text_color(theme.muted_foreground)
                    })
                    .child(
                        Icon::new(IconName::ChevronLeft)
                            .size_4()
                            .text_color(if disabled {
                                theme.muted_foreground
                            } else {
                                theme.foreground
                            }),
                    )
                    .when(show_labels, |this| this.child(previous_label))
                    .when(!disabled, |this| {
                        this.when_some(on_change, |this, on_change| {
                            let prev_page = current - 1;
                            this.on_click(move |_, window, cx| {
                                let event = PaginationEvent { page: prev_page };
                                on_change(&event, window, cx);
                            })
                        })
                    })
            })
            // Page numbers
            .children(page_numbers.into_iter().enumerate().map(|(ix, item)| {
                let on_change = on_change.clone();
                match item {
                    PageItem::Page(page) => {
                        let is_active = page == current;
                        div()
                            .id(ElementId::NamedInteger("page".into(), ix as u64))
                            .flex()
                            .items_center()
                            .justify_center()
                            .size_9()
                            .text_sm()
                            .rounded_md()
                            .cursor_pointer()
                            .border_1()
                            .border_color(gpui::transparent_black())
                            .when(is_active, |this| {
                                this.border_color(theme.border).bg(theme.background)
                            })
                            .when(!is_active, |this| {
                                this.hover(|style| {
                                    style.bg(theme.accent).text_color(theme.accent_foreground)
                                })
                            })
                            .child(page.to_string())
                            .when(!is_active, |this| {
                                this.when_some(on_change, |this, on_change| {
                                    this.on_click(move |_, window, cx| {
                                        let event = PaginationEvent { page };
                                        on_change(&event, window, cx);
                                    })
                                })
                            })
                            .into_any_element()
                    }
                    PageItem::Ellipsis => div()
                        .id(ElementId::NamedInteger("ellipsis".into(), ix as u64))
                        .flex()
                        .items_center()
                        .justify_center()
                        .size_9()
                        .child(
                            Icon::new(IconName::Ellipsis)
                                .size_4()
                                .text_color(theme.muted_foreground),
                        )
                        .into_any_element(),
                }
            }))
            // Next button
            .child({
                let on_change = on_change.clone();
                let disabled = current >= total;
                div()
                    .id("pagination-next")
                    .flex()
                    .items_center()
                    .gap_1()
                    .h_9()
                    .when(show_labels, |this| this.px_2p5())
                    .when(!show_labels, |this| this.w_9().justify_center())
                    .text_sm()
                    .rounded_md()
                    .cursor_pointer()
                    .when(!disabled, |this| {
                        this.hover(|style| {
                            style.bg(theme.accent).text_color(theme.accent_foreground)
                        })
                    })
                    .when(disabled, |this| {
                        this.cursor_not_allowed().text_color(theme.muted_foreground)
                    })
                    .when(show_labels, |this| this.child(next_label))
                    .child(
                        Icon::new(IconName::ChevronRight)
                            .size_4()
                            .text_color(if disabled {
                                theme.muted_foreground
                            } else {
                                theme.foreground
                            }),
                    )
                    .when(!disabled, |this| {
                        this.when_some(on_change, |this, on_change| {
                            let next_page = current + 1;
                            this.on_click(move |_, window, cx| {
                                let event = PaginationEvent { page: next_page };
                                on_change(&event, window, cx);
                            })
                        })
                    })
            })
    }
}
