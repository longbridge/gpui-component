use gpui::{
    div, relative, Action, App, IntoElement, KeyContext, Keystroke, ParentElement as _, RenderOnce,
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
    pub fn format(key: &Keystroke) -> String {
        if cfg!(target_os = "macos") {
            return format!("{}", key);
        }

        let mut parts = vec![];
        if key.modifiers.control {
            parts.push("Ctrl");
        }
        if key.modifiers.alt {
            parts.push("Alt");
        }
        if key.modifiers.platform {
            parts.push("Win");
        }
        if key.modifiers.shift {
            parts.push("Shift");
        }

        // Capitalize the first letter
        let key = if let Some(first_c) = key.key.chars().next() {
            format!("{}{}", first_c.to_uppercase(), &key.key[1..])
        } else {
            key.key.to_string()
        };

        parts.push(&key);
        parts.join("+")
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

        if cfg!(target_os = "windows") {
            assert_eq!(Kbd::format(&Keystroke::parse("a").unwrap()), "A");
            assert_eq!(Kbd::format(&Keystroke::parse("ctrl-a").unwrap()), "Ctrl+A");
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
                "Ctrl+Alt+Win+Shift+A"
            );
            assert_eq!(
                Kbd::format(&Keystroke::parse("ctrl-shift-backspace").unwrap()),
                "Ctrl+Shift+Backspace"
            );
        } else {
            assert_eq!(Kbd::format(&Keystroke::parse("cmd-a").unwrap()), "⌘A");
            assert_eq!(Kbd::format(&Keystroke::parse("cmd-ctrl-a").unwrap()), "^⌘A");
            assert_eq!(
                Kbd::format(&Keystroke::parse("cmd-ctrl-shift-a").unwrap()),
                "^⌘⇧A"
            );
            assert_eq!(
                Kbd::format(&Keystroke::parse("cmd-ctrl-shift-alt-a").unwrap()),
                "^⌥⌘⇧A"
            );
        }
    }
}
