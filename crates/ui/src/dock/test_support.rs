//! Panel doubles shared by the skin's tests.
//!
//! This lives beside the production modules rather than inside one module's
//! `mod tests` because the same double is needed by two of them: a tab group
//! and a tiles canvas each hand a panel to a different frame, and the question
//! — did the panel get a height? — is the same. Mirrors
//! `gpui_base::dock::test_support`.

use std::{cell::Cell, rc::Rc};

use gpui::{
    App, AppContext as _, Context, Entity, EventEmitter, FocusHandle, Focusable, IntoElement,
    Pixels, Render, Styled as _, Window, div,
};
use gpui_base::dock::PanelEvent;

use crate::{ElementExt as _, dock::Panel};

/// A panel that records the height its container actually gave it.
///
/// The defect it exists for is invisible to every behavioral test: a panel
/// whose content region resolves to zero height still activates, still
/// persists, and still opens a window. Only a measurement sees it.
pub(crate) struct MeasuredProbe {
    focus_handle: FocusHandle,
    height: Rc<Cell<Pixels>>,
}

impl MeasuredProbe {
    pub(crate) fn new(height: Rc<Cell<Pixels>>, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self {
            focus_handle: cx.focus_handle(),
            height,
        })
    }
}

impl gpui_base::dock::Panel for MeasuredProbe {
    fn panel_name(&self) -> &'static str {
        "MeasuredProbe"
    }
}

impl Panel for MeasuredProbe {}
impl EventEmitter<PanelEvent> for MeasuredProbe {}

impl Focusable for MeasuredProbe {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for MeasuredProbe {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let height = self.height.clone();
        div()
            .size_full()
            .on_prepaint(move |bounds, _, _| height.set(bounds.size.height))
    }
}
