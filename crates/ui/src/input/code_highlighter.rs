use std::{collections::HashMap, ops::Range, rc::Rc};

use gpui::{App, HighlightStyle, SharedString, TextRun, TextStyle};

use crate::highlighter::Highlighter;

#[derive(Debug, Clone)]
pub(crate) struct LineHighlightStyle {
    pub(crate) offset: usize,
    pub(crate) styles: Rc<Vec<(Range<usize>, HighlightStyle)>>,
}

impl LineHighlightStyle {
    pub(super) fn to_run(&self, text_style: &TextStyle) -> Vec<TextRun> {
        self.styles
            .iter()
            .map(|(range, style)| {
                let range = range.start + self.offset..range.end + self.offset;

                text_style
                    .clone()
                    .highlight(style.clone())
                    .to_run(range.len())
            })
            .collect()
    }
}

#[derive(Clone)]
pub(super) struct CodeHighligher {
    highlighter: Rc<Highlighter<'static>>,
    pub(super) text: SharedString,
    /// The lines by split \n
    pub(super) lines: Vec<LineHighlightStyle>,
    pub(super) cache: HashMap<u64, Rc<Vec<(Range<usize>, HighlightStyle)>>>,
}

impl CodeHighligher {
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
        self.update(self.text.clone(), cx);
    }

    pub fn update(&mut self, text: SharedString, _: &mut App) {
        if self.text == text {
            return;
        }

        let mut lines = vec![];
        let mut new_cache = HashMap::new();
        let mut offset = 0;
        for line in text.lines() {
            let line_len = line.len() + 1;
            let cache_key = gpui::hash(&line);
            println!("------ {}", offset);

            // cache hit
            if let Some(styles) = self.cache.get(&cache_key) {
                new_cache.insert(cache_key, styles.clone());
                lines.push(LineHighlightStyle {
                    offset,
                    styles: styles.clone(),
                });
            } else {
                // cache miss
                let styles = Rc::new(self.highlighter.highlight(line));
                new_cache.insert(cache_key, styles.clone());
                lines.push(LineHighlightStyle { offset, styles });
            }

            offset += line_len;
        }

        // Ensure to recreate cache to remove unused caches.
        self.cache = new_cache;
        self.lines = lines;
        self.text = text;
    }
}
