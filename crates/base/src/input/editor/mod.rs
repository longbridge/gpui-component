use gpui::{
    AbsoluteLength, App, DefiniteLength, Div, Entity, FontWeight, InteractiveElement as _,
    IntoElement, RenderOnce, SharedString, Stateful, Window,
};

use super::{EditorMode, InputBaseState, InputFont, InputModeKind};

/// State for source-code editing.
///
/// This is the shared editing engine in its code-editor kind. Languages, line
/// numbers, folding, indent guides, diagnostics, decorations, and the LSP
/// providers exist on this kind only, so an ordinary input or textarea never
/// exposes them.
pub type EditorState = InputBaseState<EditorMode>;

impl InputModeKind for EditorMode {
    const MULTI_LINE: bool = true;
    const CODE_EDITOR: bool = true;

    type Extras = super::EditorExtras;

    fn hover_definition_style(
        state: &InputBaseState<Self>,
        _cx: &App,
    ) -> Option<(std::ops::Range<usize>, gpui::HighlightStyle)> {
        state.hover_definition_style()
    }

    fn hover_definition_hitbox(
        state: &InputBaseState<Self>,
        window: &mut Window,
        _cx: &App,
    ) -> Option<gpui::Hitbox> {
        state.hover_definition_hitbox(window)
    }

    fn reset_language_features(state: &mut InputBaseState<Self>) {
        state.extras.lsp.reset();
    }

    fn reset_annotations(state: &mut InputBaseState<Self>) {
        state.extras.hover_popover = None;
        state.extras.decorations.clear();
    }

    fn adjust_annotations(
        state: &mut InputBaseState<Self>,
        range: &std::ops::Range<usize>,
        new_len: usize,
    ) {
        state.extras.decorations.adjust_for_edit(range, new_len);
    }

    fn refresh_language_features(
        state: &mut InputBaseState<Self>,
        window: &mut Window,
        cx: &mut gpui::Context<InputBaseState<Self>>,
    ) {
        let text = state.text().clone();
        state.extras.lsp.update(&text, window, cx);
    }

    fn accept_inline_completion(
        state: &mut InputBaseState<Self>,
        window: &mut Window,
        cx: &mut gpui::Context<InputBaseState<Self>>,
    ) -> bool {
        state.accept_inline_completion(window, cx)
    }

    fn has_inline_completion(state: &InputBaseState<Self>) -> bool {
        state.has_inline_completion()
    }

    fn on_click(
        state: &mut InputBaseState<Self>,
        event: &gpui::MouseDownEvent,
        offset: usize,
        window: &mut Window,
        cx: &mut gpui::Context<InputBaseState<Self>>,
    ) -> bool {
        state.handle_click_hover_definition(event, offset, window, cx)
    }

    fn clear_hover_state(
        state: &mut InputBaseState<Self>,
        cx: &mut gpui::Context<InputBaseState<Self>>,
    ) {
        state.clear_hover_state(cx);
    }

    fn on_text_typed(
        state: &mut InputBaseState<Self>,
        range: &std::ops::Range<usize>,
        text: &str,
        window: &mut Window,
        cx: &mut gpui::Context<InputBaseState<Self>>,
    ) {
        state.handle_completion_trigger(range, text, window, cx);
    }

    fn clear_inline_completion(
        state: &mut InputBaseState<Self>,
        cx: &mut gpui::Context<InputBaseState<Self>>,
    ) {
        state.clear_inline_completion(cx);
    }

    fn hide_context_menu(
        state: &mut InputBaseState<Self>,
        cx: &mut gpui::Context<InputBaseState<Self>>,
    ) {
        state.hide_context_menu(cx);
    }

    fn is_context_menu_open(state: &InputBaseState<Self>, cx: &App) -> bool {
        state.is_context_menu_open(cx)
    }

    fn handle_context_menu_action(
        state: &mut InputBaseState<Self>,
        action: Box<dyn gpui::Action>,
        window: &mut Window,
        cx: &mut gpui::Context<InputBaseState<Self>>,
    ) -> bool {
        state.handle_action_for_context_menu(action, window, cx)
    }

    fn on_hover_definition(
        state: &mut InputBaseState<Self>,
        offset: usize,
        window: &mut Window,
        cx: &mut gpui::Context<InputBaseState<Self>>,
    ) {
        state.handle_hover_definition(offset, window, cx);
    }

    fn on_mouse_move(
        state: &mut InputBaseState<Self>,
        offset: usize,
        event: &gpui::MouseMoveEvent,
        window: &mut Window,
        cx: &mut gpui::Context<InputBaseState<Self>>,
    ) {
        state.handle_mouse_move(offset, event, window, cx);
    }

    fn drive_highlighter(
        highlighter: &std::rc::Rc<std::cell::RefCell<Option<Box<dyn super::InputHighlighter>>>>,
        edit: super::InputEdit,
        text: &ropey::Rope,
        folding: bool,
        window: &mut Window,
        cx: &mut gpui::Context<InputBaseState<Self>>,
    ) {
        let mut highlighter = highlighter.borrow_mut();
        let Some(highlighter) = highlighter.as_mut() else {
            return;
        };
        highlighter.update(Some(edit), text, folding, window, cx);
    }

    fn register_actions(
        element: Stateful<Div>,
        entity: &Entity<InputBaseState<Self>>,
        window: &mut Window,
    ) -> Stateful<Div> {
        element
            .on_action(window.listener_for(entity, InputBaseState::on_action_toggle_code_actions))
            .on_action(window.listener_for(entity, InputBaseState::on_action_go_to_definition))
    }
}

impl EditorState {
    /// The LSP providers and their cached results.
    ///
    /// This exists on the editor alone: an ordinary input or textarea has no
    /// language server, and no field to reach one through.
    pub fn lsp(&self) -> &super::Lsp {
        &self.extras.lsp
    }

    /// The LSP providers, mutably. Configure the providers through this.
    pub fn lsp_mut(&mut self) -> &mut super::Lsp {
        &mut self.extras.lsp
    }
}

/// An unstyled source-code editor.
#[derive(IntoElement)]
pub struct Editor {
    state: Entity<EditorState>,
    font: InputFont,
}

impl Editor {
    pub fn new(state: &Entity<EditorState>) -> Self {
        Self {
            state: state.clone(),
            font: InputFont::default(),
        }
    }

    /// Paint the code with this font, instead of the ambient one.
    ///
    /// Source code wants a monospace family at a code-sized font, which the
    /// surrounding text style rarely carries. Left unset, the editor inherits
    /// the ambient font. The four settings below fill this in one at a time.
    pub fn font(mut self, font: InputFont) -> Self {
        self.font = font;
        self
    }

    /// The family to shape the code with. See [`Self::font`].
    pub fn font_family(mut self, font_family: impl Into<SharedString>) -> Self {
        self.font = self.font.with_family(font_family);
        self
    }

    /// The size to paint the code at. See [`Self::font`].
    pub fn font_size(mut self, font_size: impl Into<AbsoluteLength>) -> Self {
        self.font = self.font.with_size(font_size);
        self
    }

    /// The weight to paint the code at. See [`Self::font`].
    pub fn font_weight(mut self, font_weight: FontWeight) -> Self {
        self.font = self.font.with_weight(font_weight);
        self
    }

    /// The height of one row, a fraction of the font size or an absolute
    /// length. See [`Self::font`].
    pub fn line_height(mut self, line_height: impl Into<DefiniteLength>) -> Self {
        self.font = self.font.with_line_height(line_height);
        self
    }
}

impl RenderOnce for Editor {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        // The state carries the font, because the editor paints its own text
        // and has no styled element in between to inherit one from.
        if !self.font.is_inherited() {
            self.state
                .update(cx, |state, cx| state.set_font(self.font, cx));
        }

        self.state
    }
}

/// What a code editor exposes to the renderer. See [`crate::input::InputExtras`].
impl crate::input::InputExtras for super::EditorExtras {
    fn decoration_layers(&self) -> Vec<&[super::TextDecoration]> {
        self.decorations.iter().collect()
    }

    fn semantic_token_styles(
        &self,
        text: &ropey::Rope,
        range: &std::ops::Range<usize>,
        resolver: &dyn crate::input::HighlightStyleResolver,
    ) -> Vec<(std::ops::Range<usize>, gpui::HighlightStyle)> {
        self.lsp.semantic_tokens_for_range(text, range, resolver)
    }

    fn document_color_swatches(
        &self,
        text: &ropey::Rope,
        range: &std::ops::Range<usize>,
    ) -> Vec<(std::ops::Range<usize>, gpui::Hsla)> {
        self.lsp.document_colors_for_range(text, range)
    }

    fn hover_symbol_range(&self) -> Option<std::ops::Range<usize>> {
        self.hover_popover
            .as_ref()
            .map(|session| session.symbol_range.clone())
    }

    fn inline_completion_item(&self) -> Option<&lsp_types::InlineCompletionItem> {
        self.inline_completion.item.as_ref()
    }

    fn context_menu_capabilities(&self) -> (bool, bool) {
        (
            self.lsp.definition_provider.is_some(),
            !self.lsp.code_action_providers.is_empty(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{
        AppContext as _, Context, ParentElement as _, Render, Styled as _, TestAppContext,
        VisualTestContext, div, px,
    };

    struct Harness {
        state: Entity<EditorState>,
    }

    impl Render for Harness {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div().size_full().child(
                Editor::new(&self.state)
                    .font_family("Courier New")
                    .font_size(px(20.)),
            )
        }
    }

    #[gpui::test]
    fn the_element_options_reach_the_state_it_paints_from(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let mut state = None;
        let (_, cx) = cx.add_window_view(|window, cx| {
            let editor = cx.new(|cx| EditorState::new(window, cx).default_value("fn main() {}"));
            state = Some(editor.clone());
            Harness { state: editor }
        });
        let state = state.unwrap();
        VisualTestContext::update(cx, |window, cx| window.draw(cx).clear(cx));

        let (family, size) = cx.read(|cx| {
            let font = state.read(cx).font_settings().clone();
            (
                font.family().map(str::to_string),
                font.size().map(|size| size.to_pixels(px(16.))),
            )
        });
        assert_eq!(family.as_deref(), Some("Courier New"));
        assert_eq!(size, Some(px(20.)));
    }
}
