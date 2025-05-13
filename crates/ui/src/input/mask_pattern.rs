use gpui::SharedString;

#[derive(Clone, PartialEq, Debug)]
enum MaskToken {
    /// 0 Digit, equivalent to `[0]`
    // Digit0,
    /// Digit, equivalent to `[0-9]`
    Digit,
    /// Letter, equivalent to `[a-zA-Z]`
    Letter,
    /// Letter or digit, equivalent to `[a-zA-Z0-9]`
    LetterOrDigit,
    /// Decimal separator `.` used for decimal point
    DecimalSep,
    /// Group separator `,` used for thousand separator
    GroupSep,
    /// Separator
    Sep(char),
    /// Any character
    Any,
}

impl MaskToken {
    pub fn is_any(&self) -> bool {
        matches!(self, MaskToken::Any)
    }

    fn is_match(&self, ch: char) -> bool {
        match self {
            MaskToken::Digit => ch.is_ascii_digit(),
            MaskToken::Letter => ch.is_ascii_alphabetic(),
            MaskToken::LetterOrDigit => ch.is_ascii_alphanumeric(),
            MaskToken::DecimalSep => ch == '.',
            MaskToken::GroupSep => ch == ',',
            MaskToken::Any => true,
            MaskToken::Sep(c) => *c == ch,
        }
    }

    fn is_sep(&self) -> bool {
        matches!(self, MaskToken::Sep(_))
    }

    fn is_number(&self) -> bool {
        matches!(
            self,
            MaskToken::Digit
                | MaskToken::LetterOrDigit
                | MaskToken::DecimalSep
                | MaskToken::GroupSep
        )
    }

    pub fn placeholder(&self) -> char {
        match self {
            MaskToken::DecimalSep | MaskToken::GroupSep => '.',
            MaskToken::Sep(c) => *c,
            _ => '_',
        }
    }

    fn mask_char(&self, ch: char) -> char {
        match self {
            MaskToken::Digit => ch,
            MaskToken::Letter => ch,
            MaskToken::LetterOrDigit => ch,
            MaskToken::DecimalSep => '.',
            MaskToken::GroupSep => ',',
            MaskToken::Sep(c) => *c,
            MaskToken::Any => ch,
        }
    }

    fn unmask_char(&self, ch: char) -> Option<char> {
        match self {
            MaskToken::Digit => Some(ch),
            MaskToken::Letter => Some(ch),
            MaskToken::LetterOrDigit => Some(ch),
            MaskToken::Any => Some(ch),
            _ => None,
        }
    }
}

#[derive(Clone, Default)]
pub struct MaskPattern {
    pattern: SharedString,
    tokens: Vec<MaskToken>,
}

impl From<&str> for MaskPattern {
    fn from(pattern: &str) -> Self {
        Self::new(pattern)
    }
}

impl MaskPattern {
    pub fn new(pattern: &str) -> Self {
        let tokens = pattern
            .chars()
            .map(|ch| match ch {
                // '0' => MaskToken::Digit0,
                '9' => MaskToken::Digit,
                'A' => MaskToken::Letter,
                '#' => MaskToken::LetterOrDigit,
                '.' => MaskToken::DecimalSep,
                ',' => MaskToken::GroupSep,
                '*' => MaskToken::Any,
                _ => MaskToken::Sep(ch),
            })
            .collect();

        Self {
            tokens,
            pattern: pattern.to_owned().into(),
        }
    }

    pub fn pattern(&self) -> &SharedString {
        &self.pattern
    }

    pub fn placeholder(&self) -> String {
        self.tokens
            .iter()
            .map(|token| token.placeholder())
            .collect()
    }

    pub fn is_empty(&self) -> bool {
        self.tokens.is_empty()
    }

    /// Check is the mask text is valid.
    ///
    /// If the mask pattern is None, always return true.
    pub fn is_valid(&self, mask_text: &str) -> bool {
        if self.is_empty() {
            return true;
        }

        let mut text_index = 0;
        let mask_text_chars: Vec<char> = mask_text.chars().collect();
        for token in &self.tokens {
            if text_index >= mask_text_chars.len() {
                break;
            }

            let ch = mask_text_chars[text_index];
            if token.is_match(ch) {
                text_index += 1;
            }
        }
        text_index == mask_text.len()
    }

    /// Check if valid input char at the given position.
    pub fn is_valid_at(&self, ch: char, pos: usize) -> bool {
        if self.is_empty() {
            return true;
        }

        let mut pos = pos;
        while let Some(token) = self.tokens.get(pos) {
            if token.is_match(ch) {
                return true;
            }

            if !token.is_sep() {
                break;
            }

            pos += 1;
        }

        false
    }

    /// Format the text according to the mask pattern
    ///
    /// For example:
    ///
    /// - pattern: (999)999-999
    /// - text: 123456789
    /// - mask_text: (123)456-789
    pub fn mask(&self, text: &str) -> SharedString {
        if self.is_empty() {
            return text.to_owned().into();
        }

        let mut result = String::new();

        let mut text_index = 0;
        let text_chars: Vec<char> = text.chars().collect();

        for token in &self.tokens {
            if text_index >= text_chars.len() {
                break;
            }

            let ch = text_chars[text_index];
            let mask_ch = token.mask_char(ch);
            result.push(mask_ch);
            if ch == mask_ch {
                text_index += 1;
            }
        }

        result.into()
    }

    /// Extract original text from masked text
    pub fn unmask(&self, mask_text: &str) -> String {
        let mut result = String::new();
        let mask_text_chars: Vec<char> = mask_text.chars().collect();

        for (text_index, token) in self.tokens.iter().enumerate() {
            if text_index >= mask_text_chars.len() {
                break;
            }

            let ch = mask_text_chars[text_index];
            let unmask_ch = token.unmask_char(ch);
            if let Some(ch) = unmask_ch {
                result.push(ch);
            }
        }

        result
    }

    // /// Convert pos in masked text to pos in source text
    // ///
    // /// For example:
    // ///
    // /// Raw Text: 123456
    // /// Masked text: (123)456
    // ///
    // /// - position_of_raw_text(0) = 0
    // /// - position_of_raw_text(1) = 0
    // /// - position_of_raw_text(2) = 1
    // /// - position_of_raw_text(3) = 2
    // /// - position_of_raw_text(4) = 3
    // /// - position_of_raw_text(5) = 3
    // /// - position_of_raw_text(6) = 4
    // pub(crate) fn position_of_raw_text(&self, masked_pos: usize) -> usize {
    //     let Some(pattern) = &self.mask_pattern else {
    //         return masked_pos;
    //     };

    //     let mut raw_pos = 0;

    //     for (i, c) in pattern.chars().enumerate() {
    //         if i >= masked_pos {
    //             break;
    //         }

    //         match c {
    //             '9' | 'A' | '*' => {
    //                 if let Some(ch) = self.text[raw_pos..].chars().next() {
    //                     if (c == '9' && ch.is_ascii_digit())
    //                         || (c == 'A' && ch.is_ascii_alphabetic())
    //                         || (c == '*')
    //                     {
    //                         raw_pos += 1;
    //                     }
    //                 }
    //             }
    //             _ => {
    //                 raw_pos += 1;
    //             }
    //         }
    //     }

    //     raw_pos
    // }

    // /// Check if the given position is a mask position
    // pub(crate) fn is_mask_position(&self, pos: usize) -> bool {
    //     let masked_pos = self.position_of_marked_text(pos);
    //     masked_pos != pos
    // }

    // /// Convert pos in source text to pos in masked text
    // ///
    // /// For example:
    // ///
    // /// Raw Text: 123456
    // /// Masked Text: (123)456
    // ///
    // /// - position_of_marked_text(0) = 1
    // /// - position_of_marked_text(1) = 2
    // /// - position_of_marked_text(2) = 3
    // /// - position_of_marked_text(3) = 5
    // /// - position_of_marked_text(4) = 6
    // /// - position_of_marked_text(5) = 7
    // /// - position_of_marked_text(6) = 8
    // ///
    // pub(crate) fn position_of_marked_text(&self, pos: usize) -> usize {
    //     let Some(pattern) = &self.mask_pattern else {
    //         return pos;
    //     };
    //     let mut marked_pos = 0;
    //     let mut text_index = 0;

    //     for (i, c) in pattern.chars().enumerate() {
    //         if text_index >= pos {
    //             break;
    //         }

    //         match c {
    //             '9' | 'A' | '*' => {
    //                 if let Some(ch) = self.text[text_index..].chars().next() {
    //                     if (c == '9' && ch.is_ascii_digit())
    //                         || (c == 'A' && ch.is_ascii_alphabetic())
    //                         || (c == '*')
    //                     {
    //                         text_index += 1;
    //                         marked_pos = i + 1;
    //                     }
    //                 }
    //             }
    //             _ => {
    //                 marked_pos = i + 1;
    //                 if self.text[text_index..].chars().next() == Some(c) {
    //                     text_index += 1;
    //                 }
    //             }
    //         }
    //     }

    //     marked_pos
    // }
}

#[cfg(test)]
mod tests {
    use crate::input::mask_pattern::{MaskPattern, MaskToken};

    #[test]
    fn test_mask_pattern1() {
        let mask = MaskPattern::new("(AA)999-999");
        assert_eq!(
            mask.tokens,
            vec![
                MaskToken::Sep('('),
                MaskToken::Letter,
                MaskToken::Letter,
                MaskToken::Sep(')'),
                MaskToken::Digit,
                MaskToken::Digit,
                MaskToken::Digit,
                MaskToken::Sep('-'),
                MaskToken::Digit,
                MaskToken::Digit,
                MaskToken::Digit,
            ]
        );

        assert_eq!(mask.is_valid_at('(', 0), true);
        assert_eq!(mask.is_valid_at('H', 0), true);
        assert_eq!(mask.is_valid_at('3', 0), false);
        assert_eq!(mask.is_valid_at(')', 1), false);
        assert_eq!(mask.is_valid_at('H', 1), true);
        assert_eq!(mask.is_valid_at('1', 1), false);
        assert_eq!(mask.is_valid_at('e', 2), true);
        assert_eq!(mask.is_valid_at(')', 3), true);
        assert_eq!(mask.is_valid_at('1', 3), true);
        assert_eq!(mask.is_valid_at('2', 4), true);

        let text = "AB123456";
        let masked_text = mask.mask(text);
        assert_eq!(masked_text, "(AB)123-456");
        let unmasked_text = mask.unmask(&masked_text);
        assert_eq!(unmasked_text, "AB123456");
        assert_eq!(mask.is_valid(&masked_text), true);

        assert_eq!(mask.is_valid("12AB345"), false);
        assert_eq!(mask.is_valid("(11)123-456"), false);
        assert_eq!(mask.is_valid("##"), false);
        assert_eq!(mask.is_valid("(AB)123456"), true);
    }

    #[test]
    fn test_mask_pattern2() {
        let mask = MaskPattern::new("999-999-******");
        assert_eq!(
            mask.tokens,
            vec![
                MaskToken::Digit,
                MaskToken::Digit,
                MaskToken::Digit,
                MaskToken::Sep('-'),
                MaskToken::Digit,
                MaskToken::Digit,
                MaskToken::Digit,
                MaskToken::Sep('-'),
                MaskToken::Any,
                MaskToken::Any,
                MaskToken::Any,
                MaskToken::Any,
                MaskToken::Any,
                MaskToken::Any,
            ]
        );

        let text = "123456A(111)";
        let masked_text = mask.mask(text);
        assert_eq!(masked_text, "123-456-A(111)");
        let unmasked_text = mask.unmask(&masked_text);
        assert_eq!(unmasked_text, "123456A(111)");
        assert_eq!(mask.is_valid(&masked_text), true);
    }
}
