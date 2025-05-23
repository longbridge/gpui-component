use std::fmt::Debug;

use gpui::{
    point, px, App, Bounds, FontWeight, Hsla, Pixels, Point, SharedString, TextAlign, TextRun,
    Window,
};

use super::origin_point;

pub const TEXT_SIZE: f64 = 10.;
pub const TEXT_GAP: f64 = 2.;
pub const TEXT_HEIGHT: f64 = TEXT_SIZE + TEXT_GAP;

pub struct Text {
    pub text: SharedString,
    pub origin: Point<Pixels>,
    pub color: Hsla,
    pub font_size: Pixels,
    pub font_weight: FontWeight,
    pub align: TextAlign,
}

impl Text {
    pub fn new<T>(text: SharedString, origin: Point<T>, color: Hsla) -> Self
    where
        T: Default + Clone + Copy + Debug + Into<Pixels>,
    {
        let origin = point(origin.x.into(), origin.y.into());

        Self {
            text,
            origin,
            color,
            font_size: TEXT_SIZE.into(),
            font_weight: FontWeight::NORMAL,
            align: TextAlign::Left,
        }
    }

    pub fn font_size(mut self, font_size: impl Into<Pixels>) -> Self {
        self.font_size = font_size.into();
        self
    }

    pub fn font_weight(mut self, font_weight: FontWeight) -> Self {
        self.font_weight = font_weight;
        self
    }

    pub fn align(mut self, align: TextAlign) -> Self {
        self.align = align;
        self
    }
}

impl<I> From<I> for Label
where
    I: Iterator<Item = Text>,
{
    fn from(items: I) -> Self {
        Self::new(items.collect())
    }
}

#[derive(Default)]
pub struct Label(Vec<Text>);

impl Label {
    pub fn new(items: Vec<Text>) -> Self {
        Self(items)
    }

    pub fn paint(&self, bounds: &Bounds<Pixels>, window: &mut Window, cx: &mut App) {
        for Text {
            text,
            origin,
            color,
            font_size,
            font_weight,
            align,
        } in self.0.iter()
        {
            let origin = origin_point(origin.x, origin.y, bounds.origin);

            let text_run = TextRun {
                len: text.len(),
                font: window.text_style().highlight(*font_weight).font(),
                color: *color,
                background_color: None,
                underline: None,
                strikethrough: None,
            };

            if let Ok(text) =
                window
                    .text_system()
                    .shape_text(text.clone(), *font_size, &[text_run], None, None)
            {
                for line in text {
                    let origin = match align {
                        TextAlign::Left => origin,
                        TextAlign::Right => origin - point(line.size(*font_size).width, px(0.)),
                        _ => origin - point(line.size(*font_size).width / 2., px(0.)),
                    };

                    let _ = line.paint(origin, *font_size, *align, None, window, cx);
                }
            }
        }
    }
}
