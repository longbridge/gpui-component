use std::{ops::Range, rc::Rc};

use gpui::{
    fill, point, px, relative, size, App, Bounds, Corners, Element, ElementId, ElementInputHandler,
    Entity, GlobalElementId, HighlightStyle, IntoElement, LayoutId, MouseButton, MouseMoveEvent,
    Path, Pixels, Point, SharedString, Size, Style, TextAlign, TextRun, UnderlineStyle, Window,
    WrappedLine,
};
use rope::Rope;
use smallvec::SmallVec;

use crate::{
    input::{blink_cursor::CURSOR_WIDTH, RopeExt as _},
    ActiveTheme as _, Root,
};

use super::{mode::InputMode, InputState, LastLayout};

pub(super) const RIGHT_MARGIN: Pixels = px(10.);
const BOTTOM_MARGIN_ROWS: usize = 1;
pub(super) const LINE_NUMBER_RIGHT_MARGIN: Pixels = px(10.);

pub(super) struct TextElement {
    state: Entity<InputState>,
    placeholder: SharedString,
}

impl TextElement {
    pub(super) fn new(state: Entity<InputState>) -> Self {
        Self {
            state,
            placeholder: SharedString::default(),
        }
    }

    /// Set the placeholder text of the input field.
    pub fn placeholder(mut self, placeholder: impl Into<SharedString>) -> Self {
        self.placeholder = placeholder.into();
        self
    }

    fn paint_mouse_listeners(&mut self, window: &mut Window, _: &mut App) {
        window.on_mouse_event({
            let state = self.state.clone();

            move |event: &MouseMoveEvent, _, window, cx| {
                if event.pressed_button == Some(MouseButton::Left) {
                    state.update(cx, |state, cx| {
                        state.on_drag_move(event, window, cx);
                    });
                }
            }
        });
    }

    /// Returns the:
    ///
    /// - cursor bounds
    /// - scroll offset
    /// - current row index (No only the visible lines, but all lines)
    ///
    /// This method also will update for track scroll to cursor.
    fn layout_cursor(
        &self,
        last_layout: &LastLayout,
        bounds: &mut Bounds<Pixels>,
        _: &mut Window,
        cx: &mut App,
    ) -> (Option<Bounds<Pixels>>, Point<Pixels>, Option<usize>) {
        let state = self.state.read(cx);

        let line_height = last_layout.line_height;
        let visible_range = &last_layout.visible_range;
        let lines = &last_layout.lines;
        let text_wrapper = &state.text_wrapper;
        let line_number_width = last_layout.line_number_width;

        let mut selected_range = state.selected_range;
        if let Some(marked_range) = &state.marked_range {
            selected_range = (marked_range.end..marked_range.end).into();
        }

        let cursor = state.cursor();
        let mut current_row = None;
        let mut scroll_offset = state.scroll_handle.offset();
        let mut cursor_bounds = None;

        // If the input has a fixed height (Otherwise is auto-grow), we need to add a bottom margin to the input.
        let bottom_margin = if state.mode.is_auto_grow() {
            px(0.) + line_height
        } else {
            BOTTOM_MARGIN_ROWS * line_height + line_height
        };
        // The cursor corresponds to the current cursor position in the text no only the line.
        let mut cursor_pos = None;
        let mut cursor_start = None;
        let mut cursor_end = None;

        let mut prev_lines_offset = 0;
        let mut offset_y = px(0.);
        for (ix, wrap_line) in text_wrapper.lines.iter().enumerate() {
            let row = ix;
            let line_origin = point(px(0.), offset_y);

            // break loop if all cursor positions are found
            if cursor_pos.is_some() && cursor_start.is_some() && cursor_end.is_some() {
                break;
            }

            let in_visible_range = ix >= visible_range.start;
            if let Some(line) = in_visible_range
                .then(|| lines.get(ix.saturating_sub(visible_range.start)))
                .flatten()
            {
                // If in visible range lines
                if cursor_pos.is_none() {
                    let offset = cursor.saturating_sub(prev_lines_offset);
                    if let Some(pos) = line.position_for_index(offset, line_height) {
                        current_row = Some(row);
                        cursor_pos = Some(line_origin + pos);
                    }
                }
                if cursor_start.is_none() {
                    let offset = selected_range.start.saturating_sub(prev_lines_offset);
                    if let Some(pos) = line.position_for_index(offset, line_height) {
                        cursor_start = Some(line_origin + pos);
                    }
                }
                if cursor_end.is_none() {
                    let offset = selected_range.end.saturating_sub(prev_lines_offset);
                    if let Some(pos) = line.position_for_index(offset, line_height) {
                        cursor_end = Some(line_origin + pos);
                    }
                }

                offset_y += line.size(line_height).height;
                // +1 for the last `\n`
                prev_lines_offset += line.len() + 1;
            } else {
                // If not in the visible range.

                // Just increase the offset_y and prev_lines_offset.
                // This will let the scroll_offset to track the cursor position correctly.
                if prev_lines_offset >= cursor && cursor_pos.is_none() {
                    current_row = Some(row);
                    cursor_pos = Some(line_origin);
                }
                if prev_lines_offset >= selected_range.start && cursor_start.is_none() {
                    cursor_start = Some(line_origin);
                }
                if prev_lines_offset >= selected_range.end && cursor_end.is_none() {
                    cursor_end = Some(line_origin);
                }

                offset_y += wrap_line.height(line_height);
                // +1 for the last `\n`
                prev_lines_offset += wrap_line.len() + 1;
            }
        }

        if let (Some(cursor_pos), Some(cursor_start), Some(cursor_end)) =
            (cursor_pos, cursor_start, cursor_end)
        {
            let cursor_moved = state.last_cursor != Some(cursor);
            let selection_changed = state.last_selected_range != Some(selected_range);

            if cursor_moved || selection_changed {
                scroll_offset.x = if scroll_offset.x + cursor_pos.x
                    > (bounds.size.width - line_number_width - RIGHT_MARGIN)
                {
                    // cursor is out of right
                    bounds.size.width - line_number_width - RIGHT_MARGIN - cursor_pos.x
                } else if scroll_offset.x + cursor_pos.x < px(0.) {
                    // cursor is out of left
                    scroll_offset.x - cursor_pos.x
                } else {
                    scroll_offset.x
                };
                scroll_offset.y = if scroll_offset.y + cursor_pos.y + line_height
                    > bounds.size.height - bottom_margin
                {
                    // cursor is out of bottom
                    bounds.size.height - bottom_margin - cursor_pos.y
                } else if scroll_offset.y + cursor_pos.y < px(0.) {
                    // cursor is out of top
                    scroll_offset.y - cursor_pos.y
                } else {
                    scroll_offset.y
                };

                if state.selection_reversed {
                    if scroll_offset.x + cursor_start.x < px(0.) {
                        // selection start is out of left
                        scroll_offset.x = -cursor_start.x;
                    }
                    if scroll_offset.y + cursor_start.y < px(0.) {
                        // selection start is out of top
                        scroll_offset.y = -cursor_start.y;
                    }
                } else {
                    if scroll_offset.x + cursor_end.x <= px(0.) {
                        // selection end is out of left
                        scroll_offset.x = -cursor_end.x;
                    }
                    if scroll_offset.y + cursor_end.y <= px(0.) {
                        // selection end is out of top
                        scroll_offset.y = -cursor_end.y;
                    }
                }
            }

            // cursor bounds
            let cursor_height = line_height;
            cursor_bounds = Some(Bounds::new(
                point(
                    bounds.left() + cursor_pos.x + line_number_width + scroll_offset.x,
                    bounds.top() + cursor_pos.y + ((line_height - cursor_height) / 2.),
                ),
                size(CURSOR_WIDTH, cursor_height),
            ));
        }

        bounds.origin = bounds.origin + scroll_offset;

        (cursor_bounds, scroll_offset, current_row)
    }

    fn layout_selections(
        &self,
        last_layout: &LastLayout,
        bounds: &mut Bounds<Pixels>,
        _: &mut Window,
        cx: &mut App,
    ) -> Option<Path<Pixels>> {
        let line_height = last_layout.line_height;
        let visible_top = last_layout.visible_top;
        let visible_start_offset = last_layout.visible_start_offset;
        let lines = &last_layout.lines;
        let line_number_width = last_layout.line_number_width;

        let state = self.state.read(cx);
        let mut selected_range = state.selected_range;
        if let Some(marked_range) = &state.marked_range {
            if !marked_range.is_empty() {
                selected_range = (marked_range.end..marked_range.end).into();
            }
        }
        if selected_range.is_empty() {
            return None;
        }

        let (start_ix, end_ix) = if selected_range.start < selected_range.end {
            (selected_range.start, selected_range.end)
        } else {
            (selected_range.end, selected_range.start)
        };

        let mut prev_lines_offset = visible_start_offset;
        let mut offset_y = visible_top;
        let mut line_corners = vec![];

        for line in lines.iter() {
            let line_size = line.size(line_height);
            let line_wrap_width = line_size.width;

            let line_origin = point(px(0.), offset_y);

            let line_cursor_start =
                line.position_for_index(start_ix.saturating_sub(prev_lines_offset), line_height);
            let line_cursor_end =
                line.position_for_index(end_ix.saturating_sub(prev_lines_offset), line_height);

            if line_cursor_start.is_some() || line_cursor_end.is_some() {
                let start = line_cursor_start
                    .unwrap_or_else(|| line.position_for_index(0, line_height).unwrap());

                let end = line_cursor_end
                    .unwrap_or_else(|| line.position_for_index(line.len(), line_height).unwrap());

                // Split the selection into multiple items
                let wrapped_lines =
                    (end.y / line_height).ceil() as usize - (start.y / line_height).ceil() as usize;

                let mut end_x = end.x;
                if wrapped_lines > 0 {
                    end_x = line_wrap_width;
                }

                // Ensure at least 6px width for the selection for empty lines.
                end_x = end_x.max(start.x + px(6.));

                line_corners.push(Corners {
                    top_left: line_origin + point(start.x, start.y),
                    top_right: line_origin + point(end_x, start.y),
                    bottom_left: line_origin + point(start.x, start.y + line_height),
                    bottom_right: line_origin + point(end_x, start.y + line_height),
                });

                // wrapped lines
                for i in 1..=wrapped_lines {
                    let start = point(px(0.), start.y + i as f32 * line_height);
                    let mut end = point(end.x, end.y + i as f32 * line_height);
                    if i < wrapped_lines {
                        end.x = line_size.width;
                    }

                    line_corners.push(Corners {
                        top_left: line_origin + point(start.x, start.y),
                        top_right: line_origin + point(end.x, start.y),
                        bottom_left: line_origin + point(start.x, start.y + line_height),
                        bottom_right: line_origin + point(end.x, start.y + line_height),
                    });
                }
            }

            if line_cursor_start.is_some() && line_cursor_end.is_some() {
                break;
            }

            offset_y += line_size.height;
            // +1 for skip the last `\n`
            prev_lines_offset += line.len() + 1;
        }

        let mut points = vec![];
        if line_corners.is_empty() {
            return None;
        }

        // Fix corners to make sure the left to right direction
        for corners in &mut line_corners {
            if corners.top_left.x > corners.top_right.x {
                std::mem::swap(&mut corners.top_left, &mut corners.top_right);
                std::mem::swap(&mut corners.bottom_left, &mut corners.bottom_right);
            }
        }

        for corners in &line_corners {
            points.push(corners.top_right);
            points.push(corners.bottom_right);
            points.push(corners.bottom_left);
        }

        let mut rev_line_corners = line_corners.iter().rev().peekable();
        while let Some(corners) = rev_line_corners.next() {
            points.push(corners.top_left);
            if let Some(next) = rev_line_corners.peek() {
                if next.top_left.x > corners.top_left.x {
                    points.push(point(next.top_left.x, corners.top_left.y));
                }
            }
        }

        // print_points_as_svg_path(&line_corners, &points);

        let path_origin = bounds.origin + point(line_number_width, px(0.));
        let first_p = *points.get(0).unwrap();
        let mut builder = gpui::PathBuilder::fill();
        builder.move_to(path_origin + first_p);
        for p in points.iter().skip(1) {
            builder.line_to(path_origin + *p);
        }

        builder.build().ok()
    }

    /// Calculate the visible range of lines in the viewport.
    ///
    /// Returns
    ///
    /// - visible_range: The visible range is based on unwrapped lines (Zero based).
    /// - visible_top: The top position of the first visible line in the scroll viewport.
    fn calculate_visible_range(
        &self,
        state: &InputState,
        line_height: Pixels,
        input_height: Pixels,
    ) -> (Range<usize>, Pixels) {
        // Add extra rows to avoid showing empty space when scroll to bottom.
        let extra_rows = 1;
        let mut visible_top = px(0.);
        if state.mode.is_single_line() {
            return (0..1, visible_top);
        }

        let total_lines = state.text_wrapper.len();
        let scroll_top = state.scroll_handle.offset().y;

        let mut visible_range = 0..total_lines;
        let mut line_bottom = px(0.);
        for (ix, line) in state.text_wrapper.lines.iter().enumerate() {
            let wrapped_height = line.height(line_height);
            line_bottom += wrapped_height;

            if line_bottom < -scroll_top {
                visible_top = line_bottom - wrapped_height;
                visible_range.start = ix;
            }

            if line_bottom + scroll_top >= input_height {
                visible_range.end = (ix + extra_rows).min(total_lines);
                break;
            }
        }

        (visible_range, visible_top)
    }

    /// First usize is the offset of skipped.
    fn highlight_lines(
        &mut self,
        visible_range: &Range<usize>,
        _visible_top: Pixels,
        visible_byte_range: Range<usize>,
        cx: &mut App,
    ) -> Option<Vec<(Range<usize>, HighlightStyle)>> {
        let state = self.state.read(cx);
        let text = &state.text;

        let (highlighter, diagnostics) = match &state.mode {
            InputMode::CodeEditor {
                highlighter,
                diagnostics,
                ..
            } => (highlighter.borrow(), diagnostics),
            _ => return None,
        };
        let highlighter = highlighter.as_ref()?;

        let mut offset = visible_byte_range.start;
        let mut styles = vec![];

        for line in text
            .lines()
            .skip(visible_range.start)
            .take(visible_range.len())
        {
            // +1 for `\n`
            let line_len = line.len() + 1;
            let range = offset..offset + line_len;
            let line_styles = highlighter.styles(&range, cx);
            styles = gpui::combine_highlights(styles, line_styles).collect();

            offset = range.end;
        }

        let diagnostic_styles = diagnostics.styles_for_range(&visible_byte_range, cx);

        // Combine marker styles
        styles = gpui::combine_highlights(diagnostic_styles, styles).collect();

        Some(styles)
    }
}

pub(super) struct PrepaintState {
    /// The lines of entire lines.
    last_layout: LastLayout,
    /// The lines only contains the visible lines in the viewport, based on `visible_range`.
    line_numbers: Option<Vec<SmallVec<[WrappedLine; 1]>>>,
    /// Size of the scrollable area by entire lines.
    scroll_size: Size<Pixels>,
    cursor_bounds: Option<Bounds<Pixels>>,
    cursor_scroll_offset: Point<Pixels>,
    /// row index (zero based), no wrap, same line as the cursor.
    current_row: Option<usize>,
    selection_path: Option<Path<Pixels>>,
    bounds: Bounds<Pixels>,
}

impl IntoElement for TextElement {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

/// A debug function to print points as SVG path.
#[allow(unused)]
fn print_points_as_svg_path(
    line_corners: &Vec<Corners<Point<Pixels>>>,
    points: &Vec<Point<Pixels>>,
) {
    for corners in line_corners {
        println!(
            "tl: ({}, {}), tr: ({}, {}), bl: ({}, {}), br: ({}, {})",
            corners.top_left.x.0 as i32,
            corners.top_left.y.0 as i32,
            corners.top_right.x.0 as i32,
            corners.top_right.y.0 as i32,
            corners.bottom_left.x.0 as i32,
            corners.bottom_left.y.0 as i32,
            corners.bottom_right.x.0 as i32,
            corners.bottom_right.y.0 as i32,
        );
    }

    if points.len() > 0 {
        println!("M{},{}", points[0].x.0 as i32, points[0].y.0 as i32);
        for p in points.iter().skip(1) {
            println!("L{},{}", p.x.0 as i32, p.y.0 as i32);
        }
    }
}

impl Element for TextElement {
    type RequestLayoutState = ();
    type PrepaintState = PrepaintState;

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _id: Option<&GlobalElementId>,
        _: Option<&gpui::InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let state = self.state.read(cx);
        let line_height = window.line_height();

        let mut style = Style::default();
        style.size.width = relative(1.).into();
        if state.mode.is_multi_line() {
            style.flex_grow = 1.0;
            style.size.height = relative(1.).into();
            if state.mode.is_auto_grow() {
                // Auto grow to let height match to rows, but not exceed max rows.
                let rows = state.mode.max_rows().min(state.mode.rows());
                style.min_size.height = (rows * line_height).into();
            } else {
                style.min_size.height = line_height.into();
            }
        } else {
            // For single-line inputs, the minimum height should be the line height
            style.size.height = line_height.into();
        };

        (window.request_layout(style, [], cx), ())
    }

    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _: Option<&gpui::InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        let state = self.state.read(cx);
        let line_height = window.line_height();

        let (visible_range, visible_top) =
            self.calculate_visible_range(&state, line_height, bounds.size.height);
        let visible_start_offset = state.text.line_start_offset(visible_range.start);
        let visible_end_offset = state
            .text
            .line_end_offset(visible_range.end.saturating_sub(1));

        let highlight_styles = self.highlight_lines(
            &visible_range,
            visible_top,
            visible_start_offset..visible_end_offset,
            cx,
        );

        let state = self.state.read(cx);
        let multi_line = state.mode.is_multi_line();
        let text = state.text.clone();
        let is_empty = text.len() == 0;
        let placeholder = self.placeholder.clone();
        let style = window.text_style();
        let font_size = style.font_size.to_pixels(window.rem_size());
        let mut bounds = bounds;

        let (display_text, text_color) = if is_empty {
            (
                Rope::from(placeholder.as_str()),
                cx.theme().muted_foreground,
            )
        } else if state.masked {
            (
                Rope::from("*".repeat(text.chars_count())),
                cx.theme().foreground,
            )
        } else {
            (text.clone(), cx.theme().foreground)
        };

        let text_style = window.text_style();

        // Calculate the width of the line numbers
        let empty_line_number = window.text_system().shape_line(
            "+++++".into(),
            font_size,
            &[TextRun {
                len: 5,
                font: style.font(),
                color: gpui::black(),
                background_color: None,
                underline: None,
                strikethrough: None,
            }],
            None,
        );
        let line_number_width = if state.mode.line_number() {
            empty_line_number.width + px(6.) + LINE_NUMBER_RIGHT_MARGIN
        } else {
            px(0.)
        };

        let run = TextRun {
            len: display_text.len(),
            font: style.font(),
            color: text_color,
            background_color: None,
            underline: None,
            strikethrough: None,
        };
        let marked_run = TextRun {
            len: 0,
            font: style.font(),
            color: text_color,
            background_color: None,
            underline: Some(UnderlineStyle {
                thickness: px(1.),
                color: Some(text_color),
                wavy: false,
            }),
            strikethrough: None,
        };

        let runs = if !is_empty {
            if let Some(highlight_styles) = highlight_styles {
                let mut runs = vec![];

                runs.extend(highlight_styles.iter().map(|(range, style)| {
                    let mut run = text_style.clone().highlight(*style).to_run(range.len());
                    if let Some(marked_range) = &state.marked_range {
                        if range.start >= marked_range.start && range.end <= marked_range.end {
                            run.color = marked_run.color;
                            run.strikethrough = marked_run.strikethrough;
                            run.underline = marked_run.underline;
                        }
                    }

                    run
                }));

                runs.into_iter().filter(|run| run.len > 0).collect()
            } else {
                vec![run]
            }
        } else if let Some(marked_range) = &state.marked_range {
            // IME marked text
            vec![
                TextRun {
                    len: marked_range.start,
                    ..run.clone()
                },
                TextRun {
                    len: marked_range.end - marked_range.start,
                    underline: marked_run.underline,
                    ..run.clone()
                },
                TextRun {
                    len: display_text.len() - marked_range.end,
                    ..run.clone()
                },
            ]
            .into_iter()
            .filter(|run| run.len > 0)
            .collect()
        } else {
            vec![run]
        };

        let wrap_width = if multi_line && state.soft_wrap {
            Some(bounds.size.width - line_number_width)
        } else {
            None
        };

        // NOTE: Here 50 lines about 150µs
        // let measure = crate::Measure::new("shape_text");
        let visible_text = display_text
            .slice_rows(visible_range.start as u32..visible_range.end as u32)
            .to_string();

        let lines = window
            .text_system()
            .shape_text(visible_text.into(), font_size, &runs, wrap_width, None)
            .expect("failed to shape text");
        // measure.end();

        let mut longest_line_width = px(0.);
        if state.mode.is_multi_line() && lines.len() > 1 {
            let longtest_line: SharedString = state
                .text
                .line(state.text.summary().longest_row as usize)
                .to_string()
                .into();
            longest_line_width = window
                .text_system()
                .shape_line(
                    longtest_line.clone(),
                    font_size,
                    &[TextRun {
                        len: longtest_line.len(),
                        font: style.font(),
                        color: gpui::black(),
                        background_color: None,
                        underline: None,
                        strikethrough: None,
                    }],
                    wrap_width,
                )
                .width;
        }

        let total_wrapped_lines = state.text_wrapper.len();

        let scroll_size = size(
            if longest_line_width + line_number_width + RIGHT_MARGIN > bounds.size.width {
                longest_line_width + line_number_width + RIGHT_MARGIN
            } else {
                longest_line_width
            },
            (total_wrapped_lines as f32 * line_height).max(bounds.size.height),
        );

        let mut last_layout = LastLayout {
            visible_range,
            visible_top,
            visible_start_offset,
            line_height,
            wrap_width,
            line_number_width,
            lines: Rc::new(lines),
            cursor_bounds: None,
        };

        // `position_for_index` for example
        //
        // #### text
        //
        // Hello 世界，this is GPUI component.
        // The GPUI Component is a collection of UI components for
        // GPUI framework, including Button, Input, Checkbox, Radio,
        // Dropdown, Tab, and more...
        //
        // wrap_width: 444px, line_height: 20px
        //
        // #### lines[0]
        //
        // | index | pos              | line |
        // |-------|------------------|------|
        // | 5     | (37 px, 0.0)     | 0    |
        // | 38    | (261.7 px, 20.0) | 0    |
        // | 40    | None             | -    |
        //
        // #### lines[1]
        //
        // | index | position              | line |
        // |-------|-----------------------|------|
        // | 5     | (43.578125 px, 0.0)   | 0    |
        // | 56    | (422.21094 px, 0.0)   | 0    |
        // | 57    | (11.6328125 px, 20.0) | 1    |
        // | 114   | (429.85938 px, 20.0)  | 1    |
        // | 115   | (11.3125 px, 40.0)    | 2    |

        // Calculate the scroll offset to keep the cursor in view

        let (cursor_bounds, cursor_scroll_offset, current_row) =
            self.layout_cursor(&last_layout, &mut bounds, window, cx);
        last_layout.cursor_bounds = cursor_bounds;

        let selection_path = self.layout_selections(&last_layout, &mut bounds, window, cx);

        let state = self.state.read(cx);
        let line_numbers = if state.mode.line_number() {
            let mut line_numbers = vec![];
            let run_len = 4;
            let other_line_runs = vec![TextRun {
                len: run_len,
                font: style.font(),
                color: cx.theme().muted_foreground,
                background_color: None,
                underline: None,
                strikethrough: None,
            }];
            let current_line_runs = vec![TextRun {
                len: run_len,
                font: style.font(),
                color: cx.theme().foreground,
                background_color: None,
                underline: None,
                strikethrough: None,
            }];

            // build line numbers
            for (ix, line) in last_layout.lines.iter().enumerate() {
                let ix = last_layout.visible_range.start + ix;
                let line_no = ix + 1;

                let mut line_no_text = format!("{:>5}", line_no);
                if !line.wrap_boundaries.is_empty() {
                    line_no_text.push_str(&"\n    ".repeat(line.wrap_boundaries.len()));
                }

                let runs = if current_row == Some(ix) {
                    &current_line_runs
                } else {
                    &other_line_runs
                };

                let shape_line = window
                    .text_system()
                    .shape_text(line_no_text.into(), font_size, &runs, None, None)
                    .unwrap();
                line_numbers.push(shape_line);
            }
            Some(line_numbers)
        } else {
            None
        };

        PrepaintState {
            bounds,
            last_layout,
            scroll_size,
            line_numbers,
            cursor_bounds,
            cursor_scroll_offset,
            current_row,
            selection_path,
        }
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _: Option<&gpui::InspectorElementId>,
        input_bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        let focus_handle = self.state.read(cx).focus_handle.clone();
        let show_cursor = self.state.read(cx).show_cursor(window, cx);
        let focused = focus_handle.is_focused(window);
        let bounds = prepaint.bounds;
        let selected_range = self.state.read(cx).selected_range;
        let visible_range = &prepaint.last_layout.visible_range;

        window.handle_input(
            &focus_handle,
            ElementInputHandler::new(bounds, self.state.clone()),
            cx,
        );

        // Set Root focused_input when self is focused
        if focused {
            let state = self.state.clone();
            if Root::read(window, cx).focused_input.as_ref() != Some(&state) {
                Root::update(window, cx, |root, _, cx| {
                    root.focused_input = Some(state);
                    cx.notify();
                });
            }
        }

        // And reset focused_input when next_frame start
        window.on_next_frame({
            let state = self.state.clone();
            move |window, cx| {
                if !focused && Root::read(window, cx).focused_input.as_ref() == Some(&state) {
                    Root::update(window, cx, |root, _, cx| {
                        root.focused_input = None;
                        cx.notify();
                    });
                }
            }
        });

        // Paint multi line text
        let line_height = window.line_height();
        let origin = bounds.origin;

        let invisible_top_padding = prepaint.last_layout.visible_top;

        let mut mask_offset_y = px(0.);
        if self.state.read(cx).masked {
            // Move down offset for vertical centering the *****
            if cfg!(target_os = "macos") {
                mask_offset_y = px(3.);
            } else {
                mask_offset_y = px(2.5);
            }
        }

        let active_line_color = cx.theme().highlight_theme.style.active_line;

        // Paint active line
        let mut offset_y = px(0.);
        if let Some(line_numbers) = prepaint.line_numbers.as_ref() {
            offset_y += invisible_top_padding;

            // Each item is the normal lines.
            for (ix, lines) in line_numbers.iter().enumerate() {
                let row = visible_range.start + ix;
                let is_active = prepaint.current_row == Some(row);
                for line in lines {
                    let p = point(input_bounds.origin.x, origin.y + offset_y);
                    let line_size = line.size(line_height);
                    // Paint the current line background
                    if is_active {
                        if let Some(bg_color) = active_line_color {
                            window.paint_quad(fill(
                                Bounds::new(p, size(bounds.size.width, line_height)),
                                bg_color,
                            ));
                        }
                    }
                    offset_y += line_size.height;
                }
            }
        }

        // Paint selections
        if window.is_window_active() {
            if let Some(path) = prepaint.selection_path.take() {
                window.paint_path(path, cx.theme().selection);
            }
        }

        // Paint text
        let mut offset_y = mask_offset_y + invisible_top_padding;
        for line in prepaint.last_layout.lines.iter() {
            let p = point(
                origin.x + prepaint.last_layout.line_number_width,
                origin.y + offset_y,
            );
            _ = line.paint(p, line_height, TextAlign::Left, None, window, cx);
            offset_y += line.size(line_height).height;
        }

        // Paint blinking cursor
        if focused && show_cursor {
            if let Some(mut cursor_bounds) = prepaint.cursor_bounds.take() {
                cursor_bounds.origin.y += prepaint.cursor_scroll_offset.y;
                window.paint_quad(fill(cursor_bounds, cx.theme().caret));
            }
        }

        // Paint line numbers
        let mut offset_y = px(0.);
        if let Some(line_numbers) = prepaint.line_numbers.as_ref() {
            offset_y += invisible_top_padding;

            // Paint line number background
            window.paint_quad(fill(
                Bounds {
                    origin: input_bounds.origin,
                    size: size(
                        prepaint.last_layout.line_number_width - LINE_NUMBER_RIGHT_MARGIN,
                        input_bounds.size.height,
                    ),
                },
                cx.theme()
                    .highlight_theme
                    .style
                    .background
                    .unwrap_or(cx.theme().input),
            ));

            // Each item is the normal lines.
            for (ix, lines) in line_numbers.iter().enumerate() {
                let row = visible_range.start + ix;
                for line in lines {
                    let p = point(input_bounds.origin.x, origin.y + offset_y);

                    let is_active = prepaint.current_row == Some(row);
                    let line_size = line.size(line_height);

                    // paint active line number background
                    if is_active {
                        if let Some(bg_color) = active_line_color {
                            window.paint_quad(fill(
                                Bounds::new(
                                    p,
                                    size(prepaint.last_layout.line_number_width, line_height),
                                ),
                                bg_color,
                            ));
                        }
                    }

                    _ = line.paint(p, line_height, TextAlign::Left, None, window, cx);
                    offset_y += line_size.height;
                }
            }
        }

        self.state.update(cx, |state, cx| {
            state.last_layout = Some(prepaint.last_layout.clone());
            state.last_bounds = Some(bounds);
            state.last_cursor = Some(state.cursor());
            state.set_input_bounds(input_bounds, cx);
            state.last_selected_range = Some(selected_range);
            state.scroll_size = prepaint.scroll_size;
            state
                .scroll_handle
                .set_offset(prepaint.cursor_scroll_offset);
            cx.notify();
        });

        self.paint_mouse_listeners(window, cx);
    }
}
