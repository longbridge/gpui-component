use super::*;

pub trait DefinitionProvider {
    fn definitions(
        &self,
        text: &Rope,
        offset: usize,
        window: &mut Window,
        cx: &mut App,
    ) -> Task<Result<Vec<lsp_types::LocationLink>>>;
}

#[derive(Clone, Default)]
pub struct HoverDefinition {
    pub(crate) symbol_range: Range<usize>,
    pub(crate) locations: Rc<Vec<lsp_types::LocationLink>>,
    pub(crate) last_location: Option<(Range<usize>, Rc<Vec<lsp_types::LocationLink>>)>,
}

impl HoverDefinition {
    pub(crate) fn update(&mut self, range: Range<usize>, locations: Vec<lsp_types::LocationLink>) {
        self.clear();
        self.symbol_range = range;
        self.locations = Rc::new(locations);
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.locations.is_empty()
    }
    pub(crate) fn is_same(&self, offset: usize) -> bool {
        self.symbol_range.contains(&offset)
    }

    pub(crate) fn clear(&mut self) {
        if !self.locations.is_empty() {
            self.last_location = Some((self.symbol_range.clone(), self.locations.clone()));
        }
        self.symbol_range = 0..0;
        self.locations = Rc::new(Vec::new());
    }
}

impl InputState {
    pub(crate) fn handle_hover_definition(
        &mut self,
        offset: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(provider) = self.lsp.definition_provider.clone() else {
            return;
        };
        if self.hover_definition.is_same(offset) {
            return;
        }
        let task = provider.definitions(&self.text, offset, window, cx);
        let fallback_range = self.text.word_range(offset).unwrap_or(offset..offset);
        let editor = cx.entity();
        self.lsp._hover_task = cx.spawn_in(window, async move |_, cx| {
            let locations = task.await?;
            editor.update(cx, |editor, cx| {
                if locations.is_empty() {
                    editor.hover_definition.clear();
                } else {
                    let range = locations
                        .first()
                        .and_then(|location| location.origin_selection_range)
                        .map(|range| {
                            editor.text.position_to_offset(&range.start)
                                ..editor.text.position_to_offset(&range.end)
                        })
                        .unwrap_or(fallback_range);
                    editor.hover_definition.update(range, locations);
                }
                cx.notify();
            });
            Ok(())
        });
    }
    pub(crate) fn handle_click_hover_definition(
        &mut self,
        event: &MouseDownEvent,
        offset: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        if !event.modifiers.secondary() || !self.hover_definition.is_same(offset) {
            return false;
        }
        let Some(location) = self.hover_definition.locations.first().cloned() else {
            return false;
        };
        self.go_to_definition(&location, window, cx);
        true
    }

    pub(crate) fn on_action_go_to_definition(
        &mut self,
        _: &crate::input::GoToDefinition,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let offset = self.cursor();
        let Some((range, locations)) = self.hover_definition.last_location.clone() else {
            return;
        };
        if !(range.start..=range.end).contains(&offset) {
            return;
        }
        if let Some(location) = locations.first() {
            self.go_to_definition(location, window, cx);
        }
    }

    fn go_to_definition(
        &mut self,
        location: &lsp_types::LocationLink,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let external = matches!(
            location.target_uri.scheme().map(|s| s.as_str()),
            Some("http" | "https")
        );
        if let Some(handler) = self.lsp.show_document.clone() {
            let params = lsp_types::ShowDocumentParams {
                uri: location.target_uri.clone(),
                external: Some(external),
                take_focus: Some(true),
                selection: Some(location.target_selection_range),
            };
            if handler(&params, window, cx) {
                return;
            }
        }
        if external {
            cx.open_url(&location.target_uri.to_string());
        } else {
            let start = self
                .text
                .position_to_offset(&location.target_selection_range.start);
            let end = self
                .text
                .position_to_offset(&location.target_selection_range.end);
            self.move_to(start, None, cx);
            self.select_to(end, cx);
        }
    }
}
