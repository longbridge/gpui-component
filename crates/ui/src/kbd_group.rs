use gpui::{App, IntoElement, ParentElement, RenderOnce, StyleRefinement, Styled, Window, div};

use crate::{ActiveTheme, StyledExt, h_flex, kbd::Kbd};

/// A group of `Kbd` elements, rendered with a "+" separator between them.
#[derive(IntoElement)]
pub struct KbdGroup {
    style: StyleRefinement,
    children: Vec<Kbd>,
}

impl KbdGroup {
    /// Create a new empty `KbdGroup`.
    pub fn new() -> Self {
        Self {
            style: StyleRefinement::default(),
            children: Vec::new(),
        }
    }

    /// Add a `Kbd` child to the group.
    pub fn child(mut self, kbd: impl Into<Kbd>) -> Self {
        self.children.push(kbd.into());
        self
    }
}

impl ParentElement for KbdGroup {
    fn extend(&mut self, elements: impl IntoIterator<Item = gpui::AnyElement>) {
        // Not used; children are added via `.child()`.
        let _ = elements;
    }
}

impl Styled for KbdGroup {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for KbdGroup {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let mut group = h_flex().gap_1().items_center().refine_style(&self.style);

        let len = self.children.len();
        for (i, kbd) in self.children.into_iter().enumerate() {
            group = group.child(kbd);
            if i + 1 < len {
                group = group.child(div().text_color(cx.theme().muted_foreground).child("+"));
            }
        }
        group
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::TestAppContext;

    #[gpui::test]
    fn test_kbd_group_single(cx: &mut TestAppContext) {
        cx.update(|_| {
            let group = KbdGroup::new().child(Kbd::new(gpui::Keystroke::parse("cmd-c").unwrap()));
            let _ = group.into_any_element();
        });
    }

    #[gpui::test]
    fn test_kbd_group_multiple(cx: &mut TestAppContext) {
        cx.update(|_| {
            let group = KbdGroup::new()
                .child(Kbd::new(gpui::Keystroke::parse("cmd-c").unwrap()))
                .child(Kbd::new(gpui::Keystroke::parse("ctrl-v").unwrap()));
            let _ = group.into_any_element();
        });
    }
}
