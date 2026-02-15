//! Terminal keyboard escape sequence generation
//!
//! Converts GPUI keystrokes to terminal escape sequences based on xterm/VT conventions.

use std::borrow::Cow;

use alacritty_terminal::term::TermMode;
use gpui::Keystroke;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ModifierState {
    Plain,
    OnlyAlt,
    OnlyCtrl,
    OnlyShift,
    CtrlWithShift,
    Mixed,
}

impl ModifierState {
    fn from_keystroke(ks: &Keystroke) -> Self {
        let alt = ks.modifiers.alt;
        let ctrl = ks.modifiers.control;
        let shift = ks.modifiers.shift;
        let platform = ks.modifiers.platform;

        if platform {
            return ModifierState::Mixed;
        }

        match (alt, ctrl, shift) {
            (false, false, false) => ModifierState::Plain,
            (true, false, false) => ModifierState::OnlyAlt,
            (false, true, false) => ModifierState::OnlyCtrl,
            (false, false, true) => ModifierState::OnlyShift,
            (false, true, true) => ModifierState::CtrlWithShift,
            _ => ModifierState::Mixed,
        }
    }

    fn has_modifier(self) -> bool {
        !matches!(self, ModifierState::Plain)
    }
}

/// Computes the xterm modifier parameter value.
/// See: https://invisible-island.net/xterm/ctlseqs/ctlseqs.html#h2-PC-Style-Function-Keys
fn compute_modifier_param(ks: &Keystroke) -> u32 {
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

/// Converts a keystroke to its terminal escape sequence representation.
///
/// Returns `None` for regular printable characters that should be handled
/// by the caller using `key_char` or direct character output.
pub fn to_esc_str(
    keystroke: &Keystroke,
    term_mode: &TermMode,
    use_alt_as_meta: bool,
) -> Option<Cow<'static, str>> {
    let mods = ModifierState::from_keystroke(keystroke);
    let key = keystroke.key.as_ref();

    if let Some(seq) = try_special_key_binding(key, mods, term_mode) {
        return Some(Cow::Borrowed(seq));
    }

    if let Some(seq) = try_control_char_binding(key, mods) {
        return Some(Cow::Borrowed(seq));
    }

    if mods.has_modifier() {
        if let Some(seq) = try_modified_key_sequence(key, keystroke) {
            return Some(Cow::Owned(seq));
        }
    }

    if use_alt_as_meta || !cfg!(target_os = "macos") {
        if let Some(seq) = try_alt_sequence(keystroke, mods) {
            return Some(Cow::Owned(seq));
        }
    }

    None
}

fn try_special_key_binding(
    key: &str,
    mods: ModifierState,
    term_mode: &TermMode,
) -> Option<&'static str> {
    let app_cursor = term_mode.contains(TermMode::APP_CURSOR);
    let alt_screen = term_mode.contains(TermMode::ALT_SCREEN);

    match (key, mods) {
        ("tab", ModifierState::Plain) => Some("\x09"),
        ("tab", ModifierState::OnlyShift) => Some("\x1b[Z"),
        ("escape", ModifierState::Plain) => Some("\x1b"),
        ("enter", ModifierState::Plain) => Some("\x0d"),
        ("enter", ModifierState::OnlyShift) => Some("\x0a"),
        ("enter", ModifierState::OnlyAlt) => Some("\x1b\x0d"),
        ("backspace" | "back", ModifierState::Plain) => Some("\x7f"),
        ("backspace", ModifierState::OnlyShift) => Some("\x7f"),
        ("backspace", ModifierState::OnlyCtrl) => Some("\x08"),
        ("backspace", ModifierState::OnlyAlt) => Some("\x1b\x7f"),
        ("space", ModifierState::OnlyCtrl) => Some("\x00"),

        ("home", ModifierState::Plain) if app_cursor => Some("\x1bOH"),
        ("home", ModifierState::Plain) => Some("\x1b[H"),
        ("home", ModifierState::OnlyShift) if alt_screen => Some("\x1b[1;2H"),
        ("end", ModifierState::Plain) if app_cursor => Some("\x1bOF"),
        ("end", ModifierState::Plain) => Some("\x1b[F"),
        ("end", ModifierState::OnlyShift) if alt_screen => Some("\x1b[1;2F"),

        ("up", ModifierState::Plain) if app_cursor => Some("\x1bOA"),
        ("up", ModifierState::Plain) => Some("\x1b[A"),
        ("down", ModifierState::Plain) if app_cursor => Some("\x1bOB"),
        ("down", ModifierState::Plain) => Some("\x1b[B"),
        ("right", ModifierState::Plain) if app_cursor => Some("\x1bOC"),
        ("right", ModifierState::Plain) => Some("\x1b[C"),
        ("left", ModifierState::Plain) if app_cursor => Some("\x1bOD"),
        ("left", ModifierState::Plain) => Some("\x1b[D"),

        ("insert", ModifierState::Plain) => Some("\x1b[2~"),
        ("delete", ModifierState::Plain) => Some("\x1b[3~"),
        ("pageup", ModifierState::Plain) => Some("\x1b[5~"),
        ("pageup", ModifierState::OnlyShift) if alt_screen => Some("\x1b[5;2~"),
        ("pagedown", ModifierState::Plain) => Some("\x1b[6~"),
        ("pagedown", ModifierState::OnlyShift) if alt_screen => Some("\x1b[6;2~"),

        ("f1", ModifierState::Plain) => Some("\x1bOP"),
        ("f2", ModifierState::Plain) => Some("\x1bOQ"),
        ("f3", ModifierState::Plain) => Some("\x1bOR"),
        ("f4", ModifierState::Plain) => Some("\x1bOS"),
        ("f5", ModifierState::Plain) => Some("\x1b[15~"),
        ("f6", ModifierState::Plain) => Some("\x1b[17~"),
        ("f7", ModifierState::Plain) => Some("\x1b[18~"),
        ("f8", ModifierState::Plain) => Some("\x1b[19~"),
        ("f9", ModifierState::Plain) => Some("\x1b[20~"),
        ("f10", ModifierState::Plain) => Some("\x1b[21~"),
        ("f11", ModifierState::Plain) => Some("\x1b[23~"),
        ("f12", ModifierState::Plain) => Some("\x1b[24~"),
        ("f13", ModifierState::Plain) => Some("\x1b[25~"),
        ("f14", ModifierState::Plain) => Some("\x1b[26~"),
        ("f15", ModifierState::Plain) => Some("\x1b[28~"),
        ("f16", ModifierState::Plain) => Some("\x1b[29~"),
        ("f17", ModifierState::Plain) => Some("\x1b[31~"),
        ("f18", ModifierState::Plain) => Some("\x1b[32~"),
        ("f19", ModifierState::Plain) => Some("\x1b[33~"),
        ("f20", ModifierState::Plain) => Some("\x1b[34~"),

        _ => None,
    }
}

fn try_control_char_binding(key: &str, mods: ModifierState) -> Option<&'static str> {
    match (key, mods) {
        ("a", ModifierState::OnlyCtrl) | ("A", ModifierState::CtrlWithShift) => Some("\x01"),
        ("b", ModifierState::OnlyCtrl) | ("B", ModifierState::CtrlWithShift) => Some("\x02"),
        ("c", ModifierState::OnlyCtrl) | ("C", ModifierState::CtrlWithShift) => Some("\x03"),
        ("d", ModifierState::OnlyCtrl) | ("D", ModifierState::CtrlWithShift) => Some("\x04"),
        ("e", ModifierState::OnlyCtrl) | ("E", ModifierState::CtrlWithShift) => Some("\x05"),
        ("f", ModifierState::OnlyCtrl) | ("F", ModifierState::CtrlWithShift) => Some("\x06"),
        ("g", ModifierState::OnlyCtrl) | ("G", ModifierState::CtrlWithShift) => Some("\x07"),
        ("h", ModifierState::OnlyCtrl) | ("H", ModifierState::CtrlWithShift) => Some("\x08"),
        ("i", ModifierState::OnlyCtrl) | ("I", ModifierState::CtrlWithShift) => Some("\x09"),
        ("j", ModifierState::OnlyCtrl) | ("J", ModifierState::CtrlWithShift) => Some("\x0a"),
        ("k", ModifierState::OnlyCtrl) | ("K", ModifierState::CtrlWithShift) => Some("\x0b"),
        ("l", ModifierState::OnlyCtrl) | ("L", ModifierState::CtrlWithShift) => Some("\x0c"),
        ("m", ModifierState::OnlyCtrl) | ("M", ModifierState::CtrlWithShift) => Some("\x0d"),
        ("n", ModifierState::OnlyCtrl) | ("N", ModifierState::CtrlWithShift) => Some("\x0e"),
        ("o", ModifierState::OnlyCtrl) | ("O", ModifierState::CtrlWithShift) => Some("\x0f"),
        ("p", ModifierState::OnlyCtrl) | ("P", ModifierState::CtrlWithShift) => Some("\x10"),
        ("q", ModifierState::OnlyCtrl) | ("Q", ModifierState::CtrlWithShift) => Some("\x11"),
        ("r", ModifierState::OnlyCtrl) | ("R", ModifierState::CtrlWithShift) => Some("\x12"),
        ("s", ModifierState::OnlyCtrl) | ("S", ModifierState::CtrlWithShift) => Some("\x13"),
        ("t", ModifierState::OnlyCtrl) | ("T", ModifierState::CtrlWithShift) => Some("\x14"),
        ("u", ModifierState::OnlyCtrl) | ("U", ModifierState::CtrlWithShift) => Some("\x15"),
        ("v", ModifierState::OnlyCtrl) | ("V", ModifierState::CtrlWithShift) => Some("\x16"),
        ("w", ModifierState::OnlyCtrl) | ("W", ModifierState::CtrlWithShift) => Some("\x17"),
        ("x", ModifierState::OnlyCtrl) | ("X", ModifierState::CtrlWithShift) => Some("\x18"),
        ("y", ModifierState::OnlyCtrl) | ("Y", ModifierState::CtrlWithShift) => Some("\x19"),
        ("z", ModifierState::OnlyCtrl) | ("Z", ModifierState::CtrlWithShift) => Some("\x1a"),
        ("@", ModifierState::OnlyCtrl) => Some("\x00"),
        ("[", ModifierState::OnlyCtrl) => Some("\x1b"),
        ("\\", ModifierState::OnlyCtrl) => Some("\x1c"),
        ("]", ModifierState::OnlyCtrl) => Some("\x1d"),
        ("^", ModifierState::OnlyCtrl) => Some("\x1e"),
        ("_", ModifierState::OnlyCtrl) => Some("\x1f"),
        ("?", ModifierState::OnlyCtrl) => Some("\x7f"),
        _ => None,
    }
}

fn try_modified_key_sequence(key: &str, ks: &Keystroke) -> Option<String> {
    let param = compute_modifier_param(ks);

    if param == 2 {
        return match key {
            "up" | "down" | "right" | "left" | "home" | "end" => None,
            "f1" | "f2" | "f3" | "f4" | "f5" | "f6" | "f7" | "f8" | "f9" | "f10" | "f11"
            | "f12" | "f13" | "f14" | "f15" | "f16" | "f17" | "f18" | "f19" | "f20" => {
                build_modified_fkey(key, param)
            }
            _ => None,
        };
    }

    match key {
        "up" => Some(format!("\x1b[1;{param}A")),
        "down" => Some(format!("\x1b[1;{param}B")),
        "right" => Some(format!("\x1b[1;{param}C")),
        "left" => Some(format!("\x1b[1;{param}D")),
        "home" => Some(format!("\x1b[1;{param}H")),
        "end" => Some(format!("\x1b[1;{param}F")),
        "insert" => Some(format!("\x1b[2;{param}~")),
        "delete" => Some(format!("\x1b[3;{param}~")),
        "pageup" => Some(format!("\x1b[5;{param}~")),
        "pagedown" => Some(format!("\x1b[6;{param}~")),
        _ => build_modified_fkey(key, param),
    }
}

fn build_modified_fkey(key: &str, param: u32) -> Option<String> {
    match key {
        "f1" => Some(format!("\x1b[1;{param}P")),
        "f2" => Some(format!("\x1b[1;{param}Q")),
        "f3" => Some(format!("\x1b[1;{param}R")),
        "f4" => Some(format!("\x1b[1;{param}S")),
        "f5" => Some(format!("\x1b[15;{param}~")),
        "f6" => Some(format!("\x1b[17;{param}~")),
        "f7" => Some(format!("\x1b[18;{param}~")),
        "f8" => Some(format!("\x1b[19;{param}~")),
        "f9" => Some(format!("\x1b[20;{param}~")),
        "f10" => Some(format!("\x1b[21;{param}~")),
        "f11" => Some(format!("\x1b[23;{param}~")),
        "f12" => Some(format!("\x1b[24;{param}~")),
        "f13" => Some(format!("\x1b[25;{param}~")),
        "f14" => Some(format!("\x1b[26;{param}~")),
        "f15" => Some(format!("\x1b[28;{param}~")),
        "f16" => Some(format!("\x1b[29;{param}~")),
        "f17" => Some(format!("\x1b[31;{param}~")),
        "f18" => Some(format!("\x1b[32;{param}~")),
        "f19" => Some(format!("\x1b[33;{param}~")),
        "f20" => Some(format!("\x1b[34;{param}~")),
        _ => None,
    }
}

fn try_alt_sequence(ks: &Keystroke, mods: ModifierState) -> Option<String> {
    let key = &ks.key;

    let is_single_ascii = key.len() == 1 && key.is_ascii();
    if !is_single_ascii {
        return None;
    }

    let emit_alt_seq = match mods {
        ModifierState::OnlyAlt => true,
        ModifierState::Mixed if ks.modifiers.alt && ks.modifiers.shift => true,
        _ => false,
    };

    if emit_alt_seq {
        let char_to_send = if ks.modifiers.shift {
            key.to_ascii_uppercase()
        } else {
            key.clone()
        };
        return Some(format!("\x1b{char_to_send}"));
    }

    None
}
