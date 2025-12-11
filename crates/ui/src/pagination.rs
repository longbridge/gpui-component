use std::rc::Rc;

use gpui::{
    App, ClickEvent, Div, ElementId, InteractiveElement, IntoElement, ParentElement, RenderOnce,
    SharedString, Stateful, StyleRefinement, Styled, Window, div, prelude::FluentBuilder,
};

use crate::{ActiveTheme, Disableable, StyledExt, button::Button, h_flex, icon::IconName};

/// Pagination component for navigating through pages of data.
///
/// This component displays current page information and provides previous/next
/// navigation buttons. It's commonly used with tables or lists that display
/// paginated data.
///
/// # Examples
///
/// ```
/// use gpui_component::pagination::Pagination;
///
/// Pagination::new("my-pagination")
///     .current_page(1)
///     .total_pages(10)
///     .total_items(100)
///     .loading(false)
///     .on_prev(|_, _, _| {
///         // Handle previous page
///     })
///     .on_next(|_, _, _| {
///         // Handle next page
///     })
/// ```
#[derive(IntoElement)]
pub struct Pagination {
    base: Stateful<Div>,
    style: StyleRefinement,
    current_page: u32,
    total_pages: u32,
    total_items: i64,
    loading: bool,
    prev_icon: IconName,
    next_icon: IconName,
    show_info: bool,
    info_text: Option<SharedString>,
    on_prev: Option<Rc<dyn Fn(&ClickEvent, &mut Window, &mut App)>>,
    on_next: Option<Rc<dyn Fn(&ClickEvent, &mut Window, &mut App)>>,
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
            total_items: 0,
            loading: false,
            prev_icon: IconName::ChevronLeft,
            next_icon: IconName::ChevronRight,
            show_info: true,
            info_text: None,
            on_prev: None,
            on_next: None,
        }
    }

    /// Set the current page number (1-based).
    pub fn current_page(mut self, page: u32) -> Self {
        self.current_page = page;
        self
    }

    /// Set the total number of pages.
    pub fn total_pages(mut self, pages: u32) -> Self {
        self.total_pages = pages;
        self
    }

    /// Set the total number of items across all pages.
    pub fn total_items(mut self, items: i64) -> Self {
        self.total_items = items;
        self
    }

    /// Set the loading state. When true, buttons will be disabled.
    pub fn loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    /// Set a custom icon for the previous button.
    pub fn prev_icon(mut self, icon: IconName) -> Self {
        self.prev_icon = icon;
        self
    }

    /// Set a custom icon for the next button.
    pub fn next_icon(mut self, icon: IconName) -> Self {
        self.next_icon = icon;
        self
    }

    /// Hide the page information text.
    pub fn hide_info(mut self) -> Self {
        self.show_info = false;
        self
    }

    /// Set custom information text instead of the default.
    ///
    /// If not set, displays "Page {current} of {total} • {items} items"
    pub fn info_text(mut self, text: impl Into<SharedString>) -> Self {
        self.info_text = Some(text.into());
        self
    }

    /// Set the handler for previous page button click.
    pub fn on_prev(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_prev = Some(Rc::new(handler));
        self
    }

    /// Set the handler for next page button click.
    pub fn on_next(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_next = Some(Rc::new(handler));
        self
    }
}

impl Styled for Pagination {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for Pagination {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let muted_fg = cx.theme().muted_foreground;
        let can_prev = self.current_page > 1 && !self.loading;
        let can_next = self.current_page < self.total_pages && !self.loading;

        let info_text = self.info_text.unwrap_or_else(|| {
            SharedString::from(format!(
                "Page {} of {} • {} items",
                self.current_page, self.total_pages, self.total_items
            ))
        });

        self.base
            .refine_style(&self.style)
            .flex()
            .flex_shrink_0()
            .bg(cx.theme().background)
            .px_2()
            .py_2()
            .rounded_lg()
            .justify_between()
            .items_center()
            .when(self.show_info, |this| {
                this.child(div().text_sm().text_color(muted_fg).child(info_text))
            })
            .child(
                h_flex()
                    .gap_2()
                    .items_center()
                    .child({
                        let mut btn = Button::new("pagination-prev")
                            .icon(self.prev_icon)
                            .compact()
                            .disabled(!can_prev);

                        if let Some(handler) = self.on_prev {
                            btn = btn.on_click(move |event, window, cx| handler(event, window, cx));
                        }
                        btn
                    })
                    .child({
                        let mut btn = Button::new("pagination-next")
                            .icon(self.next_icon)
                            .compact()
                            .disabled(!can_next);

                        if let Some(handler) = self.on_next {
                            btn = btn.on_click(move |event, window, cx| handler(event, window, cx));
                        }
                        btn
                    }),
            )
    }
}
