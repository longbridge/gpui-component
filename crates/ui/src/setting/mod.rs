mod field;
mod group;
mod page;
mod section;

pub use field::*;
pub use group::*;
pub use page::*;
pub use section::*;

use crate::{
    resizable::{h_resizable, resizable_panel},
    sidebar::{Sidebar, SidebarGroup, SidebarMenu, SidebarMenuItem},
    Collapsible,
};
use gpui::{
    div, App, ElementId, IntoElement, ParentElement as _, RenderOnce, SharedString, Window,
};

/// The settings structure containing multiple sections for app settings.
///
/// The hierarchy of settings is as follows:
///
/// ```ignore
/// Settings
///   SettingSection
///     SettingPage     <- The single active page displayed
///       SettingGroup
///         SettingItem
///           Label
///           Setting Field (e.g., Toggle, Slider, Input)
/// ```
#[derive(IntoElement)]
pub struct Settings {
    id: ElementId,
    sections: Vec<SettingSection>,
}

impl Settings {
    /// Create a new settings structure with the given ID.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            sections: Vec::new(),
        }
    }

    /// Add a section to the settings with closure.
    pub fn section<F>(mut self, section: F) -> Self
    where
        F: FnOnce(SettingSection) -> SettingSection,
    {
        let section = section(SettingSection::new(SharedString::default()));
        self.sections.push(section);
        self
    }

    fn render_active_section(
        self,
        section_ix: usize,
        page_id: Option<ElementId>,
    ) -> impl IntoElement {
        for (ix, section) in self.sections.into_iter().enumerate() {
            if ix == section_ix {
                return section.render_page(page_id).into_any_element();
            }
        }

        return div().into_any_element();
    }
}

struct SettingsState {
    selected_page_id: Option<ElementId>,
}

impl RenderOnce for Settings {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let state = window.use_keyed_state(self.id.clone(), cx, |_, _| SettingsState {
            selected_page_id: None,
        });

        let selected_page_id = state.read(cx).selected_page_id.clone();
        let section_ix = self
            .sections
            .iter()
            .position(|section| {
                section
                    .pages
                    .iter()
                    .any(|page| Some(&page.id) == selected_page_id.as_ref())
            })
            .unwrap_or(0);

        h_resizable(self.id.clone())
            .child(resizable_panel().child(Sidebar::left().children(
                self.sections.iter().enumerate().map(|(ix, section)| {
                    let collapsed = ix != section_ix;
                    SidebarGroup::new(section.title.clone())
                        .collapsed(collapsed)
                        .child(
                            SidebarMenu::new().children(section.pages.iter().map(|page| {
                                SidebarMenuItem::new(page.title.clone()).on_click({
                                    let page_id = page.id.clone();
                                    let state = state.clone();
                                    move |_, _, cx| {
                                        state.update(cx, |state, cx| {
                                            state.selected_page_id = Some(page_id.clone());
                                            cx.notify();
                                        })
                                    }
                                })
                            })),
                        )
                }),
            )))
            .child(
                resizable_panel().child(self.render_active_section(section_ix, selected_page_id)),
            )
    }
}
