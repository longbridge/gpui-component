use crate::setting::SettingPage;
use gpui::{div, ElementId, IntoElement, ParentElement as _, SharedString, Styled as _};

/// A section in the settings, containing multiple pages.
pub struct SettingSection {
    pub(super) title: SharedString,
    pub(super) pages: Vec<SettingPage>,
}

impl SettingSection {
    pub(super) fn new(title: impl Into<SharedString>) -> Self {
        Self {
            title: title.into(),
            pages: Vec::new(),
        }
    }

    /// Set the title of the section.
    pub fn title(mut self, title: impl Into<SharedString>) -> Self {
        self.title = title.into();
        self
    }

    /// Add a page to the section using a closure.
    pub fn page<F>(mut self, f: F) -> Self
    where
        F: FnOnce(SettingPage) -> SettingPage,
    {
        let page = f(SettingPage::new(ElementId::Integer(0)));
        self.pages.push(page);
        self
    }

    pub(super) fn render_page(self, page_id: Option<ElementId>) -> impl IntoElement {
        for page in self.pages {
            if Some(page.id.clone()) == page_id {
                div().size_full().child(page);
            }
        }

        div()
            .flex()
            .size_full()
            .items_center()
            .justify_around()
            .child("No page selected. Please select a page from the sidebar.")
            .into_any_element()
    }
}
