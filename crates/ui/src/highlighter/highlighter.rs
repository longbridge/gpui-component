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
    parser: Parser,
    old_tree: Option<Tree>,
    text: SharedString,
    highlighter: Highlighter,
    config: Option<Arc<HighlightConfiguration>>,

    combined_injections_query: Option<Query>,
    locals_pattern_index: usize,
    highlights_pattern_index: usize,
    // highlight_indices: Vec<Option<Highlight>>,
    non_local_variable_patterns: Vec<bool>,
    injection_content_capture_index: Option<u32>,
    injection_language_capture_index: Option<u32>,
    local_scope_capture_index: Option<u32>,
    local_def_capture_index: Option<u32>,
    local_def_value_capture_index: Option<u32>,
    local_ref_capture_index: Option<u32>,

    /// Cache of highlight, the range is offset of the token in the tree.
    ///
    /// The Vec is ordered by the range from 0 to the end of the line.
    cache: Vec<(Range<usize>, String)>,
}

impl SyntaxHighlighter {
    /// Create a new SyntaxHighlighter for HTML.
    pub fn new(lang: &str) -> Self {
        Self::build_combined_injections_query(&lang).unwrap()
    }

    fn build_combined_injections_query(lang: &str) -> Option<Self> {
        let language = Language::from_str(&lang);
        let Some(language) = language else {
            return None;
        };

        let mut parser = Parser::new();
        _ = parser.set_language(&language.config().language);
        let config = language.config();

        let mut query = Query::new(&config.language, &config.highlights).unwrap();

        let locals_query_offset = config.locals.len();
        let mut locals_pattern_index = 0;
        let highlights_query_offset = config.highlights.len();
        let mut highlights_pattern_index = 0;
        for i in 0..(query.pattern_count()) {
            let pattern_offset = query.start_byte_for_pattern(i);
            if pattern_offset < highlights_query_offset {
                if pattern_offset < highlights_query_offset {
                    highlights_pattern_index += 1;
                }
                if pattern_offset < locals_query_offset {
                    locals_pattern_index += 1;
                }
            }
        }

        let Some(mut combined_injections_query) =
            Query::new(&config.language, &config.injections).ok()
        else {
            return None;
        };

        let mut has_combined_queries = false;
        for pattern_index in 0..locals_pattern_index {
            let settings = query.property_settings(pattern_index);
            if settings.iter().any(|s| &*s.key == "injection.combined") {
                has_combined_queries = true;
                query.disable_pattern(pattern_index);
            } else {
                combined_injections_query.disable_pattern(pattern_index);
            }
        }
        let combined_injections_query = if has_combined_queries {
            Some(combined_injections_query)
        } else {
            None
        };

        // Find all of the highlighting patterns that are disabled for nodes that
        // have been identified as local variables.
        let non_local_variable_patterns = (0..query.pattern_count())
            .map(|i| {
                query
                    .property_predicates(i)
                    .iter()
                    .any(|(prop, positive)| !*positive && prop.key.as_ref() == "local")
            })
            .collect();

        // Store the numeric ids for all of the special captures.
        let mut injection_content_capture_index = None;
        let mut injection_language_capture_index = None;
        let mut local_def_capture_index = None;
        let mut local_def_value_capture_index = None;
        let mut local_ref_capture_index = None;
        let mut local_scope_capture_index = None;
        for (i, name) in query.capture_names().iter().enumerate() {
            let i = Some(i as u32);
            match *name {
                "injection.content" => injection_content_capture_index = i,
                "injection.language" => injection_language_capture_index = i,
                "local.definition" => local_def_capture_index = i,
                "local.definition-value" => local_def_value_capture_index = i,
                "local.reference" => local_ref_capture_index = i,
                "local.scope" => local_scope_capture_index = i,
                _ => {}
            }
        }

        // let highlight_indices = vec![None; query.capture_names().len()];

        Some(Self {
            language: Some(language),
            query: Some(query),
            parser,
            old_tree: None,
            text: SharedString::new(""),
            highlighter: Highlighter::new(),
            config: None,
            cache: vec![],
            combined_injections_query,
            locals_pattern_index,
            highlights_pattern_index,
            non_local_variable_patterns,
            injection_content_capture_index,
            injection_language_capture_index,
            local_scope_capture_index,
            local_def_capture_index,
            local_def_value_capture_index,
            local_ref_capture_index,
        })
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
                self.cache
                    .push((node_range.clone(), highlight_name.to_string()));
                last_end = node_range.end;
            }
        }
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
