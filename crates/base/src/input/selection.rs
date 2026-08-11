use std::ops::Range;

use ropey::Rope;
use sum_tree::Bias;

use super::RopeExt as _;

/// Stateless text-selection algorithms shared by input frontends.
pub struct TextSelector;

impl TextSelector {
    pub fn line_range(text: &Rope, offset: usize) -> Range<usize> {
        let offset = text.clip_offset(offset, Bias::Left);
        let row = text.offset_to_point(offset).row;
        text.line_start_offset(row)..text.line_end_offset(row)
    }

    pub fn word_range(text: &Rope, offset: usize) -> Option<Range<usize>> {
        let offset = text.clip_offset(offset, Bias::Left);
        let character = text.char_at(offset)?;
        let end = offset + character.len_utf8();
        Some(word_range_from_chars(
            offset,
            character,
            text.chars_at(offset).reversed().take(128),
            text.chars_at(end).take(128),
        ))
    }
}

#[derive(Clone, Copy)]
enum CharType {
    Word,
    Whitespace,
    Newline,
    Other,
}

impl From<char> for CharType {
    fn from(c: char) -> Self {
        if c == '_'
            || c.is_ascii_alphanumeric()
            || matches!(c, '\u{00C0}'..='\u{024F}' | '\u{0400}'..='\u{04FF}' | '\u{1E00}'..='\u{1EFF}' | '\u{0300}'..='\u{036F}')
        {
            Self::Word
        } else if matches!(c, '\n' | '\r') {
            Self::Newline
        } else if c.is_whitespace() {
            Self::Whitespace
        } else {
            Self::Other
        }
    }
}

impl CharType {
    fn connects(self, c: char) -> bool {
        matches!(
            (self, Self::from(c)),
            (Self::Word, Self::Word) | (Self::Whitespace, Self::Whitespace)
        )
    }
}

fn word_range_from_chars(
    offset: usize,
    c: char,
    prev: impl Iterator<Item = char>,
    next: impl Iterator<Item = char>,
) -> Range<usize> {
    let kind = CharType::from(c);
    let mut start = offset;
    let mut end = offset + c.len_utf8();
    for c in prev.take(128) {
        if kind.connects(c) {
            start -= c.len_utf8();
        } else {
            break;
        }
    }
    for c in next.take(128) {
        if kind.connects(c) {
            end += c.len_utf8();
        } else {
            break;
        }
    }
    start..end
}
