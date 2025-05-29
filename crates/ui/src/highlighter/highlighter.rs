use gpui::{App, HighlightStyle, SharedString};
use std::{collections::HashMap, ops::Range, sync::Arc};
use tree_sitter::{
    InputEdit, Node, Parser, Point, Query, QueryCursor, QueryMatch, StreamingIterator, Tree,
};
use tree_sitter_highlight::{HighlightConfiguration, Highlighter};

use super::{HighlightTheme, Language};

/// A syntax highlighter that supports incremental parsing, multiline text,
/// and caching of highlight results.
#[allow(unused)]
pub struct SyntaxHighlighter {
    language_name: &'static str,
    language: Option<Language>,
    query: Option<Query>,
    injection_queries: HashMap<&'static str, Query>,
    parser: Parser,
    old_tree: Option<Tree>,
    text: SharedString,
    highlighter: Highlighter,
    config: Option<Arc<HighlightConfiguration>>,

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

    /// Build the combined injections query for the given language.
    ///
    /// https://github.com/tree-sitter/tree-sitter/blob/v0.25.5/highlight/src/lib.rs#L336
    fn build_combined_injections_query(lang: &str) -> Option<Self> {
        let language = Language::from_str(&lang);
        let Some(language) = language else {
            return None;
        };
        let config = language.config();

        let mut parser = Parser::new();
        _ = parser.set_language(&config.language);

        // Concatenate the query strings, keeping track of the start offset of each section.
        let mut query_source = String::new();
        query_source.push_str(&config.injections);
        let locals_query_offset = query_source.len();
        query_source.push_str(&config.locals);
        let highlights_query_offset = query_source.len();
        query_source.push_str(&config.highlights);

        // Construct a single query by concatenating the three query strings, but record the
        // range of pattern indices that belong to each individual string.
        let Some(query) = Query::new(&config.language, &query_source).ok() else {
            return None;
        };

        let mut locals_pattern_index = 0;
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

        // let Some(mut combined_injections_query) =
        //     Query::new(&config.language, &config.injections).ok()
        // else {
        //     return None;
        // };

        // let mut has_combined_queries = false;
        // for pattern_index in 0..locals_pattern_index {
        //     let settings = query.property_settings(pattern_index);
        //     if settings.iter().any(|s| &*s.key == "injection.combined") {
        //         has_combined_queries = true;
        //         query.disable_pattern(pattern_index);
        //     } else {
        //         combined_injections_query.disable_pattern(pattern_index);
        //     }
        // }
        // let combined_injections_query = if has_combined_queries {
        //     Some(combined_injections_query)
        // } else {
        //     None
        // };

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

        let mut injection_queries = HashMap::new();
        for inj_language in language.injection_languages() {
            let inj_config = inj_language.config();

            match Query::new(&inj_config.language, &inj_config.highlights) {
                Ok(q) => {
                    injection_queries.insert(inj_language.name(), q);
                }
                Err(e) => {
                    println!(
                        "failed to build injection query for {:?}: {:?}",
                        inj_language, e
                    );
                }
            }
        }

        // let highlight_indices = vec![None; query.capture_names().len()];

        Some(Self {
            language_name: language.name(),
            language: Some(language),
            query: Some(query),
            injection_queries,
            parser,
            old_tree: None,
            text: SharedString::new(""),
            highlighter: Highlighter::new(),
            config: None,
            cache: vec![],
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

        // FIXME: use build_combined_injections_query to build the query.

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
        let source = self.text.as_bytes();
        let mut query_cursor = QueryCursor::new();
        let mut matches = query_cursor.matches(&query, tree.root_node(), source);

        // TODO: Merge duplicate ranges.

        let mut last_end = 0;
        while let Some(m) = matches.next() {
            // Ref:
            // https://github.com/tree-sitter/tree-sitter/blob/460118b4c82318b083b4d527c9c750426730f9c0/highlight/src/lib.rs#L556
            let (language_name, content_node, _) = self.injection_for_match(None, query, m, source);
            if let Some(language_name) = language_name {
                if let Some(content_node) = content_node {
                    if content_node.start_byte() < last_end {
                        continue;
                    }

                    self.cache
                        .extend(self.handle_injection(&language_name, content_node, source));
                    last_end = content_node.end_byte();
                }

                continue;
            }

            for cap in m.captures {
                let node = cap.node;
                if node.start_byte() < last_end {
                    continue;
                }

                let highlight_name = query.capture_names()[cap.index as usize];
                let node_range: Range<usize> = node.start_byte()..node.end_byte();
                self.cache
                    .push((node_range.clone(), highlight_name.to_string()));
                last_end = node_range.end;
            }
        }

        // for item in self.cache.iter() {
        //     println!("---------- item: {:?}", item);
        // }
    }

    /// TODO: Use incremental parsing to handle the injection.
    fn handle_injection(
        &self,
        injection_language: &str,
        node: Node,
        source: &[u8],
    ) -> Vec<(Range<usize>, String)> {
        let start_offset = node.start_byte();
        let end_offset = node.end_byte();
        let mut cache = vec![];
        let Some(query) = &self.injection_queries.get(injection_language) else {
            return cache;
        };
        let Some(content) = source.get(node.start_byte()..node.end_byte()) else {
            return cache;
        };
        if content.is_empty() {
            return cache;
        };
        let Some(lang) = super::Language::from_str(injection_language) else {
            return cache;
        };
        let lang_config = lang.config();
        let mut parser = Parser::new();
        if parser.set_language(&lang_config.language).is_err() {
            return cache;
        }
        let Some(tree) = parser.parse(content, None) else {
            return cache;
        };

        let mut query_cursor = QueryCursor::new();
        let mut matches = query_cursor.matches(query, tree.root_node(), content);

        let mut last_end = start_offset;
        while let Some(m) = matches.next() {
            for cap in m.captures {
                let cap_node = cap.node;

                let node_range: Range<usize> =
                    start_offset + cap_node.start_byte()..start_offset + cap_node.end_byte();

                if node_range.start < last_end {
                    continue;
                }
                if node_range.end > end_offset {
                    break;
                }

                let highlight_name = query.capture_names()[cap.index as usize];
                last_end = node_range.end;
                cache.push((node_range, highlight_name.to_string()));
            }
        }

        cache
    }

    /// Ref:
    /// https://github.com/tree-sitter/tree-sitter/blob/v0.25.5/highlight/src/lib.rs#L1229
    ///
    /// Returns:
    /// - `language_name`: The language name of the injection.
    /// - `content_node`: The content node of the injection.
    /// - `include_children`: Whether to include the children of the content node.
    fn injection_for_match<'a>(
        &self,
        parent_name: Option<&'a str>,
        query: &'a Query,
        query_match: &QueryMatch<'a, 'a>,
        source: &'a [u8],
    ) -> (Option<&'a str>, Option<Node<'a>>, bool) {
        let content_capture_index = self.injection_content_capture_index;
        let language_capture_index = self.injection_language_capture_index;

        let mut language_name = None;
        let mut content_node = None;

        for capture in query_match.captures {
            let index = Some(capture.index);
            if index == language_capture_index {
                language_name = capture.node.utf8_text(source).ok();
            } else if index == content_capture_index {
                content_node = Some(capture.node);
            }
        }

        let mut include_children = false;
        for prop in query.property_settings(query_match.pattern_index) {
            match prop.key.as_ref() {
                // In addition to specifying the language name via the text of a
                // captured node, it can also be hard-coded via a `#set!` predicate
                // that sets the injection.language key.
                "injection.language" => {
                    if language_name.is_none() {
                        language_name = prop
                            .value
                            .as_ref()
                            .map(std::convert::AsRef::as_ref)
                            .to_owned();
                    }
                }

                // Setting the `injection.self` key can be used to specify that the
                // language name should be the same as the language of the current
                // layer.
                "injection.self" => {
                    if language_name.is_none() {
                        language_name = Some(self.language_name);
                    }
                }

                // Setting the `injection.parent` key can be used to specify that
                // the language name should be the same as the language of the
                // parent layer
                "injection.parent" => {
                    if language_name.is_none() {
                        language_name = parent_name;
                    }
                }

                // By default, injections do not include the *children* of an
                // `injection.content` node - only the ranges that belong to the
                // node itself. This can be changed using a `#set!` predicate that
                // sets the `injection.include-children` key.
                "injection.include-children" => include_children = true,
                _ => {}
            }
        }

        (language_name, content_node, include_children)
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
        for (node_range, highlight_name) in
            self.cache.iter().skip_while(|(r, _)| r.start < range.start)
        {
            // TODO: If break, the `comment.doc` will not work.
            // Ref: https://github.com/longbridge/gpui-component/pull/904/commits/d8f886939d3b472f228c1ce72154a951e98f32c5
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
