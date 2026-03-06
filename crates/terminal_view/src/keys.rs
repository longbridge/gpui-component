//! 终端按键到转义序列的转换。
//!
//! 本模块基于 xterm/VT 约定生成输入序列。

use std::borrow::Cow;

use alacritty_terminal::term::TermMode;
use gpui::Keystroke;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum KeyMods {
    None,
    Alt,
    Ctrl,
    Shift,
    CtrlShift,
    Other,
}

impl KeyMods {
    fn from_keystroke(ks: &Keystroke) -> Self {
        match (
            ks.modifiers.alt,
            ks.modifiers.control,
            ks.modifiers.shift,
            ks.modifiers.platform,
        ) {
            (false, false, false, false) => KeyMods::None,
            (true, false, false, false) => KeyMods::Alt,
            (false, true, false, false) => KeyMods::Ctrl,
            (false, false, true, false) => KeyMods::Shift,
            (false, true, true, false) => KeyMods::CtrlShift,
            _ => KeyMods::Other,
        }
    }

    fn has_any(self) -> bool {
        !matches!(self, KeyMods::None)
    }
}

const PLAIN_FKEYS: [(&str, &str); 20] = [
    ("f1", "\x1bOP"),
    ("f2", "\x1bOQ"),
    ("f3", "\x1bOR"),
    ("f4", "\x1bOS"),
    ("f5", "\x1b[15~"),
    ("f6", "\x1b[17~"),
    ("f7", "\x1b[18~"),
    ("f8", "\x1b[19~"),
    ("f9", "\x1b[20~"),
    ("f10", "\x1b[21~"),
    ("f11", "\x1b[23~"),
    ("f12", "\x1b[24~"),
    ("f13", "\x1b[25~"),
    ("f14", "\x1b[26~"),
    ("f15", "\x1b[28~"),
    ("f16", "\x1b[29~"),
    ("f17", "\x1b[31~"),
    ("f18", "\x1b[32~"),
    ("f19", "\x1b[33~"),
    ("f20", "\x1b[34~"),
];

const MOD_FKEY_PREFIX: [(&str, char); 4] = [("f1", 'P'), ("f2", 'Q'), ("f3", 'R'), ("f4", 'S')];

const MOD_FKEY_TILDE: [(&str, u16); 16] = [
    ("f5", 15),
    ("f6", 17),
    ("f7", 18),
    ("f8", 19),
    ("f9", 20),
    ("f10", 21),
    ("f11", 23),
    ("f12", 24),
    ("f13", 25),
    ("f14", 26),
    ("f15", 28),
    ("f16", 29),
    ("f17", 31),
    ("f18", 32),
    ("f19", 33),
    ("f20", 34),
];

const MOD_NAV_SUFFIX: [(&str, char); 6] = [
    ("up", 'A'),
    ("down", 'B'),
    ("right", 'C'),
    ("left", 'D'),
    ("home", 'H'),
    ("end", 'F'),
];

const MOD_TILDE_KEYS: [(&str, u16); 4] =
    [("insert", 2), ("delete", 3), ("pageup", 5), ("pagedown", 6)];

/// 计算 xterm 修饰键参数。
fn modifier_param(ks: &Keystroke) -> u32 {
    let mut value = 0u32;
    if ks.modifiers.shift {
        value |= 1;
    }
    if ks.modifiers.alt {
        value |= 2;
    }
    if ks.modifiers.control {
        value |= 4;
    }
    value + 1
}

fn lookup_str(table: &[(&str, &'static str)], key: &str) -> Option<&'static str> {
    table
        .iter()
        .find_map(|(name, seq)| (*name == key).then_some(*seq))
}

fn lookup_char(table: &[(&str, char)], key: &str) -> Option<char> {
    table
        .iter()
        .find_map(|(name, suffix)| (*name == key).then_some(*suffix))
}

fn lookup_u16(table: &[(&str, u16)], key: &str) -> Option<u16> {
    table
        .iter()
        .find_map(|(name, code)| (*name == key).then_some(*code))
}

fn plain_special_key(key: &str, mods: KeyMods, mode: &TermMode) -> Option<&'static str> {
    let app_cursor = mode.contains(TermMode::APP_CURSOR);
    let alt_screen = mode.contains(TermMode::ALT_SCREEN);

    match mods {
        KeyMods::None => match key {
            "tab" => Some("\x09"),
            "escape" => Some("\x1b"),
            "enter" => Some("\x0d"),
            "backspace" | "back" => Some("\x7f"),
            "home" if app_cursor => Some("\x1bOH"),
            "home" => Some("\x1b[H"),
            "end" if app_cursor => Some("\x1bOF"),
            "end" => Some("\x1b[F"),
            "up" if app_cursor => Some("\x1bOA"),
            "up" => Some("\x1b[A"),
            "down" if app_cursor => Some("\x1bOB"),
            "down" => Some("\x1b[B"),
            "right" if app_cursor => Some("\x1bOC"),
            "right" => Some("\x1b[C"),
            "left" if app_cursor => Some("\x1bOD"),
            "left" => Some("\x1b[D"),
            "insert" => Some("\x1b[2~"),
            "delete" => Some("\x1b[3~"),
            "pageup" => Some("\x1b[5~"),
            "pagedown" => Some("\x1b[6~"),
            _ => lookup_str(&PLAIN_FKEYS, key),
        },
        KeyMods::Shift => match key {
            "tab" => Some("\x1b[Z"),
            "enter" => Some("\x0a"),
            "backspace" | "back" => Some("\x7f"),
            "home" if alt_screen => Some("\x1b[1;2H"),
            "end" if alt_screen => Some("\x1b[1;2F"),
            "pageup" if alt_screen => Some("\x1b[5;2~"),
            "pagedown" if alt_screen => Some("\x1b[6;2~"),
            _ => None,
        },
        KeyMods::Alt => match key {
            "enter" => Some("\x1b\x0d"),
            "backspace" | "back" => Some("\x1b\x7f"),
            _ => None,
        },
        KeyMods::Ctrl => match key {
            "backspace" | "back" => Some("\x08"),
            "space" => Some("\x00"),
            _ => None,
        },
        _ => None,
    }
}

fn ctrl_ascii_sequence(key: &str, mods: KeyMods) -> Option<String> {
    if !matches!(mods, KeyMods::Ctrl | KeyMods::CtrlShift) || key.chars().count() != 1 {
        return None;
    }

    let ch = key.chars().next()?.to_ascii_uppercase();
    let code = if ch.is_ascii_alphabetic() {
        (ch as u8).wrapping_sub(b'@')
    } else {
        match ch {
            '@' => 0,
            '[' => 27,
            '\\' => 28,
            ']' => 29,
            '^' => 30,
            '_' => 31,
            '?' => 127,
            _ => return None,
        }
    };

    Some((code as char).to_string())
}

fn modified_key_sequence(key: &str, ks: &Keystroke) -> Option<String> {
    let param = modifier_param(ks);

    if param == 2 && matches!(key, "up" | "down" | "right" | "left" | "home" | "end") {
        return None;
    }

    if let Some(suffix) = lookup_char(&MOD_NAV_SUFFIX, key) {
        return Some(format!("\x1b[1;{param}{suffix}"));
    }

    if let Some(code) = lookup_u16(&MOD_TILDE_KEYS, key) {
        return Some(format!("\x1b[{code};{param}~"));
    }

    modified_fkey_sequence(key, param)
}

fn modified_fkey_sequence(key: &str, param: u32) -> Option<String> {
    if let Some(suffix) = lookup_char(&MOD_FKEY_PREFIX, key) {
        return Some(format!("\x1b[1;{param}{suffix}"));
    }

    if let Some(code) = lookup_u16(&MOD_FKEY_TILDE, key) {
        return Some(format!("\x1b[{code};{param}~"));
    }

    None
}

fn alt_meta_sequence(ks: &Keystroke, mods: KeyMods) -> Option<String> {
    if !ks.key.is_ascii() || ks.key.chars().count() != 1 {
        return None;
    }

    let can_emit = matches!(mods, KeyMods::Alt)
        || (matches!(mods, KeyMods::Other) && ks.modifiers.alt && ks.modifiers.shift);

    if !can_emit {
        return None;
    }

    let key = if ks.modifiers.shift {
        ks.key.to_ascii_uppercase()
    } else {
        ks.key.clone()
    };
    Some(format!("\x1b{key}"))
}

/// 将按键转换为终端转义序列。
///
/// 对于普通可打印字符返回 `None`，调用方应使用原始字符写入 PTY。
pub fn to_esc_str(
    keystroke: &Keystroke,
    term_mode: &TermMode,
    use_alt_as_meta: bool,
) -> Option<Cow<'static, str>> {
    let key = keystroke.key.as_str();
    let mods = KeyMods::from_keystroke(keystroke);

    if let Some(seq) = plain_special_key(key, mods, term_mode) {
        return Some(Cow::Borrowed(seq));
    }

    if let Some(seq) = ctrl_ascii_sequence(key, mods) {
        return Some(Cow::Owned(seq));
    }

    if mods.has_any() {
        if let Some(seq) = modified_key_sequence(key, keystroke) {
            return Some(Cow::Owned(seq));
        }
    }

    if use_alt_as_meta || !cfg!(target_os = "macos") {
        if let Some(seq) = alt_meta_sequence(keystroke, mods) {
            return Some(Cow::Owned(seq));
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use alacritty_terminal::term::TermMode;
    use gpui::Keystroke;

    use super::to_esc_str;

    #[test]
    fn app_cursor_arrow_mapping() {
        let up = Keystroke::parse("up").unwrap();
        assert_eq!(
            to_esc_str(&up, &TermMode::NONE, false).unwrap().as_ref(),
            "\x1b[A"
        );
        assert_eq!(
            to_esc_str(&up, &TermMode::APP_CURSOR, false)
                .unwrap()
                .as_ref(),
            "\x1bOA"
        );
    }

    #[test]
    fn shift_arrow_stays_none() {
        let shift_up = Keystroke::parse("shift-up").unwrap();
        assert_eq!(to_esc_str(&shift_up, &TermMode::NONE, false), None);
    }

    #[test]
    fn ctrl_letter_mapping() {
        let ctrl_a = Keystroke::parse("ctrl-a").unwrap();
        let seq = to_esc_str(&ctrl_a, &TermMode::NONE, false).unwrap();
        assert_eq!(seq.as_ref().as_bytes(), b"\x01");
    }

    #[test]
    fn alt_meta_mapping() {
        let alt_a = Keystroke::parse("alt-a").unwrap();
        assert_eq!(
            to_esc_str(&alt_a, &TermMode::NONE, true).unwrap().as_ref(),
            "\x1ba"
        );
    }
}
