//! Off-screen culling of top-level blocks (light virtualization).
//!
//! Each frame, rendered top-level blocks are wrapped in a pass-through
//! [`Measured`] element that records their window-space top and bottom.
//! The next render lays out only the blocks within one viewport of the
//! visible part of the editor (the content mask at prepaint) plus the blocks
//! holding the selection ends. Runs of the others become one spacer sized
//! from the measured heights (or an estimate). Navigation into a culled
//! block falls back to the model (`BlockEditorState::vertical`).

use std::{cell::RefCell, collections::HashMap, rc::Rc};

use gpui::{
    AnyElement, App, Bounds, Element, ElementId, GlobalElementId, InspectorElementId, IntoElement,
    LayoutId, Pixels, Window, px,
};

use super::id::BlockId;

/// Estimated height of a never-measured top-level block.
pub(crate) const ESTIMATE: Pixels = px(28.);

#[derive(Default)]
pub(crate) struct CullState {
    /// Top-to-next-top distance of each measured top-level block.
    pub(crate) heights: HashMap<BlockId, Pixels>,
    /// Visible span of the editor in editor-local y, from the last prepaint.
    pub(crate) visible: Option<(Pixels, Pixels)>,
    /// Blocks measured in the current frame: (id, index, top, bottom).
    pub(crate) measured: Vec<(BlockId, usize, Pixels, Pixels)>,
    /// Editor-local y span that the current frame actually laid out.
    pub(crate) rendered_span: Option<(Pixels, Pixels)>,
    /// Number of top-level blocks laid out in the last render (perf tests).
    pub(crate) rendered: usize,
}

impl CullState {
    pub(crate) fn height(&self, id: BlockId) -> Pixels {
        self.heights.get(&id).copied().unwrap_or(ESTIMATE)
    }

    /// After prepaint: fold this frame's measurements into the heights and
    /// record the visible span. Returns whether the visible span left the
    /// laid-out span (the caller then requests another frame).
    pub(crate) fn finish_frame(&mut self, editor: Bounds<Pixels>, mask: Bounds<Pixels>) -> bool {
        let measured = std::mem::take(&mut self.measured);
        for (ix, (id, index, top, bottom)) in measured.iter().enumerate() {
            let height = match measured.get(ix + 1) {
                Some((_, next_index, next_top, _)) if *next_index == index + 1 => *next_top - *top,
                _ => self
                    .heights
                    .get(id)
                    .copied()
                    .unwrap_or(*bottom - *top)
                    .max(*bottom - *top),
            };
            self.heights.insert(*id, height);
        }
        let top = (mask.top() - editor.top()).max(px(0.));
        let bottom = (mask.bottom() - editor.top()).min(editor.size.height);
        self.visible = Some((top, bottom.max(top)));
        match (self.rendered_span, measured.first(), measured.last()) {
            (_, Some(first), Some(last)) => {
                let span = (first.2 - editor.top(), last.3 - editor.top());
                top < span.0 - px(1.) || bottom > span.1 + px(1.)
            }
            _ => false,
        }
    }
}

/// Pass-through wrapper that reports its child's bounds.
pub(crate) struct Measured {
    pub(crate) id: BlockId,
    pub(crate) index: usize,
    pub(crate) state: Rc<RefCell<CullState>>,
    pub(crate) child: Option<AnyElement>,
}

impl IntoElement for Measured {
    type Element = Self;
    fn into_element(self) -> Self {
        self
    }
}

impl Element for Measured {
    type RequestLayoutState = AnyElement;
    type PrepaintState = ();

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, AnyElement) {
        let mut child = self
            .child
            .take()
            .unwrap_or_else(|| gpui::Empty.into_any_element());
        (child.request_layout(window, cx), child)
    }

    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        child: &mut AnyElement,
        window: &mut Window,
        cx: &mut App,
    ) {
        self.state
            .borrow_mut()
            .measured
            .push((self.id, self.index, bounds.top(), bounds.bottom()));
        child.prepaint(window, cx);
    }

    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        child: &mut AnyElement,
        _: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) {
        child.paint(window, cx);
    }
}
