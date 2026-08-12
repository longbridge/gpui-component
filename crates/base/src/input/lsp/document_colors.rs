use super::*;

pub trait DocumentColorProvider {
    fn document_colors(
        &self,
        text: &Rope,
        window: &mut Window,
        cx: &mut App,
    ) -> Task<Result<Vec<ColorInformation>>>;
}

impl Lsp {
    pub(crate) fn update_document_colors(
        &mut self,
        text: &Rope,
        window: &mut Window,
        cx: &mut Context<InputState>,
    ) {
        let Some(provider) = self.document_color_provider.clone() else {
            return;
        };
        let text = text.clone();
        let state = cx.entity();
        let executor = cx.background_executor().clone();
        self._document_color_task = cx.spawn_in(window, async move |_, cx| {
            executor.timer(Duration::from_millis(100)).await;
            let task = cx
                .update(|window, cx| provider.document_colors(&text, window, cx))
                .ok();
            if let Some(task) = task {
                if let Ok(colors) = task.await {
                    let _ = state.update(cx, |state, cx| {
                        let mut colors: Vec<_> = colors
                            .into_iter()
                            .map(|info| {
                                let color: Hsla = gpui::Rgba {
                                    r: info.color.red,
                                    g: info.color.green,
                                    b: info.color.blue,
                                    a: info.color.alpha,
                                }
                                .into();
                                (info.range, color)
                            })
                            .collect();
                        colors.sort_by_key(|(range, _)| range.start);
                        if colors != state.lsp.document_colors {
                            state.lsp.document_colors = colors;
                            cx.notify();
                        }
                    });
                }
            }
        });
    }
    pub(crate) fn document_colors_for_range(
        &self,
        text: &Rope,
        visible: &Range<usize>,
    ) -> Vec<(Range<usize>, Hsla)> {
        self.document_colors
            .iter()
            .filter_map(|(range, color)| {
                let start = text.position_to_offset(&range.start);
                let end = text.position_to_offset(&range.end);
                (start < visible.end && end > visible.start).then_some((start..end, *color))
            })
            .collect()
    }
}
