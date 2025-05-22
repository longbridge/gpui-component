use std::{collections::HashMap, ops::Range, rc::Rc};

use gpui::{App, HighlightStyle, SharedString, TextRun, TextStyle};

use crate::highlighter::Highlighter;

#[derive(Debug, Clone)]
pub(crate) struct LineHighlightStyle {
    pub(crate) styles: Rc<Vec<(Range<usize>, HighlightStyle)>>,
}

impl LineHighlightStyle {
    pub(super) fn to_run(&self, text_style: &TextStyle) -> Vec<TextRun> {
        self.styles
            .iter()
            .map(|(range, style)| text_style.clone().highlight(*style).to_run(range.len()))
            // Add last `\n` Run with len 1
            .chain(std::iter::once(text_style.clone().to_run(1)))
            .collect()
    }
}

#[derive(Clone)]
pub(super) struct CodeHighlighter {
    highlighter: Rc<Highlighter<'static>>,
    pub(super) text: SharedString,
    /// The lines by split \n
    pub(super) lines: Vec<LineHighlightStyle>,
    pub(super) cache: HashMap<u64, LineHighlightStyle>,
}

impl CodeHighlighter {
    pub(super) fn new(highlighter: Rc<Highlighter<'static>>) -> Self {
        Self {
            highlighter,
            text: SharedString::default(),
            lines: vec![],
            cache: HashMap::new(),
        }
    }

    pub fn set_highlighter(&mut self, highlighter: Rc<Highlighter<'static>>, cx: &mut App) {
        self.highlighter = highlighter;
        self.lines.clear();
        self.cache.clear();
        self.update(self.text.clone(), true, cx);
    }

    pub fn update(&mut self, text: SharedString, force: bool, _: &mut App) {
        if self.text == text && !force {
            return;
        }

        let mut lines = vec![];
        let mut new_cache = HashMap::new();
        for line in text.split('\n') {
            let cache_key = gpui::hash(&line);

            // cache hit
            if let Some(line_style) = self.cache.get(&cache_key) {
                new_cache.insert(cache_key, line_style.clone());
                lines.push(line_style.clone());
            } else {
                // cache miss
                let styles = Rc::new(self.highlighter.highlight(line));
                let line_style = LineHighlightStyle { styles };
                new_cache.insert(cache_key, line_style.clone());
                lines.push(line_style);
            }
        }

        // Ensure to recreate cache to remove unused caches.
        self.cache = new_cache;
        self.lines = lines;
        self.text = text;
    }
}
