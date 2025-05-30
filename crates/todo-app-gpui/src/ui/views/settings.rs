use gpui::*;

pub struct Settings;

impl Settings {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        Self 
    }

    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }
}

impl Render for Settings {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div().p_4().size_full().child("")
    }
}
