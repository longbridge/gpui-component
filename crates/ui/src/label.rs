use std::ops::Range;

use gpui::{
    div, prelude::FluentBuilder, rems, App, HighlightStyle, IntoElement, ParentElement, RenderOnce,
    SharedString, StyleRefinement, Styled, StyledText, Window,
};

use crate::{ActiveTheme, StyledExt};

const MASKED: &'static str = "•";

#[derive(IntoElement)]
pub struct Label {
    style: StyleRefinement,
    label: SharedString,
    secondary: Option<SharedString>,
    masked: bool,
    highlights_text: Option<SharedString>,
}

impl Label {
    pub fn new(label: impl Into<SharedString>) -> Self {
        let label: SharedString = label.into();
        Self {
            style: Default::default(),
            label,
            secondary: None,
            masked: false,
            highlights_text: None,
        }
    }

    /// Set the secondary text for the label,
    /// the secondary text will be displayed after the label text with `muted` color.
    pub fn secondary(mut self, secondary: impl Into<SharedString>) -> Self {
        self.secondary = Some(secondary.into());
        self
    }

    pub fn masked(mut self, masked: bool) -> Self {
        self.masked = masked;
        self
    }

    pub fn highlights(mut self, text: impl Into<SharedString>) -> Self {
        self.highlights_text = Some(text.into());
        self
    }

    fn full_text(&self) -> SharedString {
        match &self.secondary {
            Some(secondary) => format!("{} {}", self.label, secondary).into(),
            None => self.label.clone(),
        }
    }

    fn highlight_ranges(&self, total_length: usize) -> Vec<Range<usize>> {
        let mut ranges = Vec::new();
        let full_text = self.full_text();

        if self.secondary.is_some() {
            ranges.push(0..self.label.len());
            ranges.push(self.label.len()..total_length);
        }

        if let Some(matched) = &self.highlights_text {
            if !matched.is_empty() {
                let search_lower = matched.to_lowercase();
                let full_text_lower = full_text.to_lowercase();

                let mut search_start = 0;
                while let Some(pos) = full_text_lower[search_start..].find(&search_lower) {
                    let match_start = search_start + pos;
                    let match_end = match_start + matched.len();

                    if match_end <= full_text.len() {
                        ranges.push(match_start..match_end);
                    }

                    search_start = match_start + 1;
                    while !full_text.is_char_boundary(search_start)
                        && search_start < full_text.len()
                    {
                        search_start += 1;
                    }

                    if search_start >= full_text.len() {
                        break;
                    }
                }
            }
        }

        ranges
    }

    fn measure_highlights(
        &self,
        length: usize,
        cx: &mut App,
    ) -> Option<Vec<(Range<usize>, HighlightStyle)>> {
        let ranges = self.highlight_ranges(length);
        if ranges.is_empty() {
            return None;
        }

        let mut highlights = Vec::new();
        let mut highlight_ranges_added = 0;

        if self.secondary.is_some() {
            highlights.push((ranges[0].clone(), HighlightStyle::default()));
            highlights.push((
                ranges[1].clone(),
                HighlightStyle {
                    color: Some(cx.theme().muted_foreground),
                    ..Default::default()
                },
            ));
            highlight_ranges_added = 2;
        }

        for range in ranges.iter().skip(highlight_ranges_added) {
            highlights.push((
                range.clone(),
                HighlightStyle {
                    color: Some(cx.theme().blue),
                    ..Default::default()
                },
            ));
        }

        Some(highlights)
    }
}

impl Styled for Label {
    fn style(&mut self) -> &mut gpui::StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for Label {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let mut text = self.full_text();
        let chars_count = text.chars().count();

        if self.masked {
            text = SharedString::from(MASKED.repeat(chars_count))
        };

        let highlights = self.measure_highlights(text.len(), cx);

        div()
            .line_height(rems(1.25))
            .text_color(cx.theme().foreground)
            .refine_style(&self.style)
            .child(
                StyledText::new(&text).when_some(highlights, |this, hl| this.with_highlights(hl)),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_highlights() {
        let label = Label::new("Hello World");
        let result = label.highlight_ranges("Hello World".len());
        assert_eq!(result, Vec::<Range<usize>>::new());
    }

    #[test]
    fn test_secondary_text_ranges() {
        let label = Label::new("Hello").secondary("World");
        let total_length = "Hello World".len();
        let result = label.highlight_ranges(total_length);

        assert_eq!(result.len(), 2);
        assert_eq!(result[0], 0..5); // "Hello"
        assert_eq!(result[1], 5..11); // " World"
    }

    #[test]
    fn test_highlights_text_single_match() {
        let label = Label::new("Hello World").highlights("World");
        let result = label.highlight_ranges("Hello World".len());

        assert_eq!(result.len(), 1);
        assert_eq!(result[0], 6..11); // "World"
    }

    #[test]
    fn test_highlights_text_case_insensitive() {
        let label = Label::new("Hello World").highlights("WORLD");
        let result = label.highlight_ranges("Hello World".len());

        assert_eq!(result.len(), 1);
        assert_eq!(result[0], 6..11); // "World"
    }

    #[test]
    fn test_highlights_text_multiple_matches() {
        let label = Label::new("Hello Hello Hello").highlights("Hello");
        let result = label.highlight_ranges("Hello Hello Hello".len());

        assert_eq!(result.len(), 3);
        assert_eq!(result[0], 0..5); // First "Hello"
        assert_eq!(result[1], 6..11); // Second "Hello"
        assert_eq!(result[2], 12..17); // Third "Hello"
    }

    #[test]
    fn test_highlights_text_no_match() {
        let label = Label::new("Hello World").highlights("xyz");
        let result = label.highlight_ranges("Hello World".len());

        assert_eq!(result, Vec::<Range<usize>>::new());
    }

    #[test]
    fn test_highlights_text_empty_search() {
        let label = Label::new("Hello World").highlights("");
        let result = label.highlight_ranges("Hello World".len());

        assert_eq!(result, Vec::<Range<usize>>::new());
    }

    #[test]
    fn test_both_secondary_and_highlights() {
        let label = Label::new("Hello").secondary("World").highlights("llo");
        let total_length = "Hello World".len();
        let result = label.highlight_ranges(total_length);

        assert_eq!(result.len(), 3);
        assert_eq!(result[0], 0..5); // Main text range
        assert_eq!(result[1], 5..11); // Secondary text range
        assert_eq!(result[2], 2..5); // "llo" in "Hello"
    }

    #[test]
    fn test_highlights_text_boundary() {
        let label = Label::new("Hello World Hello").highlights("Hello");
        let result = label.highlight_ranges("Hello World Hello".len());

        assert_eq!(result.len(), 2);
        assert_eq!(result[0], 0..5); // Start of "Hello"
        assert_eq!(result[1], 12..17); // End of "Hello"
    }

    #[test]
    fn test_highlights_text_overlapping_match() {
        let label = Label::new("aaaa").highlights("aa");
        let result = label.highlight_ranges("aaaa".len());

        assert!(result.len() >= 2);
        assert_eq!(result[0], 0..2); // First "aa"
        assert_eq!(result[1], 1..3); // Overlapping "aa"
        if result.len() >= 3 {
            assert_eq!(result[2], 2..4); // Third "aa"
        }
    }

    #[test]
    fn test_partial_word_highlight() {
        let label = Label::new("JavaScript is great").highlights("Script");
        let result = label.highlight_ranges("JavaScript is great".len());

        assert_eq!(result.len(), 1);
        assert_eq!(result[0], 4..10); // "Script" in "JavaScript"
    }

    #[test]
    fn test_unicode_text_highlight() {
        let label = Label::new("你好世界，Hello World").highlights("世界");
        let result = label.highlight_ranges("你好世界，Hello World".len());

        assert_eq!(result.len(), 1);
        let text = "你好世界，Hello World";
        let start = text.find("世界").unwrap();
        let end = start + "世界".len();
        assert_eq!(result[0], start..end);
    }
}
