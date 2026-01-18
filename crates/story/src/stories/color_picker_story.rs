use gpui::{
    prelude::FluentBuilder as _, App, AppContext, Context, Entity, Focusable, Hsla, IntoElement,
    ParentElement as _, Render, Styled as _, Subscription, Window,
};
use gpui_component::{
    color_picker::{ColorPicker, ColorPickerEvent, ColorPickerState},
    v_flex, ActiveTheme as _, Colorize, Sizable,
};

use crate::section;

pub struct ColorPickerStory {
    color: Entity<ColorPickerState>,
    selected_color: Option<Hsla>,
    _subscriptions: Vec<Subscription>,
}

impl super::Story for ColorPickerStory {
    fn title() -> &'static str {
        "ColorPicker"
    }

    fn description() -> &'static str {
        "A color picker to select color."
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }
}

impl ColorPickerStory {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let color =
            cx.new(|cx| ColorPickerState::new(window, cx).default_value(cx.theme().primary));

        let _subscriptions = vec![cx.subscribe(&color, |this, _, ev, _| match ev {
            ColorPickerEvent::Change(color) => {
                this.selected_color = *color;
                println!("Color changed to: {:?}", color);
            }
        })];

        Self {
            color,
            selected_color: Some(cx.theme().primary),
            _subscriptions,
        }
    }
}

impl Focusable for ColorPickerStory {
    fn focus_handle(&self, cx: &gpui::App) -> gpui::FocusHandle {
        self.color.read(cx).focus_handle(cx)
    }
}

impl Render for ColorPickerStory {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        v_flex().gap_3().child(
            section("Normal")
                .max_w_md()
                .child(ColorPicker::new(&self.color).small())
                .when_some(self.selected_color, |this, color| {
                    this.child(color.to_hex())
                }),
        )
    }
}
