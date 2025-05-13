use gpui::SharedString;

#[derive(Clone, Default)]
pub enum MaskPattern {
    #[default]
    Any,
    Str(SharedString),
    Regex(regex::Regex),
}

impl From<&str> for MaskPattern {
    fn from(value: &str) -> Self {
        MaskPattern::Str(value.to_owned().into())
    }
}

impl From<regex::Regex> for MaskPattern {
    fn from(value: regex::Regex) -> Self {
        MaskPattern::Regex(value)
    }
}

impl MaskPattern {
    pub(crate) fn is_any(&self) -> bool {
        matches!(self, MaskPattern::Any)
    }

    /// Check is the mask text is valid.
    ///
    /// If the mask pattern is None, always return true.
    pub(crate) fn is_valid(&self, mask_text: &str) -> bool {
        if self.is_any() {
            return true;
        }

        let mut text_index = 0;
        match self {
            MaskPattern::Any => return true,
            MaskPattern::Str(pattern) => {
                for c in pattern.chars() {
                    if text_index >= mask_text.len() {
                        break;
                    }
                    match c {
                        '9' => {
                            if !mask_text[text_index..]
                                .chars()
                                .next()
                                .map_or(false, |c| c.is_ascii_digit())
                            {
                                return false;
                            }
                        }
                        'A' => {
                            if !mask_text[text_index..]
                                .chars()
                                .next()
                                .map_or(false, |c| c.is_ascii_alphabetic())
                            {
                                return false;
                            }
                        }
                        '*' => {}
                        _ => {
                            if mask_text[text_index..].chars().next() != Some(c) {
                                return false;
                            }
                        }
                    }
                    text_index += 1;
                }
            }
            MaskPattern::Regex(pattern) => {
                if !pattern.is_match(mask_text) {
                    return false;
                }
            }
        }

        true
    }

    /// Format the text according to the mask pattern
    pub(crate) fn mask(&self, text: &str) -> SharedString {
        if self.is_any() {
            return text.to_owned().into();
        }

        let mut result = String::new();
        let mut text_index = 0;

        match self {
            MaskPattern::Any => return text.to_owned().into(),
            MaskPattern::Str(pattern) => {
                for c in pattern.chars() {
                    if text_index >= text.len() {
                        break;
                    }
                    match c {
                        '9' => {
                            if text[text_index..]
                                .chars()
                                .next()
                                .map_or(false, |c| c.is_ascii_digit())
                            {
                                result.push(text[text_index..].chars().next().unwrap());
                                text_index += 1;
                            }
                        }
                        'A' => {
                            if text[text_index..]
                                .chars()
                                .next()
                                .map_or(false, |c| c.is_ascii_alphabetic())
                            {
                                result.push(text[text_index..].chars().next().unwrap());
                                text_index += 1;
                            }
                        }
                        '*' => {
                            result.push(text[text_index..].chars().next().unwrap());
                            text_index += 1;
                        }
                        _ => {
                            result.push(c);
                            if text_index < text.len()
                                && text[text_index..].chars().next() == Some(c)
                            {
                                text_index += 1;
                            }
                        }
                    }
                }
            }
            MaskPattern::Regex(pattern) => {
                if !pattern.is_match(text) {
                    return text.to_owned().into();
                }
                let captures = pattern.captures(text).unwrap();
                for i in 0..captures.len() {
                    if let Some(m) = captures.get(i) {
                        result.push_str(m.as_str());
                    }
                }
            }
        }

        result.into()
    }

    /// Extract original text from masked text
    pub(crate) fn unmask(&self, mask_text: &str) -> String {
        let mut result = String::new();
        let mut text_index = 0;

        match self {
            MaskPattern::Any => return mask_text.to_owned(),
            MaskPattern::Str(pattern) => {
                for c in pattern.chars() {
                    if text_index >= mask_text.len() {
                        break;
                    }

                    match c {
                        '9' | 'A' | '*' => {
                            if let Some(ch) = mask_text[text_index..].chars().next() {
                                if (c == '9' && ch.is_ascii_digit())
                                    || (c == 'A' && ch.is_ascii_alphabetic())
                                    || (c == '*')
                                {
                                    result.push(ch);
                                }
                                text_index += 1;
                            }
                        }
                        _ => {
                            if mask_text[text_index..].chars().next() == Some(c) {
                                text_index += 1;
                            }
                        }
                    }
                }
            }
            MaskPattern::Regex(_) => {
                // Regex patterns are not supported for unmasking
                // This is a placeholder implementation
                result = mask_text.to_owned();
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
    use crate::input::mask_pattern::MaskPattern;

    #[test]
    fn test_mask_pattern1() {
        let mask = MaskPattern::from("(AA)999-999");
        let text = "AB123456";
        let masked_text = mask.mask(text);
        assert_eq!(masked_text, "(AB)123-456");
        let unmasked_text = mask.unmask(&masked_text);
        assert_eq!(unmasked_text, "AB123456");
        assert_eq!(mask.is_valid(&masked_text), true);

        let invalid_text = "AB12345";
        assert_eq!(mask.is_valid(invalid_text), false);
    }

    #[test]
    fn test_mask_pattern2() {
        let mask = MaskPattern::from("999-999-******");
        let text = "123456A(111)";
        let masked_text = mask.mask(text);
        assert_eq!(masked_text, "123-456-A(111)");
        let unmasked_text = mask.unmask(&masked_text);
        assert_eq!(unmasked_text, "123456A(111)");
        assert_eq!(mask.is_valid(&masked_text), true);
    }
}
