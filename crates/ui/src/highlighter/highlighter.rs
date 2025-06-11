use super::HighlightTheme;
use crate::highlighter::LanguageRegistry;
use gpui::{App, HighlightStyle, SharedString};
use indexset::BTreeMap;
use std::{
    collections::HashMap,
    ops::{Bound, Range},
};
use tree_sitter::{
    InputEdit, Node, Parser, Point, Query, QueryCursor, QueryMatch, StreamingIterator, Tree,
};

/// A syntax highlighter that supports incremental parsing, multiline text,
/// and caching of highlight results.
#[allow(unused)]
pub struct SyntaxHighlighter {
    language: SharedString,
    query: Option<Query>,
    injection_queries: HashMap<SharedString, Query>,
    parser: Parser,
    old_tree: Option<Tree>,
    text: SharedString,

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
    /// The BTreeMap is ordered by the range in the entire text.
    ///
    /// - The `key` is the `start` of the range.
    /// -The `value` is a tuple of the range (in the entire text) and the highlight name.
    cache: BTreeMap<usize, (Range<usize>, SharedString)>,
}

impl SyntaxHighlighter {
    /// Create a new SyntaxHighlighter for HTML.
    pub fn new(lang: &str, cx: &App) -> Self {
        Self::build_combined_injections_query(&lang, cx).unwrap_or_else(|| panic!(
            "failed to build language {}, please make sure have registered the language in LanguageRegistry",
            lang
        ))
    }

    /// Build the combined injections query for the given language.
    ///
    /// https://github.com/tree-sitter/tree-sitter/blob/v0.25.5/highlight/src/lib.rs#L336
    fn build_combined_injections_query(lang: &str, cx: &App) -> Option<Self> {
        let registry = LanguageRegistry::global(cx);
        let config = registry.language(&lang)?;

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
        let query = match Query::new(&config.language, &query_source) {
            Ok(query) => Some(query),
            Err(err) => {
                panic!("failed create Query for language {}, err: {}", lang, err);
            }
        }?;

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
        for inj_language in config.injection_languages.iter() {
            if let Some(inj_config) = registry.language(&inj_language) {
                match Query::new(&inj_config.language, &inj_config.highlights) {
                    Ok(q) => {
                        injection_queries.insert(inj_config.name.clone(), q);
                    }
                    Err(e) => {
                        tracing::error!(
                            "failed to build injection query for {:?}: {:?}",
                            inj_config.name,
                            e
                        );
                    }
                }
            }
        }

        // let highlight_indices = vec![None; query.capture_names().len()];

        Some(Self {
            language: config.name.clone(),
            query: Some(query),
            injection_queries,
            parser,
            old_tree: None,
            text: SharedString::new(""),
            cache: BTreeMap::new(),
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

    pub fn is_empty(&self) -> bool {
        self.text.is_empty()
    }

    /// Highlight the given text, returning a map from byte ranges to highlight captures.
    /// Uses incremental parsing, detects changed ranges, and caches unchanged results.
    pub fn update(
        &mut self,
        selected_range: &Range<usize>,
        full_text: SharedString,
        new_text: &str,
        cx: &mut App,
    ) {
        if self.text == full_text {
            return;
        }

        // If insert a chart, this is 1.
        // If backspace or delete, this is -1.
        // If selected to delete, this is the length of the selected text.
        let changed_len = new_text.len() as isize - selected_range.len() as isize;

        let new_tree = match &self.old_tree {
            None => self.parser.parse(full_text.as_ref(), None),
            Some(old) => {
                let edit = InputEdit {
                    start_byte: selected_range.start,
                    old_end_byte: selected_range.end,
                    new_end_byte: (selected_range.end as isize + changed_len) as usize,
                    start_position: Point::new(0, 0),
                    old_end_position: Point::new(0, 0),
                    new_end_position: Point::new(0, 0),
                };
                let mut old_cloned = old.clone();
                old_cloned.edit(&edit);
                // NOTE: 10K lines, about 4.5ms
                self.parser.parse(full_text.as_ref(), Some(&old_cloned))
            }
        };

        let Some(new_tree) = new_tree else {
            return;
        };

        let mut changed_ranges = None;
        if let Some(old_tree) = &self.old_tree {
            changed_ranges = Some(new_tree.changed_ranges(old_tree));
        }

        // Update state
        self.old_tree = Some(new_tree);
        self.text = full_text;

        // let measure = Measure::new("build_styles");
        self.build_styles(changed_ranges, changed_len, cx);
        // measure.end();
    }

    /// NOTE: 10K lines, about 180ms
    fn build_styles(
        &mut self,
        changed_ranges: Option<impl ExactSizeIterator<Item = tree_sitter::Range>>,
        changed_len: isize,
        cx: &mut App,
    ) {
        let Some(tree) = &self.old_tree else {
            return;
        };

        let Some(query) = &self.query else {
            return;
        };

        let source = self.text.as_bytes();
        let mut query_cursor = QueryCursor::new();
        let mut root_node = tree.root_node();

        // Incremental parsing to only update changed ranges.
        if let Some(changed_ranges) = changed_ranges {
            let mut total_range = 0..0;
            for change_range in changed_ranges {
                if total_range.start == 0 {
                    total_range.start = change_range.start_byte;
                }
                if total_range.end == 0 {
                    total_range.end = change_range.end_byte;
                }
            }

            if total_range.len() == 0 {
                return;
            }

            if let Some(node) =
                root_node.descendant_for_byte_range(total_range.start, total_range.end)
            {
                root_node = node;
            }

            let byte_range = root_node.byte_range();

            // let measure = Measure::new("update cache to change range offset");

            // FIXME: If we delete 1 char in a node, that node will not highlighted.

            // Remove the cache entries that are range is intersecting with the byte_range.
            self.cache.retain(|_, (range, _)| {
                if range.start < byte_range.end && range.end > byte_range.start {
                    // Remove the item if it is intersecting with the byte_range.
                    false
                } else {
                    // Keep the item if it is not intersecting with the byte_range.
                    true
                }
            });

            // Apply changed_len to reorder the cache to move the range offset
            let mut old_cache: BTreeMap<usize, (Range<usize>, SharedString)> = BTreeMap::new();
            std::mem::swap(&mut self.cache, &mut old_cache);

            // NOTE: 10K lines, about 35ms
            for (start, (old_range, highlight_name)) in old_cache.into_iter() {
                if old_range.end >= byte_range.start {
                    let new_range = Range {
                        start: (old_range.start as isize + changed_len) as usize,
                        end: (old_range.end as isize + changed_len) as usize,
                    };

                    self.cache
                        .insert(new_range.start, (new_range, highlight_name));
                } else {
                    self.cache.insert(start, (old_range, highlight_name));
                }
            }
            // measure.end();
        } else {
            self.cache.clear();
        }

        let mut matches = query_cursor.matches(&query, root_node, source);

        while let Some(m) = matches.next() {
            // Ref:
            // https://github.com/tree-sitter/tree-sitter/blob/460118b4c82318b083b4d527c9c750426730f9c0/highlight/src/lib.rs#L556
            let (language_name, content_node, _) = self.injection_for_match(None, query, m, source);
            if let Some(language_name) = language_name {
                if let Some(content_node) = content_node {
                    let styles = self.handle_injection(&language_name, content_node, source, cx);
                    for (node_range, highlight_name) in styles {
                        self.cache.insert(
                            node_range.start,
                            (node_range, highlight_name.to_string().into()),
                        );
                    }
                }

                continue;
            }

            for cap in m.captures {
                let node = cap.node;

                let Some(highlight_name) = query.capture_names().get(cap.index as usize) else {
                    continue;
                };

                let node_range: Range<usize> = node.start_byte()..node.end_byte();
                let highlight_name = SharedString::from(highlight_name.to_string());

                // Merge near range and same highlight name
                let last_item = self.cache.last_key_value().map(|kv| kv.1);
                let last_range = last_item.map(|(range, _)| range).unwrap_or(&(0..0));
                let last_highlight_name = last_item.map(|(_, name)| name.clone());

                if last_range.end <= node_range.start
                    && last_highlight_name.as_ref() == Some(&highlight_name)
                {
                    self.cache.insert(
                        last_range.start,
                        (last_range.start..node_range.end, highlight_name.clone()),
                    );
                } else {
                    self.cache
                        .insert(node_range.start, (node_range, highlight_name.clone()));
                }
            }
        }

        // DO NOT REMOVE THIS PRINT, it's useful for debugging
        // for item in self.cache.iter() {
        //     println!("item: {:?}", item);
        // }
    }

    /// TODO: Use incremental parsing to handle the injection.
    fn handle_injection(
        &self,
        injection_language: &str,
        node: Node,
        source: &[u8],
        cx: &App,
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
        let Some(config) = LanguageRegistry::global(cx).language(injection_language) else {
            return cache;
        };
        let mut parser = Parser::new();
        if parser.set_language(&config.language).is_err() {
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

                if let Some(highlight_name) = query.capture_names().get(cap.index as usize) {
                    last_end = node_range.end;
                    cache.push((node_range, highlight_name.to_string()));
                }
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
        parent_name: Option<SharedString>,
        query: &'a Query,
        query_match: &QueryMatch<'a, 'a>,
        source: &'a [u8],
    ) -> (Option<SharedString>, Option<Node<'a>>, bool) {
        let content_capture_index = self.injection_content_capture_index;
        let language_capture_index = self.injection_language_capture_index;

        let mut language_name: Option<SharedString> = None;
        let mut content_node = None;

        for capture in query_match.captures {
            let index = Some(capture.index);
            if index == language_capture_index {
                language_name = capture
                    .node
                    .utf8_text(source)
                    .ok()
                    .map(ToString::to_string)
                    .map(SharedString::from);
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
                            .map(ToString::to_string)
                            .map(SharedString::from);
                    }
                }

                // Setting the `injection.self` key can be used to specify that the
                // language name should be the same as the language of the current
                // layer.
                "injection.self" => {
                    if language_name.is_none() {
                        language_name = Some(self.language.clone());
                    }
                }

                // Setting the `injection.parent` key can be used to specify that
                // the language name should be the same as the language of the
                // parent layer
                "injection.parent" => {
                    if language_name.is_none() {
                        language_name = parent_name.clone();
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
    pub(crate) fn styles(
        &self,
        range: &Range<usize>,
        theme: &HighlightTheme,
    ) -> Vec<(Range<usize>, HighlightStyle)> {
        let mut styles = vec![];
        let start_offset = range.start;
        let mut last_range = start_offset..start_offset;

        // NOTE: Iterate over the cache and print the range and style for each item.
        // for (_, (range, style)) in self.cache.iter() {
        //     println!("-- range: {:?}, style: {:?}", range, style);
        // }

        let mut cursor = self.cache.lower_bound(Bound::Included(&range.start));
        // Move to the previous item if the current item is not the start of the range.
        // This is for case like JsDoc, where token may contains multiple lines.
        if cursor.key() != Some(&range.start) {
            cursor.move_prev();
        }

        while let Some((node_range, name)) = cursor.value() {
            // Break loop if the node_range is out of the range
            if node_range.start > range.end {
                break;
            }

            let mut node_range = node_range.start.max(range.start)..node_range.end.min(range.end);
            // Avoid start larger than end
            if node_range.start > node_range.end {
                node_range.end = node_range.start;
            }

            // Ensure every range is connected.
            if last_range.end < node_range.start {
                styles.push((last_range.end..node_range.start, HighlightStyle::default()));
            }

            last_range = node_range.clone();
            styles.push((
                node_range.clone(),
                theme.style(name.as_ref()).unwrap_or_default(),
            ));

            cursor.move_next();
        }

        // If the matched styles is empty, return a default range.
        if styles.len() == 0 {
            return vec![(start_offset..range.end, HighlightStyle::default())];
        }

        // Ensure the last range is connected to the end of the line.
        if last_range.end < range.end {
            styles.push((last_range.end..range.end, HighlightStyle::default()));
        }

        let styles = unique_styles(styles);

        // NOTE: DO NOT remove this comment, it is used for debugging.
        // for style in &result {
        //     println!("---- style: {:?} - {:?}", style.0, style.1.color);
        // }
        // println!("--------------------------------");

        styles
    }
}

/// To merge intersection ranges
///
/// ```
/// vec![
///     (0..10, clean),
///     (0..10, clean),
///     (5..11, red),
///     (10..15, green),
///     (15..30, clean),
///     (29..35, blue),
///     (35..40, green),
/// ];
/// ```
///
/// to
///
/// ```
/// vec![
///   (0..5, clean),
///   (5..10, red),
///   (10..11, green),
///   (11..15, green),
///   (15..29, clean),
///   (29..30, blue),
///   (30..35, blue),
///   (35..40, green),
/// ];
/// ```
pub(crate) fn unique_styles(
    styles: Vec<(Range<usize>, HighlightStyle)>,
) -> Vec<(Range<usize>, HighlightStyle)> {
    let mut result: Vec<(Range<usize>, HighlightStyle)> = vec![];
    let mut current_range: Option<(Range<usize>, HighlightStyle)> = None;

    for (range, style) in styles.into_iter() {
        if range.is_empty() {
            continue;
        }

        if let Some((last_range, last_style)) = current_range.as_mut() {
            if last_style.color == style.color && range.start <= last_range.end {
                // Merge overlapping or adjacent ranges with the same style
                last_range.end = last_range.end.max(range.end);
            } else if range.start < last_range.end {
                // Split overlapping ranges with different styles
                let overlap_start = range.start;
                let overlap_end = last_range.end.min(range.end);

                if overlap_start > last_range.start {
                    result.push((last_range.start..overlap_start, *last_style));
                }

                result.push((overlap_start..overlap_end, style));

                last_range.end = overlap_start;
                if overlap_end < range.end {
                    current_range = Some((overlap_end..range.end, style));
                } else {
                    current_range = None;
                }
            } else {
                // Push the completed range and start a new one
                result.push((last_range.clone(), *last_style));
                current_range = Some((range, style));
            }
        } else {
            current_range = Some((range, style));
        }
    }

    if let Some((last_range, last_style)) = current_range {
        result.push((last_range, last_style));
    }

    result
}

#[cfg(test)]
mod tests {
    use gpui::Hsla;

    use super::*;
    use crate::Colorize as _;

    fn color_style(color: Hsla) -> HighlightStyle {
        let mut style = HighlightStyle::default();
        style.color = Some(color);
        style
    }

    #[track_caller]
    fn assert_unique_styles(
        left: Vec<(Range<usize>, HighlightStyle)>,
        right: Vec<(Range<usize>, HighlightStyle)>,
    ) {
        fn color_name(c: Option<Hsla>) -> String {
            match c {
                Some(c) => {
                    if c == gpui::red() {
                        "red".to_string()
                    } else if c == gpui::green() {
                        "green".to_string()
                    } else if c == gpui::blue() {
                        "blue".to_string()
                    } else {
                        c.to_hex()
                    }
                }
                None => "clean".to_string(),
            }
        }

        let left = unique_styles(left);
        if left.len() != right.len() {
            println!("\n---------------------------------------------");
            for (range, style) in left.iter() {
                println!("({:?}, {})", range, color_name(style.color));
            }
            println!("---------------------------------------------");
            panic!("left {} styles, right {} styles", left.len(), right.len());
        }
        for (left, right) in left.into_iter().zip(right) {
            if left.1.color != right.1.color || left.0 != right.0 {
                panic!(
                    "\n left: ({:?}, {})\nright: ({:?}, {})\n",
                    left.0,
                    color_name(left.1.color),
                    right.0,
                    color_name(right.1.color)
                );
            }
        }
    }

    #[test]
    fn test_unique_styles() {
        let red = color_style(gpui::red());
        let green = color_style(gpui::green());
        let blue = color_style(gpui::blue());
        let clean = HighlightStyle::default();

        assert_unique_styles(
            vec![
                (0..10, clean),
                (0..10, clean),
                (5..11, red),
                (10..15, green),
                (15..30, clean),
                (29..35, blue),
                (35..40, green),
            ],
            vec![
                (0..5, clean),
                (5..10, red),
                (10..11, green),
                (11..15, green),
                (15..29, clean),
                (29..30, blue),
                (30..35, blue),
                (35..40, green),
            ],
        );
    }
}
