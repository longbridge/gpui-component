use std::fmt::Debug;

use gpui::{
    point, px, App, Bounds, FontWeight, Hsla, Pixels, Point, SharedString, TextAlign, TextRun,
    Window,
};

use super::origin_point;

pub const TEXT_SIZE: f32 = 10.;
pub const TEXT_GAP: f32 = 2.;
pub const TEXT_HEIGHT: f32 = TEXT_SIZE + TEXT_GAP;

/// Returns the rendered width of `text` at `font_size` using the window's
/// current text style.  Used for layout calculations that need to reserve
/// space for labels before the scale ranges are fixed.
pub fn measure_text_width(text: &SharedString, font_size: Pixels, window: &mut Window) -> f32 {
    if text.is_empty() {
        return 0.;
    }
    let text_run = TextRun {
        len: text.len(),
        font: window.text_style().font(),
        color: Hsla {
            h: 0.,
            s: 0.,
            l: 0.,
            a: 1.,
        },
        background_color: None,
        underline: None,
        strikethrough: None,
    };
    window
        .text_system()
        .shape_line(text.clone(), font_size, &[text_run], None)
        .width()
        .as_f32()
}

pub struct Text {
    pub text: SharedString,
    pub origin: Point<Pixels>,
    pub color: Hsla,
    pub font_size: Pixels,
    pub font_weight: FontWeight,
    pub align: TextAlign,
}

impl Text {
    pub fn new<T>(text: impl Into<SharedString>, origin: Point<T>, color: Hsla) -> Self
    where
        T: Default + Clone + Copy + Debug + PartialEq + Into<Pixels>,
    {
        let origin = point(origin.x.into(), origin.y.into());

        Self {
            text: text.into(),
            origin,
            color,
            font_size: TEXT_SIZE.into(),
            font_weight: FontWeight::NORMAL,
            align: TextAlign::Left,
        }
    }

    /// Set the font size of the Text.
    pub fn font_size(mut self, font_size: impl Into<Pixels>) -> Self {
        self.font_size = font_size.into();
        self
    }

    /// Set the font weight of the Text.
    pub fn font_weight(mut self, font_weight: FontWeight) -> Self {
        self.font_weight = font_weight;
        self
    }

    /// Set the alignment of the Text.
    pub fn align(mut self, align: TextAlign) -> Self {
        self.align = align;
        self
    }
}

impl<I> From<I> for PlotLabel
where
    I: Iterator<Item = Text>,
{
    fn from(items: I) -> Self {
        Self::new(items.collect())
    }
}

#[derive(Default)]
pub struct PlotLabel(Vec<Text>);

impl PlotLabel {
    pub fn new(items: Vec<Text>) -> Self {
        Self(items)
    }

    /// Paint the Label.
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
