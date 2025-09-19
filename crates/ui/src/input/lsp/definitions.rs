use anyhow::Result;
use gpui::{
    px, App, Context, HighlightStyle, Hitbox, MouseDownEvent, Task, UnderlineStyle, Window,
};
use rope::Rope;
use std::{ops::Range, rc::Rc};

use crate::{
    input::{element::TextElement, InputState, RopeExt},
    ActiveTheme,
};

/// Definition provider
///
/// https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#textDocument_definition
pub trait DefinitionProvider {
    /// textDocument/definition
    ///
    /// https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#textDocument_definition
    fn definitions(
        &self,
        _text: &Rope,
        _offset: usize,
        _window: &mut Window,
        _cx: &mut App,
    ) -> Task<Result<Vec<lsp_types::LocationLink>>>;
}

#[derive(Clone, Default)]
pub(crate) struct HoverDefinition {
    /// The range of the symbol that triggered the hover.
    symbol_range: Range<usize>,
    pub(crate) locations: Rc<Vec<lsp_types::LocationLink>>,
}

impl HoverDefinition {
    pub(crate) fn new(symbol_range: Range<usize>, locations: Vec<lsp_types::LocationLink>) -> Self {
        Self {
            symbol_range,
            locations: Rc::new(locations),
        }
    }

    pub(crate) fn is_same(&self, offset: usize) -> bool {
        self.symbol_range.contains(&offset)
    }
}

impl InputState {
    pub(super) fn handle_hover_definition(
        &mut self,
        offset: usize,
        window: &mut Window,
        cx: &mut Context<InputState>,
    ) {
        let Some(provider) = self.lsp.definition_provider.clone() else {
            return;
        };
        if let Some(hover_definition) = self.hover_definition.as_ref() {
            if hover_definition.is_same(offset) {
                return;
            }
        }

        // Currently not implemented.
        let task = provider.definitions(&self.text, offset, window, cx);
        let mut symbol_range = self.text.word_range(offset).unwrap_or(offset..offset);
        let editor = cx.entity();
        self.lsp._hover_task = cx.spawn_in(window, async move |_, cx| {
            let locations = task.await?;

            _ = editor.update(cx, |editor, cx| {
                if locations.is_empty() {
                    editor.hover_definition = None;
                } else {
                    if let Some(location) = locations.first() {
                        if let Some(range) = location.origin_selection_range {
                            let start = editor.text.position_to_offset(&range.start);
                            let end = editor.text.position_to_offset(&range.end);
                            symbol_range = start..end;
                        }
                    }

                    editor.hover_definition = Some(HoverDefinition::new(symbol_range, locations));
                }
                cx.notify();
            });

            Ok(())
        });
    }

    /// Return true if handled.
    pub(crate) fn handle_click_hover_definition(
        &mut self,
        event: &MouseDownEvent,
        offset: usize,
        _: &mut Window,
        cx: &mut Context<InputState>,
    ) -> bool {
        if !event.modifiers.secondary() {
            return false;
        }

        let Some(hover_definition) = self.hover_definition.as_ref() else {
            return false;
        };
        if !hover_definition.is_same(offset) {
            return false;
        }

        let Some(location) = hover_definition.locations.first().cloned() else {
            return false;
        };

        if location
            .target_uri
            .scheme()
            .map(|s| s.as_str() == "https" || s.as_str() == "http")
            == Some(true)
        {
            cx.open_url(&location.target_uri.to_string());
        } else {
            // Move to the location.
            let target_range = location.target_range;
            let start = self.text.position_to_offset(&target_range.start);
            let end = self.text.position_to_offset(&target_range.end);

            self.move_to(start, cx);
            self.select_to(end, cx);
        }

        true
    }
}

impl TextElement {
    pub(crate) fn layout_hover_definition(
        &self,
        cx: &App,
    ) -> Option<(Range<usize>, HighlightStyle)> {
        let editor = self.state.read(cx);
        if !editor.mode.is_code_editor() {
            return None;
        }

        let Some(hover_definition) = editor.hover_definition.as_ref() else {
            return None;
        };

        let mut highlight_style: HighlightStyle = cx
            .theme()
            .highlight_theme
            .link_text
            .map(|style| style.into())
            .unwrap_or_default();

        highlight_style.underline = Some(UnderlineStyle {
            thickness: px(1.),
            ..UnderlineStyle::default()
        });

        Some((hover_definition.symbol_range.clone(), highlight_style))
    }

    pub(crate) fn layout_hover_definition_hitbox(
        &self,
        editor: &InputState,
        window: &mut Window,
        _cx: &App,
    ) -> Option<Hitbox> {
        if !editor.mode.is_code_editor() {
            return None;
        }

        let Some(hover_definition) = editor.hover_definition.as_ref() else {
            return None;
        };

        let Some(bounds) = editor.range_to_bounds(&hover_definition.symbol_range) else {
            return None;
        };

        Some(window.insert_hitbox(bounds, gpui::HitboxBehavior::Normal))
    }
}
