use gpui::{
    div, rems, App, HighlightStyle, IntoElement, ParentElement, RenderOnce, SharedString,
    StyleRefinement, Styled, StyledText, Window,
};

use crate::{ActiveTheme, StyledExt};

const MASKED: &'static str = "•";

#[derive(IntoElement)]
pub struct Label {
    style: StyleRefinement,
    label: SharedString,
    secondary: Option<SharedString>,
    masked: bool,
}

impl Label {
    pub fn new(label: impl Into<SharedString>) -> Self {
        let label: SharedString = label.into();
        Self {
            style: Default::default(),
            label,
            secondary: None,
            masked: false,
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
}

impl Styled for Label {
    fn style(&mut self) -> &mut gpui::StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for Label {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let mut text = match &self.secondary {
            Some(secondary) => format!("{} {}", self.label, secondary).into(),
            None => self.label.clone(),
        };
        let chars_count = text.chars().count();
        if self.masked {
            text = SharedString::from(MASKED.repeat(chars_count))
        };

        let mut highlights = vec![(0..self.label.len(), HighlightStyle::default())];
        if self.secondary.is_some() {
            highlights.push((
                self.label.len()..text.len(),
                HighlightStyle {
                    color: Some(cx.theme().muted_foreground),
                    ..Default::default()
                },
            ));
        }

        div()
            .line_height(rems(1.25))
            .text_color(cx.theme().foreground)
            .refine_style(&self.style)
            .child(StyledText::new(&text).with_highlights(highlights))
    }
}
