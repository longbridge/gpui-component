use gpui::{
    fill, point, px, relative, size, App, Bounds, Corners, Element, ElementId, ElementInputHandler,
    Entity, GlobalElementId, IntoElement, LayoutId, MouseButton, MouseMoveEvent, PaintQuad, Path,
    Pixels, Point, Style, TextAlign, TextRun, UnderlineStyle, Window, WrappedLine,
};
use smallvec::SmallVec;

use crate::{ActiveTheme as _, Root};

use super::TextInput;

const RIGHT_MARGIN: Pixels = px(5.);
const BOTTOM_MARGIN: Pixels = px(20.);

pub(super) struct TextElement {
    input: Entity<TextInput>,
}

impl TextElement {
    pub(super) fn new(input: Entity<TextInput>) -> Self {
        Self { input }
    }

    fn paint_mouse_listeners(&mut self, window: &mut Window, _: &mut App) {
        window.on_mouse_event({
            let input = self.input.clone();

            move |event: &MouseMoveEvent, _, window, cx| {
                if event.pressed_button == Some(MouseButton::Left) {
                    input.update(cx, |input, cx| {
                        input.on_drag_move(event, window, cx);
                    });
                }
            }
        });
    }

    fn layout_cursor(
        &self,
        lines: &[WrappedLine],
        line_height: Pixels,
        bounds: &mut Bounds<Pixels>,
        window: &mut Window,
        cx: &mut App,
    ) -> (Option<PaintQuad>, Point<Pixels>) {
        let input = self.input.read(cx);
        let selected_range = &input.selected_range;
        let cursor_offset = input.cursor_offset();
        let mut scroll_offset = input.scroll_handle.offset();
        let mut cursor = None;

        // The cursor corresponds to the current cursor position in the text no only the line.
        let mut cursor_pos = None;
        let mut cursor_start = None;
        let mut cursor_end = None;

        let mut prev_lines_offset = 0;
        let mut offset_y = px(0.);
        for line in lines.iter() {
            // break loop if all cursor positions are found
            if cursor_pos.is_some() && cursor_start.is_some() && cursor_end.is_some() {
                break;
            }

            let line_origin = point(px(0.), offset_y);
            if cursor_pos.is_none() {
                let offset = cursor_offset.saturating_sub(prev_lines_offset);
                if let Some(pos) = line.position_for_index(offset, line_height) {
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
            // +1 for skip the last `\n`
            prev_lines_offset += line.len() + 1;
        }

        if let (Some(cursor_pos), Some(cursor_start), Some(cursor_end)) =
            (cursor_pos, cursor_start, cursor_end)
        {
            let cursor_moved = input.last_cursor_offset != Some(cursor_offset);
            let selection_changed = input.last_selected_range != Some(selected_range.clone());

            if cursor_moved || selection_changed {
                scroll_offset.x =
                    if scroll_offset.x + cursor_pos.x > (bounds.size.width - RIGHT_MARGIN) {
                        // cursor is out of right
                        bounds.size.width - RIGHT_MARGIN - cursor_pos.x
                    } else if scroll_offset.x + cursor_pos.x < px(0.) {
                        // cursor is out of left
                        scroll_offset.x - cursor_pos.x
                    } else {
                        scroll_offset.x
                    };
                scroll_offset.y =
                    if scroll_offset.y + cursor_pos.y > (bounds.size.height - BOTTOM_MARGIN) {
                        // cursor is out of bottom
                        bounds.size.height - BOTTOM_MARGIN - cursor_pos.y
                    } else if scroll_offset.y + cursor_pos.y < px(0.) {
                        // cursor is out of top
                        scroll_offset.y - cursor_pos.y
                    } else {
                        scroll_offset.y
                    };

                if input.selection_reversed {
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

            bounds.origin = bounds.origin + scroll_offset;

            if input.show_cursor(window, cx) {
                // cursor blink
                let cursor_height =
                    window.text_style().font_size.to_pixels(window.rem_size()) + px(2.);
                cursor = Some(fill(
                    Bounds::new(
                        point(
                            bounds.left() + cursor_pos.x,
                            bounds.top() + cursor_pos.y + ((line_height - cursor_height) / 2.),
                        ),
                        size(px(1.), cursor_height),
                    ),
                    cx.theme().caret,
                ))
            };
        }

        (cursor, scroll_offset)
    }

    fn layout_selections(
        &self,
        lines: &[WrappedLine],
        line_height: Pixels,
        bounds: &mut Bounds<Pixels>,
        _: &mut Window,
        cx: &mut App,
    ) -> Option<Path<Pixels>> {
        let input = self.input.read(cx);
        let selected_range = &input.selected_range;
        if selected_range.is_empty() {
            return None;
        }

        let (start_ix, end_ix) = if selected_range.start < selected_range.end {
            (selected_range.start, selected_range.end)
        } else {
            (selected_range.end, selected_range.start)
        };

        let mut prev_lines_offset = 0;
        let mut line_corners = vec![];

        let mut offset_y = px(0.);
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

        let first_p = *points.get(0).unwrap();
        let mut builder = gpui::PathBuilder::fill();
        builder.move_to(bounds.origin + first_p);
        for p in points.iter().skip(1) {
            builder.line_to(bounds.origin + *p);
        }

        builder.build().ok()
    }
}

pub(super) struct PrepaintState {
    lines: SmallVec<[WrappedLine; 1]>,
    cursor: Option<PaintQuad>,
    cursor_scroll_offset: Point<Pixels>,
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

    fn request_layout(
        &mut self,
        _id: Option<&GlobalElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let input = self.input.read(cx);
        let mut style = Style::default();
        style.size.width = relative(1.).into();
        if self.input.read(cx).is_multi_line() {
            style.flex_grow = 1.0;
            if let Some(h) = input.height {
                style.size.height = h.into();
                style.min_size.height = window.line_height().into();
            } else {
                style.size.height = relative(1.).into();
                style.min_size.height = (input.rows.max(1) as f32 * window.line_height()).into();
            }
        } else {
            // For single-line inputs, the minimum height should be the line height
            style.size.height = window.line_height().into();
        };

        (window.request_layout(style, [], cx), ())
    }

    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        let multi_line = self.input.read(cx).is_multi_line();
        let line_height = window.line_height();
        let input = self.input.read(cx);
        let text = input.text.clone();
        let placeholder = input.placeholder.clone();
        let style = window.text_style();
        let mut bounds = bounds;

        let (display_text, text_color) = if text.is_empty() {
            (placeholder, cx.theme().muted_foreground)
        } else if input.masked {
            (
                "*".repeat(text.chars().count()).into(),
                cx.theme().foreground,
            )
        } else {
            (text, cx.theme().foreground)
        };

        let run = TextRun {
            len: display_text.len(),
            font: style.font(),
            color: text_color,
            background_color: None,
            underline: None,
            strikethrough: None,
        };

        let runs = if let Some(marked_range) = input.marked_range.as_ref() {
            vec![
                TextRun {
                    len: marked_range.start,
                    ..run.clone()
                },
                TextRun {
                    len: marked_range.end - marked_range.start,
                    underline: Some(UnderlineStyle {
                        color: Some(run.color),
                        thickness: px(1.0),
                        wavy: false,
                    }),
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

        let font_size = style.font_size.to_pixels(window.rem_size());
        let wrap_width = if multi_line {
            Some(bounds.size.width - RIGHT_MARGIN)
        } else {
            None
        };

        let lines = window
            .text_system()
            .shape_text(display_text, font_size, &runs, wrap_width, None)
            .unwrap();

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

        let (cursor, cursor_scroll_offset) =
            self.layout_cursor(&lines, line_height, &mut bounds, window, cx);

        let selection_path = self.layout_selections(&lines, line_height, &mut bounds, window, cx);

        PrepaintState {
            bounds,
            lines,
            cursor,
            cursor_scroll_offset,
            selection_path,
        }
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        input_bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        let focus_handle = self.input.read(cx).focus_handle.clone();
        let focused = focus_handle.is_focused(window);
        let bounds = prepaint.bounds;
        let selected_range = self.input.read(cx).selected_range.clone();

        window.handle_input(
            &focus_handle,
            ElementInputHandler::new(bounds, self.input.clone()),
            cx,
        );

        // Set Root focused_input when self is focused
        if focused {
            let input_view = self.input.clone();
            if Root::read(window, cx).focused_input.as_ref() != Some(&input_view) {
                Root::update(window, cx, |root, _, cx| {
                    root.focused_input = Some(input_view);
                    cx.notify();
                });
            }
        }

        // And reset focused_input when next_frame start
        window.on_next_frame({
            let input_view = self.input.clone();
            move |window, cx| {
                if !focused && Root::read(window, cx).focused_input.as_ref() == Some(&input_view) {
                    Root::update(window, cx, |root, _, cx| {
                        root.focused_input = None;
                        cx.notify();
                    });
                }
            }
        });

        // Paint selections
        if let Some(path) = prepaint.selection_path.take() {
            window.paint_path(path, cx.theme().selection);
        }

        // Paint multi line text
        let line_height = window.line_height();
        let origin = bounds.origin;

        let mut offset_y = px(0.);
        if self.input.read(cx).masked {
            // Move down offset for vertical centering the *****
            if cfg!(target_os = "macos") {
                offset_y = px(3.);
            } else {
                offset_y = px(2.5);
            }
        }
        for line in prepaint.lines.iter() {
            let p = point(origin.x, origin.y + offset_y);
            _ = line.paint(p, line_height, TextAlign::Left, None, window, cx);
            offset_y += line.size(line_height).height;
        }

        if focused {
            if let Some(cursor) = prepaint.cursor.take() {
                window.paint_quad(cursor);
            }
        }

        let width = prepaint
            .lines
            .iter()
            .map(|l| l.width())
            .max()
            .unwrap_or_default();
        let height = prepaint
            .lines
            .iter()
            .map(|l| l.size(line_height).height.0)
            .sum::<f32>();

        let scroll_size = size(width, px(height));

        self.input.update(cx, |input, _cx| {
            input.last_layout = Some(prepaint.lines.clone());
            input.last_bounds = Some(bounds);
            input.last_cursor_offset = Some(input.cursor_offset());
            input.last_line_height = line_height;
            input.input_bounds = input_bounds;
            input.last_selected_range = Some(selected_range);
            input
                .scroll_handle
                .set_offset(prepaint.cursor_scroll_offset);
            input.scroll_size = scroll_size;
        });

        self.paint_mouse_listeners(window, cx);
    }
}
