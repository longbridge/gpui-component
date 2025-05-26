use std::{collections::HashMap, ops::Range, rc::Rc};

use gpui::{App, HighlightStyle, SharedString, TextRun, TextStyle};

use crate::{highlighter::Highlighter, ActiveTheme, ThemeMode};

#[derive(Debug, Clone)]
pub(crate) struct LineHighlightStyle {
    offset: usize,
    pub(crate) styles: Rc<Vec<(Range<usize>, HighlightStyle)>>,
}

impl LineHighlightStyle {
    pub(super) fn to_run(
        &self,
        text_style: &TextStyle,
        marked_range: &Option<Range<usize>>,
        marked_run: &TextRun,
    ) -> Vec<TextRun> {
        self.styles
            .iter()
            .map(|(range, style)| {
                let mut run = text_style.clone().highlight(*style).to_run(range.len());
                if let Some(marked_range) = marked_range {
                    if self.offset + range.start >= marked_range.start
                        && self.offset + range.end <= marked_range.end
                    {
                        run.color = marked_run.color;
                        run.strikethrough = marked_run.strikethrough;
                        run.underline = marked_run.underline;
                    }
                }
                run
            })
            // Add last `\n` Run with len 1
            .chain(std::iter::once(text_style.clone().to_run(1)))
            .collect()
    }
}

#[derive(Clone)]
pub(super) struct CodeHighlighter {
    pub(super) highlighter: Rc<Highlighter<'static>>,
    pub(super) text: SharedString,
    /// The lines by split \n
    pub(super) lines: Vec<LineHighlightStyle>,
    pub(super) cache_theme_mode: ThemeMode,
    pub(super) cache: HashMap<u64, LineHighlightStyle>,
}

impl CodeHighlighter {
    pub(super) fn new(highlighter: Rc<Highlighter<'static>>) -> Self {
        Self {
            highlighter,
            text: SharedString::default(),
            lines: vec![],
            cache_theme_mode: ThemeMode::default(),
            cache: HashMap::new(),
        }
    }

    pub fn set_highlighter(&mut self, highlighter: Rc<Highlighter<'static>>, cx: &mut App) {
        self.highlighter = highlighter;
        self.lines.clear();
        self.cache.clear();
        self.update(self.text.clone(), true, cx);
    }

    pub fn update(&mut self, text: SharedString, force: bool, cx: &mut App) {
        if self.text == text && self.cache_theme_mode == cx.theme().mode && !force {
            return;
        }

        // Clear if mode is changed
        if self.cache_theme_mode != cx.theme().mode {
            self.cache.clear();
        }

        let mut lines = vec![];
        let mut offset = 0;
        let mut new_cache = HashMap::new();
        for line in text.split('\n') {
            let cache_key = gpui::hash(&line);

            // cache hit
            if let Some(line_style) = self.cache.get(&cache_key) {
                let new_style = LineHighlightStyle {
                    offset,
                    styles: line_style.styles.clone(),
                };
                new_cache.insert(cache_key, new_style.clone());
                lines.push(new_style);
            } else {
                // cache miss
                let styles = Rc::new(self.highlighter.highlight(line, cx.theme().is_dark()));
                let line_style = LineHighlightStyle { offset, styles };
                new_cache.insert(cache_key, line_style.clone());
                lines.push(line_style);
            }

            // +1 for '\n'
            offset += line.len() + 1;
        }

        // Ensure to recreate cache to remove unused caches.
        self.cache_theme_mode = cx.theme().mode;
        self.cache = new_cache;
        self.lines = lines;
        self.text = text;
    }
}
