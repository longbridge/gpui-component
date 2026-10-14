//! Themed, content-sized Markdown editor. Base retains all editing behavior.
use crate::ActiveTheme;
use gpui::prelude::FluentBuilder as _;
use gpui::{
    App, Entity, InteractiveElement, IntoElement, ParentElement, RenderOnce,
    StatefulInteractiveElement, Styled, Window, div, rems,
};
pub use gpui_base::text::block_editor::{
    BlockEditorEvent, BlockEditorState, CapturedRange, CapturedRangeError, ClipboardPolicy,
    ImagePolicy, PasteRequest, SourceSnapshot, SubmitPolicy,
};

#[derive(IntoElement)]
pub struct BlockEditor {
    state: Entity<BlockEditorState>,
    min_rows: usize,
    max_rows: usize,
}
impl BlockEditor {
    pub fn new(state: &Entity<BlockEditorState>) -> Self {
        Self {
            state: state.clone(),
            min_rows: 2,
            max_rows: 16,
        }
    }
    /// Content-sized viewport clamped to logical line-height bounds. The
    /// editor's retained handle owns scrolling and caret reveal; no outer
    /// scrolling node or application height mirror is needed.
    pub fn auto_grow(mut self, min: usize, max: usize) -> Self {
        self.min_rows = min.max(1);
        self.max_rows = max.max(self.min_rows);
        self
    }
}
impl RenderOnce for BlockEditor {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let scroll = self.state.read(cx).scroll_handle();
        div()
            .id(("block-editor-viewport", self.state.entity_id()))
            .w_full()
            .min_w_0()
            .min_h(rems(self.min_rows as f32 * 1.5))
            .max_h(rems(self.max_rows as f32 * 1.5))
            .overflow_y_scroll()
            .when_some(scroll, |this, scroll| this.track_scroll(&scroll))
            .text_color(cx.theme().foreground)
            .child(gpui_base::text::block_editor::BlockEditor::new(&self.state))
    }
}
