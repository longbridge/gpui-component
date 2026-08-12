use super::*;

pub trait DocumentRangeSemanticTokensProvider {
    fn legend(&self) -> SemanticTokensLegend;

    fn semantic_tokens(
        &self,
        text: &Rope,
        range: Range<usize>,
        window: &mut Window,
        cx: &mut App,
    ) -> Task<Result<SemanticTokens>>;
}

impl Lsp {
    pub(crate) fn update_semantic_tokens(
        &mut self,
        text: &Rope,
        window: &mut Window,
        cx: &mut Context<InputState>,
    ) {
        let Some(provider) = self.semantic_tokens_provider.clone() else {
            return;
        };
        let legend = provider.legend();
        let text = text.clone();
        let range = 0..text.len();
        let state = cx.entity();
        let executor = cx.background_executor().clone();
        self._semantic_tokens_task = cx.spawn_in(window, async move |_, cx| {
            executor.timer(Duration::from_millis(100)).await;
            let task = cx
                .update(|window, cx| provider.semantic_tokens(&text, range, window, cx))
                .ok();
            if let Some(task) = task {
                if let Ok(tokens) = task.await {
                    let decoded = decode_semantic_tokens(&tokens, &legend);
                    let _ = state.update(cx, |state, cx| {
                        if decoded != state.lsp.semantic_tokens {
                            state.lsp.semantic_tokens = decoded;
                            cx.notify();
                        }
                    });
                }
            }
        });
    }
    pub(crate) fn semantic_tokens_for_range(
        &self,
        text: &Rope,
        visible: &Range<usize>,
        theme: &dyn HighlightStyleResolver,
    ) -> Vec<(Range<usize>, HighlightStyle)> {
        self.semantic_tokens
            .iter()
            .filter_map(|(range, name)| {
                let start = text.position_to_offset(&range.start);
                let end = text.position_to_offset(&range.end);
                if start >= end || start >= visible.end || end <= visible.start {
                    return None;
                }
                Some((start..end, theme.style(name.as_ref())?))
            })
            .collect()
    }
}

pub(crate) fn decode_semantic_tokens(
    tokens: &SemanticTokens,
    legend: &SemanticTokensLegend,
) -> Vec<(lsp_types::Range, SharedString)> {
    let names: Vec<SharedString> = legend
        .token_types
        .iter()
        .map(|token| SharedString::from(token.as_str().to_owned()))
        .collect();
    let mut out = Vec::with_capacity(tokens.data.len());
    let (mut line, mut character) = (0, 0);
    for token in &tokens.data {
        if token.delta_line > 0 {
            line += token.delta_line;
            character = token.delta_start;
        } else {
            character += token.delta_start;
        }
        let Some(name) = names.get(token.token_type as usize) else {
            continue;
        };
        let start = lsp_types::Position::new(line, character);
        let end = lsp_types::Position::new(line, character + token.length);
        out.push((lsp_types::Range { start, end }, name.clone()));
    }
    out.sort_by_key(|(range, _)| range.start);
    out
}
