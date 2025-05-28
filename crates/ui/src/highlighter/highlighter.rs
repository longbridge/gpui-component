use gpui::{App, HighlightStyle, SharedString};
use std::{ops::Range, sync::Arc};
use tree_sitter::{
    InputEdit, Node, Parser, Point, Query, QueryCursor, StreamingIterator as _, Tree,
};
use tree_sitter_highlight::{HighlightConfiguration, Highlighter};

use super::{HighlightTheme, Language};

/// A syntax highlighter that supports incremental parsing, multiline text,
/// and caching of highlight results.
pub struct SyntaxHighlighter {
    language: Option<Language>,
    query: Option<Query>,
    injection_queries: Option<Vec<Query>>,
    parser: Parser,
    old_tree: Option<Tree>,
    text: SharedString,
    highlighter: Highlighter,
    config: Option<Arc<HighlightConfiguration>>,
    /// Cache of highlight, the range is offset of the token in the tree.
    ///
    /// The Vec is ordered by the range from 0 to the end of the line.
    cache: Vec<(Range<usize>, String)>,
}

impl SyntaxHighlighter {
    /// Create a new SyntaxHighlighter for HTML.
    pub fn new(lang: impl Into<SharedString>) -> Self {
        let mut parser = Parser::new();
        let lang: SharedString = lang.into();
        let language = Language::from_str(&lang);
        if let Some(language) = language {
            _ = parser.set_language(&language.config().language);
        }

        SyntaxHighlighter {
            language,
            query: language.map(|l| l.query()),
            injection_queries: language
                .map(|l| l.injection_languages().iter().map(|l| l.query()).collect()),
            parser,
            old_tree: None,
            text: SharedString::new(""),
            highlighter: Highlighter::new(),
            config: None,
            cache: vec![],
        }
    }

    pub fn set_language(&mut self, lang: impl Into<SharedString>) {
        let lang = lang.into();
        let language = Language::from_str(&lang);
        if self.language == language {
            return;
        }

        if let Some(language) = language {
            _ = self.parser.set_language(&language.config().language);
        }

        self.language = language;
        self.query = language.map(|l| l.query());
        self.injection_queries =
            language.map(|l| l.injection_languages().iter().map(|l| l.query()).collect());
        self.old_tree = None;
        self.text = SharedString::new("");
        self.highlighter = Highlighter::new();
        self.config = None;
        self.cache.clear();
    }

    pub fn is_empty(&self) -> bool {
        self.text.is_empty()
    }

    /// Highlight the given text, returning a map from byte ranges to highlight captures.
    /// Uses incremental parsing, detects changed ranges, and caches unchanged results.
    pub fn update(
        &mut self,
        selected_range: &Range<usize>,
        pending_text: &str,
        new_text: &str,
        cx: &mut App,
    ) {
        if self.text == pending_text {
            return;
        }

        let new_tree = match &self.old_tree {
            None => self.parser.parse(pending_text, None),
            Some(old) => {
                let edit = InputEdit {
                    start_byte: selected_range.start,
                    old_end_byte: selected_range.end,
                    new_end_byte: selected_range.end + new_text.len(),
                    start_position: Point::new(0, 0),
                    old_end_position: Point::new(0, 0),
                    new_end_position: Point::new(0, 0),
                };
                let mut old_cloned = old.clone();
                old_cloned.edit(&edit);
                self.parser.parse(pending_text, Some(&old_cloned))
            }
        }
        .expect("failed to parse");

        // Update state
        self.old_tree = Some(new_tree);
        self.text = SharedString::from(pending_text.to_string());
        self.build_styles(cx);
    }

    fn build_styles(&mut self, _: &mut App) {
        let Some(tree) = &self.old_tree else {
            return;
        };

        let Some(query) = &self.query else {
            return;
        };

        self.cache.clear();
        let mut query_cursor = QueryCursor::new();

        let mut matches = query_cursor.matches(&query, tree.root_node(), self.text.as_bytes());

        let mut last_end = 0;
        while let Some(m) = matches.next() {
            for cap in m.captures {
                let node = cap.node;

                let node_range: Range<usize> = (node.start_byte()..node.end_byte()).into();

                if node_range.start < last_end {
                    continue;
                }

                let highlight_name = query.capture_names()[cap.index as usize];

                if highlight_name.starts_with("injection.") {
                    println!("injection. {}", highlight_name);
                    self.cache.extend(self.handle_injection(node));
                } else {
                    self.cache
                        .push((node_range.clone(), highlight_name.to_string()));
                }
                last_end = node_range.end;
            }
        }
    }

    fn handle_injection(&self, node: Node) -> Vec<(Range<usize>, String)> {
        let mut cache = vec![];
        let Some(injection_queries) = &self.injection_queries else {
            return cache;
        };
        let Some(query) = &self.query else {
            return cache;
        };

        let mut query_cursor = QueryCursor::new();

        for inj_query in injection_queries.iter() {
            let mut matches = query_cursor.matches(inj_query, node, self.text.as_bytes());
            while let Some(m) = matches.next() {
                for cap in m.captures {
                    let content_node = cap.node;
                    let highlight_name = query.capture_names()[cap.index as usize];

                    let content_range: Range<usize> =
                        (content_node.start_byte()..content_node.end_byte()).into();
                    cache.push((content_range, highlight_name.to_string()));
                }
            }
        }

        cache
    }

    /// The argument `range` is the range of the line in the text.
    ///
    /// Returns `range` is the range in the line.
    pub fn styles(
        &self,
        range: &Range<usize>,
        theme: &HighlightTheme,
    ) -> Vec<(Range<usize>, HighlightStyle)> {
        let mut styles = vec![];
        let start_offset = range.start;
        let line_len = range.len();

        let mut last_range = 0..0;
        // NOTE: the ranges in the cache may have duplicates, so we need to merge them.
        for (node_range, highlight_name) in self.cache.iter() {
            if node_range.start < range.start {
                continue;
            }

            if node_range.end > range.end {
                break;
            }

            let range_in_line = node_range.start.saturating_sub(start_offset)
                ..node_range.end.saturating_sub(start_offset);

            // Ensure every range is connected.
            if last_range.end < range_in_line.start {
                styles.push((
                    last_range.end..range_in_line.start,
                    HighlightStyle::default(),
                ));
            }

            let style = theme.style(&highlight_name).unwrap_or_default();

            styles.push((range_in_line.clone(), style));
            last_range = range_in_line;
        }

        // If the matched styles is empty, return a default range.
        if styles.len() == 0 {
            return vec![(0..line_len, HighlightStyle::default())];
        }

        // Ensure the last range is connected to the end of the line.
        if last_range.end < line_len {
            styles.push((last_range.end..line_len, HighlightStyle::default()));
        }

        styles
    }
}
