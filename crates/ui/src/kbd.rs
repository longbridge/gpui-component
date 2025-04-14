use gpui::{
    div, relative, Action, IntoElement, KeyContext, Keystroke, ParentElement as _, RenderOnce,
    Styled as _, Window,
};

use crate::ActiveTheme;

/// A key binding tag
#[derive(IntoElement, Clone, Debug)]
pub struct Kbd {
    stroke: gpui::Keystroke,
    appearance: bool,
}

impl From<Keystroke> for Kbd {
    fn from(stroke: Keystroke) -> Self {
        Self {
            stroke,
            appearance: true,
        }
    }
}

impl Kbd {
    pub fn new(stroke: Keystroke) -> Self {
        Self {
            stroke,
            appearance: true,
        }
    }

    /// Set the appearance of the keybinding.
    pub fn appearance(mut self, appearance: bool) -> Self {
        self.appearance = appearance;
        self
    }

    /// Return the first keybinding for the given action and context.
    pub fn binding_for_action(
        action: &dyn Action,
        context: Option<&str>,
        window: &Window,
    ) -> Option<Self> {
        let key_context = context.and_then(|context| KeyContext::parse(context).ok());
        let bindings = match key_context {
            Some(context) => window.bindings_for_action_in_context(action, context),
            None => window.bindings_for_action(action),
        };

        bindings.first().and_then(|binding| {
            if let Some(key) = binding.keystrokes().first() {
                Some(Self::new(key.clone()))
            } else {
                None
            }
        })
    }

    /// Return the Platform specific keybinding string by KeyStroke
    ///
    /// macOS: https://support.apple.com/en-us/HT201236
    /// Windows: https://support.microsoft.com/en-us/windows/keyboard-shortcuts-in-windows-dcc61a57-8ff0-cffe-9796-cb9706c75eec
    pub fn format(key: &Keystroke) -> String {
        #[cfg(target_os = "macos")]
        const DIVIDER: &str = "";
        #[cfg(not(target_os = "macos"))]
        const DIVIDER: &str = "+";

        let mut parts = vec![];

        // The key map order in macOS is: ⌃⌥⇧⌘
        // And in Windows is: Ctrl+Alt+Shift+Win

        if key.modifiers.control {
            #[cfg(target_os = "macos")]
            parts.push("⌃");

            #[cfg(not(target_os = "macos"))]
            parts.push("Ctrl");
        }

        if key.modifiers.alt {
            #[cfg(target_os = "macos")]
            parts.push("⌥");

            #[cfg(not(target_os = "macos"))]
            parts.push("Alt");
        }

        if key.modifiers.shift {
            #[cfg(target_os = "macos")]
            parts.push("⇧");

            #[cfg(not(target_os = "macos"))]
            parts.push("Shift");
        }

        if key.modifiers.platform {
            #[cfg(target_os = "macos")]
            parts.push("⌘");

            #[cfg(not(target_os = "macos"))]
            parts.push("Win");
        }

        let mut keys = String::new();

        for key in key.key.split("-") {
            if parts.len() > 0 && keys.len() > 0 {
                keys.push_str(DIVIDER);
            }

            match key {
                #[cfg(target_os = "macos")]
                "ctrl" => keys.push('⌃'),
                #[cfg(not(target_os = "macos"))]
                "ctrl" => keys.push_str("Ctrl"),
                #[cfg(target_os = "macos")]
                "alt" => keys.push('⌥'),
                #[cfg(not(target_os = "macos"))]
                "alt" => keys.push_str("Alt"),
                #[cfg(target_os = "macos")]
                "shift" => keys.push('⇧'),
                #[cfg(not(target_os = "macos"))]
                "shift" => keys.push_str("Shift"),
                #[cfg(target_os = "macos")]
                "cmd" => keys.push('⌘'),
                #[cfg(not(target_os = "macos"))]
                "cmd" => keys.push_str("Win"),
                #[cfg(target_os = "macos")]
                "space" => keys.push_str("Space"),
                #[cfg(target_os = "macos")]
                "backspace" => keys.push('⌫'),
                #[cfg(not(target_os = "macos"))]
                "backspace" => keys.push_str("Backspace"),
                #[cfg(target_os = "macos")]
                "delete" => keys.push('⌫'),
                #[cfg(not(target_os = "macos"))]
                "delete" => keys.push_str("Delete"),
                #[cfg(target_os = "macos")]
                "escape" => keys.push('⎋'),
                #[cfg(not(target_os = "macos"))]
                "escape" => keys.push_str("Esc"),
                #[cfg(target_os = "macos")]
                "enter" => keys.push('⏎'),
                #[cfg(not(target_os = "macos"))]
                "enter" => keys.push_str("Enter"),
                "pagedown" => keys.push_str("Page Down"),
                "pageup" => keys.push_str("Page Up"),
                #[cfg(target_os = "macos")]
                "left" => keys.push('←'),
                #[cfg(not(target_os = "macos"))]
                "left" => keys.push_str("Left"),
                #[cfg(target_os = "macos")]
                "right" => keys.push('→'),
                #[cfg(not(target_os = "macos"))]
                "right" => keys.push_str("Right"),
                #[cfg(target_os = "macos")]
                "up" => keys.push('↑'),
                #[cfg(not(target_os = "macos"))]
                "up" => keys.push_str("Up"),
                #[cfg(target_os = "macos")]
                "down" => keys.push('↓'),
                #[cfg(not(target_os = "macos"))]
                "down" => keys.push_str("Down"),
                _ => {
                    if key.len() == 1 {
                        keys.push_str(&key.to_uppercase());
                    } else {
                        if let Some(first_char) = key.chars().next() {
                            keys.push_str(&format!("{}{}", first_char.to_uppercase(), &key[1..]));
                        } else {
                            keys.push_str(&key);
                        }
                    }
                }
            }
        }

        parts.push(&keys);
        parts.join(DIVIDER)
    }
}

impl RenderOnce for Kbd {
    fn render(self, _: &mut gpui::Window, cx: &mut gpui::App) -> impl gpui::IntoElement {
        if !self.appearance {
            return Self::format(&self.stroke).into_any_element();
        }

        div()
            .border_1()
            .border_color(cx.theme().border)
            .text_color(cx.theme().muted_foreground)
            .bg(cx.theme().background)
            .py_0p5()
            .px_1()
            .min_w_5()
            .text_center()
            .rounded_sm()
            .line_height(relative(1.))
            .text_xs()
            .child(Self::format(&self.stroke))
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_format() {
        use super::Kbd;
        use gpui::Keystroke;

        if cfg!(target_os = "macos") {
            assert_eq!(Kbd::format(&Keystroke::parse("cmd-a").unwrap()), "⌘A");
            assert_eq!(Kbd::format(&Keystroke::parse("cmd-enter").unwrap()), "⌘⏎");
            assert_eq!(
                Kbd::format(&Keystroke::parse("secondary-f12").unwrap()),
                "⌘F12"
            );
            assert_eq!(
                Kbd::format(&Keystroke::parse("shift-pagedown").unwrap()),
                "⇧Page Down"
            );
            assert_eq!(
                Kbd::format(&Keystroke::parse("shift-pageup").unwrap()),
                "⇧Page Up"
            );
            assert_eq!(
                Kbd::format(&Keystroke::parse("shift-space").unwrap()),
                "⇧Space"
            );
            assert_eq!(Kbd::format(&Keystroke::parse("cmd-ctrl-a").unwrap()), "⌃⌘A");
            assert_eq!(
                Kbd::format(&Keystroke::parse("cmd-alt-backspace").unwrap()),
                "⌥⌘⌫"
            );
            assert_eq!(
                Kbd::format(&Keystroke::parse("shift-delete").unwrap()),
                "⇧⌫"
            );
            assert_eq!(
                Kbd::format(&Keystroke::parse("cmd-ctrl-shift-a").unwrap()),
                "⌃⇧⌘A"
            );
            assert_eq!(
                Kbd::format(&Keystroke::parse("cmd-ctrl-shift-alt-a").unwrap()),
                "⌃⌥⇧⌘A"
            );
        } else {
            assert_eq!(Kbd::format(&Keystroke::parse("a").unwrap()), "A");
            assert_eq!(Kbd::format(&Keystroke::parse("ctrl-a").unwrap()), "Ctrl+A");
            assert_eq!(
                Kbd::format(&Keystroke::parse("shift-space").unwrap()),
                "Shift+Space"
            );
            assert_eq!(
                Kbd::format(&Keystroke::parse("ctrl-alt-a").unwrap()),
                "Ctrl+Alt+A"
            );
            assert_eq!(
                Kbd::format(&Keystroke::parse("ctrl-alt-shift-a").unwrap()),
                "Ctrl+Alt+Shift+A"
            );
            assert_eq!(
                Kbd::format(&Keystroke::parse("ctrl-alt-shift-win-a").unwrap()),
                "Ctrl+Alt+Shift+Win+A"
            );
            assert_eq!(
                Kbd::format(&Keystroke::parse("ctrl-shift-backspace").unwrap()),
                "Ctrl+Shift+Backspace"
            );
            assert_eq!(
                Kbd::format(&Keystroke::parse("alt-delete").unwrap()),
                "Alt+Delete"
            );
            assert_eq!(
                Kbd::format(&Keystroke::parse("alt-tab").unwrap()),
                "Alt+Tab"
            );
        }
    }
}
