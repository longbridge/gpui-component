//! Tab stops for shaped lines.
//!
//! The platform shapers do not agree on what a `\t` is: DirectWrite gives it
//! no width at all, CoreText uses its own default stops. Neither knows the
//! editor's tab size. So a line is shaped as is and its tabs are then widened
//! to the next tab stop, by moving every glyph after a tab to the right. The
//! caret, hit testing and selection all read glyph positions, so they follow.

use std::sync::Arc;

use gpui::{Font, LineLayout, Pixels, ShapedGlyph, ShapedLine, ShapedRun, WindowTextSystem, px};

/// The distance between two tab stops: `tab_size` spaces of `font`.
pub(crate) fn tab_stop_width(
    text_system: &WindowTextSystem,
    font: &Font,
    font_size: Pixels,
    tab_size: usize,
) -> Pixels {
    let font_id = text_system.resolve_font(font);
    text_system.layout_width(font_id, font_size, ' ') * tab_size as f32
}

/// Widens each tab of `line` to end on the next multiple of `stop`.
///
/// Stops are counted from the start of `line`, so a soft-wrapped row starts
/// its own stops, the same way the wrapper measures it.
pub(crate) fn expand_tabs(line: &mut ShapedLine, stop: Pixels) {
    if let Some(layout) = expanded_layout(line, &line.text, stop) {
        **line = Arc::new(layout);
    }
}

/// The width of `layout` (shaped from `text`) once its tabs are expanded.
pub(crate) fn expanded_width(layout: &LineLayout, text: &str, stop: Pixels) -> Pixels {
    expanded_layout(layout, text, stop).map_or(layout.width, |layout| layout.width)
}

/// `layout` with its tabs widened, or `None` when it has none to widen.
fn expanded_layout(layout: &LineLayout, text: &str, stop: Pixels) -> Option<LineLayout> {
    if stop <= px(0.) || !text.contains('\t') {
        return None;
    }

    // The total shift of everything after each tab, in text order.
    let mut shifts: Vec<(usize, Pixels)> = Vec::new();
    let mut shift = px(0.);
    for (ix, _) in text.match_indices('\t') {
        let start = layout.x_for_index(ix) + shift;
        let end = layout.x_for_index(ix + 1) + shift;
        let next_stop = stop * ((start / stop).floor() + 1.);
        shift += next_stop - end;
        shifts.push((ix, shift));
    }

    let shift_after = |index: usize| {
        let tabs_before = shifts.partition_point(|&(tab, _)| tab < index);
        tabs_before
            .checked_sub(1)
            .map_or(px(0.), |last| shifts[last].1)
    };

    let runs = layout
        .runs
        .iter()
        .map(|run| ShapedRun {
            font_id: run.font_id,
            glyphs: run
                .glyphs
                .iter()
                .map(|glyph| ShapedGlyph {
                    position: gpui::point(
                        glyph.position.x + shift_after(glyph.index),
                        glyph.position.y,
                    ),
                    ..glyph.clone()
                })
                .collect(),
        })
        .collect();

    Some(LineLayout {
        font_size: layout.font_size,
        width: layout.width + shift,
        ascent: layout.ascent,
        descent: layout.descent,
        runs,
        len: layout.len,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{FontId, GlyphId, point};

    /// A line where every byte is one glyph `advance` wide, except tabs,
    /// which are `tab_advance` wide.
    fn layout(text: &str, advance: f32, tab_advance: f32) -> LineLayout {
        let mut x = 0.;
        let glyphs = text
            .char_indices()
            .map(|(index, c)| {
                let glyph = ShapedGlyph {
                    id: GlyphId(0),
                    position: point(px(x), px(0.)),
                    index,
                    is_emoji: false,
                };
                x += if c == '\t' { tab_advance } else { advance };
                glyph
            })
            .collect();
        LineLayout {
            font_size: px(10.),
            width: px(x),
            ascent: px(8.),
            descent: px(2.),
            runs: vec![ShapedRun {
                font_id: FontId(0),
                glyphs,
            }],
            len: text.len(),
        }
    }

    fn xs(layout: &LineLayout) -> Vec<f32> {
        layout.runs[0]
            .glyphs
            .iter()
            .map(|glyph| f32::from(glyph.position.x))
            .collect()
    }

    #[test]
    fn leaves_lines_without_tabs_alone() {
        assert!(expanded_layout(&layout("abc", 10., 0.), "abc", px(40.)).is_none());
    }

    #[test]
    fn zero_width_tabs_reach_the_next_stop() {
        let text = "\t\tx";
        let expanded = expanded_layout(&layout(text, 10., 0.), text, px(40.)).unwrap();
        assert_eq!(xs(&expanded), vec![0., 40., 80.]);
        assert_eq!(expanded.width, px(90.));
    }

    #[test]
    fn a_tab_after_text_only_fills_up_to_the_stop() {
        let text = "ab\tc";
        let expanded = expanded_layout(&layout(text, 10., 0.), text, px(40.)).unwrap();
        assert_eq!(xs(&expanded), vec![0., 10., 20., 40.]);
    }

    #[test]
    fn a_tab_on_a_stop_takes_a_whole_stop() {
        let text = "abcd\te";
        let expanded = expanded_layout(&layout(text, 10., 0.), text, px(40.)).unwrap();
        assert_eq!(xs(&expanded)[5], 80.);
    }

    #[test]
    fn shaper_widths_for_tabs_are_replaced() {
        let text = "\tx";
        let expanded = expanded_layout(&layout(text, 10., 28.), text, px(40.)).unwrap();
        assert_eq!(xs(&expanded), vec![0., 40.]);
        assert_eq!(expanded.width, px(50.));
    }

    #[test]
    fn a_trailing_tab_widens_the_line() {
        let text = "a\t";
        assert_eq!(
            expanded_width(&layout(text, 10., 0.), text, px(40.)),
            px(40.)
        );
    }
}
