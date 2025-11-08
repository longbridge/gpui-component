use std::{char, ops::Range};

use gpui::{Context, Window};
use ropey::Rope;

use crate::{input::InputState, RopeExt as _};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CharType {
    Alnum,       // A-Z, a-z, 0-9
    Connector,   // _
    Punctuation, // . , ; : ( ) [ ] { } ...
    Space,       // ' '
    Whitespace,  // '\t', '\n', '\r', '\u{00A0}' etc.
    Other,       // others
}

impl CharType {
    /// Determine the CharType from a given character.
    fn from_char(c: char) -> Self {
        match c {
            c if c.is_ascii_alphanumeric() => CharType::Alnum,
            '_' => CharType::Connector,
            c if c.is_ascii_punctuation() => CharType::Punctuation,
            ' ' => CharType::Space,
            c if c.is_whitespace() => CharType::Whitespace,
            _ => CharType::Other,
        }
    }

    /// Check if two CharTypes are connectable
    fn is_connectable(self, other: CharType) -> bool {
        match (self, other) {
            (CharType::Alnum, CharType::Alnum) => true,
            (CharType::Connector, CharType::Connector) => true,
            (CharType::Connector, CharType::Alnum) => true,
            (CharType::Alnum, CharType::Connector) => true,
            (CharType::Punctuation, CharType::Punctuation) => true,
            (CharType::Space, CharType::Space) => true,
            _ => false,
        }
    }
}

impl InputState {
    /// Select the word at the given offset on double-click.
    ///
    /// The offset is the UTF-8 offset.
    pub(super) fn select_word_for_double_click(
        &mut self,
        offset: usize,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(range) = TextSelector::double_click_select(&self.text, offset) else {
            return;
        };
        self.selected_range = (range.start..range.end).into();
        self.selected_word_range = Some(self.selected_range);
        cx.notify()
    }
}

struct TextSelector;

impl TextSelector {
    /// Select a word in the given text at the specified offset.
    ///
    /// The offset is the UTF-8 offset.
    ///
    /// Returns the start and end offsets of the selected word.
    pub fn double_click_select(text: &Rope, offset: usize) -> Option<Range<usize>> {
        let line = text.byte_to_line_idx(offset, ropey::LineType::LF);
        let line_start_offset = text.line_start_offset(line);
        let line_end_offset = text.line_end_offset(line);
        if line_start_offset == line_end_offset {
            return None;
        }
        let offset = if offset == line_end_offset {
            // set offset to be within line bounds
            line_end_offset - 1
        } else {
            offset
        };
        let Ok(click_char) = text.get_char(offset) else {
            return None;
        };
        let click_char_len = click_char.len_utf8();
        let click_char_type = CharType::from_char(click_char);

        let mut start = offset;
        let mut end = (offset + click_char_len).min(line_end_offset);
        let prev_text = text.slice(line_start_offset..start).to_string();
        let next_text = text.slice(end..line_end_offset).to_string();

        let prev_chars = prev_text.chars().rev();
        let next_chars = next_text.chars();

        let pre_chars_count = prev_chars.clone().count();
        let mut pre_char_type = click_char_type;
        for (ix, c) in prev_chars.enumerate() {
            let char_type = CharType::from_char(c);
            if !pre_char_type.is_connectable(char_type) {
                break;
            }

            if ix < pre_chars_count {
                start = start.saturating_sub(c.len_utf8());
            }
            pre_char_type = char_type;
        }

        let mut pre_char_type = click_char_type;
        for (_, c) in next_chars.enumerate() {
            let char_type = CharType::from_char(c);
            if !pre_char_type.is_connectable(char_type) {
                break;
            }

            end += c.len_utf8();
            pre_char_type = char_type;
        }

        if start == end {
            return None;
        }
        Some(start..end)
    }
}

#[cfg(test)]
mod tests {
    use std::ops::Range;

    use ropey::Rope;

    use super::*;

    trait RopeExt {
        fn text_for_range(&self, range: Range<usize>) -> Option<String>;
    }

    impl RopeExt for Rope {
        fn text_for_range(&self, range: Range<usize>) -> Option<String> {
            let len = self.len();
            if range.start > len || range.end > len || range.start > range.end {
                return None;
            }
            Some(self.slice(range).to_string())
        }
    }

    #[test]
    fn test_char_type_from_char() {
        assert_eq!(CharType::from_char('a'), CharType::Alnum);
        assert_eq!(CharType::from_char('Z'), CharType::Alnum);
        assert_eq!(CharType::from_char('0'), CharType::Alnum);
        assert_eq!(CharType::from_char('_'), CharType::Connector);
        assert_eq!(CharType::from_char('.'), CharType::Punctuation);
        assert_eq!(CharType::from_char(','), CharType::Punctuation);
        assert_eq!(CharType::from_char(';'), CharType::Punctuation);
        assert_eq!(CharType::from_char('!'), CharType::Punctuation);
        assert_eq!(CharType::from_char('?'), CharType::Punctuation);
        assert_eq!(CharType::from_char('['), CharType::Punctuation);
        assert_eq!(CharType::from_char('{'), CharType::Punctuation);
        assert_eq!(CharType::from_char(' '), CharType::Space);
        assert_eq!(CharType::from_char('\n'), CharType::Whitespace);
        assert_eq!(CharType::from_char('\r'), CharType::Whitespace);
        assert_eq!(CharType::from_char('\t'), CharType::Whitespace);
        assert_eq!(CharType::from_char('\u{00A0}'), CharType::Whitespace);
        assert_eq!(CharType::from_char('汉'), CharType::Other);
        assert_eq!(CharType::from_char('é'), CharType::Other);
    }

    #[test]
    fn test_double_click_selection() {
        let rope = Rope::from(
            r#"test text:
abcde中文🎉 test
hello()
test_connector ____
Rope"#,
        );
        // We must use the correct offsets, because the given offsets from double-click are always correct.
        let tests = vec![
            (0, 0, Some("test")),
            (0, 4, Some(" ")),
            (1, 0, Some("abcde")),
            (1, 5, Some("中")),
            (1, 8, Some("文")),
            (1, 11, Some("🎉")),
            (1, 15, Some(" ")),
            (1, 20, Some("test")),
            (2, 7, Some("()")),
            (2, 5, Some("()")),
            (2, 0, Some("hello")),
            (3, 0, Some("test_connector")),
            (3, 4, Some("test_connector")),
            (3, 15, Some("____")),
            (4, 4, Some("Rope")),
        ];
        for (line, column, expected) in tests {
            let line_start_offset = rope.line_start_offset(line);
            let offset = line_start_offset + column;
            let range = TextSelector::double_click_select(&rope, offset);
            let selected_text = range.and_then(|r| rope.text_for_range(r));
            let expected_text = expected.map(|s| s.to_string());
            assert_eq!(selected_text, expected_text);
        }
    }
}
