use gpui::{
    div, px, App, ElementId, IntoElement, ParentElement as _, RenderOnce, SharedString, Styled as _,
};
use gpui_component::clipboard::Clipboard;
use gpui_component::{h_flex, ActiveTheme as _};

#[derive(IntoElement)]
pub struct ColorTextLabel {
    copy_id: ElementId,
    text: SharedString,
    width_px: Option<f32>,
    show_icon: bool,
}

impl ColorTextLabel {
    pub fn new(copy_id: impl Into<ElementId>, text: SharedString) -> Self {
        Self {
            copy_id: copy_id.into(),
            text,
            width_px: None,
            show_icon: true,
        }
    }

    pub fn width_px(mut self, width_px: f32) -> Self {
        self.width_px = Some(width_px);
        self
    }

    pub fn show_icon(mut self, show_icon: bool) -> Self {
        self.show_icon = show_icon;
        self
    }
}

impl RenderOnce for ColorTextLabel {
    fn render(self, _window: &mut gpui::Window, cx: &mut App) -> impl IntoElement {
        let mut row = h_flex()
            .items_center()
            .justify_center()
            .gap_1()
            .font_family(cx.theme().mono_font_family.clone())
            .text_size(px(10.0))
            .text_color(cx.theme().muted_foreground)
            .child(div().child(self.text.clone()));

        if self.show_icon {
            row = row.child(Clipboard::new(self.copy_id).value(self.text));
        }

        if let Some(width_px) = self.width_px {
            row = row.w(px(width_px));
        } else {
            row = row.w_full();
        }

        row
    }
}
