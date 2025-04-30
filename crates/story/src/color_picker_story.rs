use gpui::{
    prelude::FluentBuilder as _, App, AppContext, Context, Entity, Focusable, Hsla, IntoElement,
    ParentElement as _, Render, Styled as _, Subscription, Window,
};
use gpui_component::{
    blue_500,
    color_picker::{ColorPicker, ColorPickerEvent},
    green_500, red_500, v_flex, yellow_500, Colorize,
};

use crate::section;

pub struct ColorPickerStory {
    color_picker: Entity<ColorPicker>,
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

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render + Focusable> {
        Self::view(window, cx)
    }
}

impl ColorPickerStory {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let color_picker = cx.new(|cx| {
            ColorPicker::new("1", window, cx)
                .default_value(red_500())
                .featured_colors(vec![red_500(), blue_500(), green_500(), yellow_500()])
        });

        let _subscriptions = vec![cx.subscribe(&color_picker, |this, _, ev, _| match ev {
            ColorPickerEvent::Change(color) => {
                this.selected_color = *color;
                println!("Color changed to: {:?}", color);
            }
        })];

        Self {
            color_picker,
            selected_color: Some(red_500()),
            _subscriptions,
        }
    }
}

impl Focusable for ColorPickerStory {
    fn focus_handle(&self, cx: &gpui::App) -> gpui::FocusHandle {
        self.color_picker.focus_handle(cx)
    }
}

impl Render for ColorPickerStory {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        v_flex().gap_3().child(
            section("Normal")
                .max_w_md()
                .child(self.color_picker.clone())
                .when_some(self.selected_color, |this, color| {
                    this.child(color.to_hex())
                }),
        )
    }
}
