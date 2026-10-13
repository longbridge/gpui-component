//! `BlockEditor` view and its custom element (design §7).
//!
//! `render` takes the cached projection snapshot; prepaint shapes leaves and
//! builds the [`FrameLayout`]; paint draws from that layout only and hands
//! it to the state's frame sink. Paint never updates the editor entity.

use std::rc::Rc;

use gpui::{
    App, Bounds, CursorStyle, DispatchPhase, Element, ElementId, ElementInputHandler, Entity,
    FontStyle, FontWeight, GlobalElementId, Hitbox, HitboxBehavior, InspectorElementId,
    InteractiveElement, IntoElement, LayoutId, MouseButton, MouseDownEvent, MouseMoveEvent,
    MouseUpEvent, ParentElement, Pixels, RenderOnce, SharedString, StrikethroughStyle, Style,
    Styled, TextAlign, TextRun, UnderlineStyle, Window, div, fill, point, px, relative, size,
};

use super::{
    frame::{FrameLayout, LeafFrame, LeafLine},
    projection::{LeafProjection, LeafStyle},
    state::*,
    style::BlockEditorStyle,
};
use crate::Theme;

/// A block-based WYSIWYG Markdown editor (spike). Hold an
/// `Entity<BlockEditorState>` and pass it here in `render`.
#[derive(IntoElement)]
pub struct BlockEditor {
    state: Entity<BlockEditorState>,
}

impl BlockEditor {
    pub fn new(state: &Entity<BlockEditorState>) -> Self {
        Self {
            state: state.clone(),
        }
    }
}

impl RenderOnce for BlockEditor {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let tokens = Theme::global(cx).tokens;
        let (focus, snapshot, style, frame_sink) = self.state.update(cx, |state, _| {
            (
                state.focus_handle.clone(),
                state.snapshot(),
                state.style.resolved(&tokens),
                state.frame.clone(),
            )
        });
        let s = &self.state;
        div()
            .id("block-editor")
            .key_context(CONTEXT)
            .track_focus(&focus)
            .cursor(CursorStyle::IBeam)
            .w_full()
            .on_action(window.listener_for(s, BlockEditorState::on_backspace))
            .on_action(window.listener_for(s, BlockEditorState::on_delete))
            .on_action(window.listener_for(s, BlockEditorState::on_enter))
            .on_action(window.listener_for(s, BlockEditorState::on_move_left))
            .on_action(window.listener_for(s, BlockEditorState::on_move_right))
            .on_action(window.listener_for(s, BlockEditorState::on_move_up))
            .on_action(window.listener_for(s, BlockEditorState::on_move_down))
            .on_action(window.listener_for(s, BlockEditorState::on_move_home))
            .on_action(window.listener_for(s, BlockEditorState::on_move_end))
            .on_action(window.listener_for(s, BlockEditorState::on_select_left))
            .on_action(window.listener_for(s, BlockEditorState::on_select_right))
            .on_action(window.listener_for(s, BlockEditorState::on_select_up))
            .on_action(window.listener_for(s, BlockEditorState::on_select_down))
            .on_action(window.listener_for(s, BlockEditorState::on_select_home))
            .on_action(window.listener_for(s, BlockEditorState::on_select_end))
            .on_action(window.listener_for(s, BlockEditorState::on_select_all))
            .on_action(window.listener_for(s, BlockEditorState::on_undo))
            .on_action(window.listener_for(s, BlockEditorState::on_redo))
            .on_action(window.listener_for(s, BlockEditorState::on_toggle_bold))
            .on_action(window.listener_for(s, BlockEditorState::on_toggle_italic))
            .on_action(window.listener_for(s, BlockEditorState::on_toggle_code))
            .on_action(window.listener_for(s, BlockEditorState::on_indent))
            .on_action(window.listener_for(s, BlockEditorState::on_outdent))
            .on_action(window.listener_for(s, BlockEditorState::on_copy))
            .on_action(window.listener_for(s, BlockEditorState::on_cut))
            .on_action(window.listener_for(s, BlockEditorState::on_paste))
            .child(BlockEditorElement {
                state: self.state.clone(),
                snapshot,
                style,
                mono: tokens.typography.mono.clone(),
                frame_sink,
            })
    }
}

struct BlockEditorElement {
    state: Entity<BlockEditorState>,
    snapshot: Rc<Vec<Rc<LeafProjection>>>,
    style: BlockEditorStyle,
    mono: SharedString,
    frame_sink: Rc<std::cell::RefCell<FrameLayout>>,
}

impl IntoElement for BlockEditorElement {
    type Element = Self;
    fn into_element(self) -> Self::Element {
        self
    }
}

/// Text metrics for one leaf style.
struct Metrics {
    font_size: Pixels,
    line_height: Pixels,
}

fn metrics(style: LeafStyle, window: &Window) -> Metrics {
    let base = window.text_style().font_size.to_pixels(window.rem_size());
    let base_line = window.line_height();
    let scale = match style {
        LeafStyle::Heading(1) => 2.0,
        LeafStyle::Heading(2) => 1.5,
        LeafStyle::Heading(3) => 1.25,
        LeafStyle::Heading(_) => 1.1,
        LeafStyle::Code => 0.9,
        _ => 1.0,
    };
    Metrics {
        font_size: base * scale,
        line_height: base_line * scale,
    }
}

fn text_runs(
    leaf: &LeafProjection,
    style: &BlockEditorStyle,
    mono: &SharedString,
    window: &Window,
) -> Vec<TextRun> {
    let base_font = window.text_style().font();
    let heading = matches!(leaf.style, LeafStyle::Heading(_));
    let color = match leaf.style {
        LeafStyle::Atom => style.muted_foreground(),
        _ if leaf.quote => style.muted_foreground(),
        _ => style.foreground(),
    };
    leaf.runs
        .iter()
        .map(|(len, mark)| {
            let mut font = base_font.clone();
            if mark.code {
                font.family = mono.clone();
            }
            if mark.bold || heading {
                font.weight = FontWeight::BOLD;
            }
            if mark.italic {
                font.style = FontStyle::Italic;
            }
            let line = |c| Some(c);
            TextRun {
                len: *len,
                font,
                color,
                background_color: mark
                    .highlight
                    .or((mark.code && leaf.style != LeafStyle::Code)
                        .then(|| style.code_background())),
                underline: (mark.underline || mark.link.is_some()).then(|| UnderlineStyle {
                    thickness: px(1.),
                    color: line(color),
                    wavy: false,
                }),
                strikethrough: mark.strikethrough.then(|| StrikethroughStyle {
                    thickness: px(1.),
                    color: line(color),
                }),
            }
        })
        .collect()
}

/// Shape every leaf at `width`; returns the frame with origins relative to
/// `origin`.
fn layout_leaves(
    snapshot: &[Rc<LeafProjection>],
    style: &BlockEditorStyle,
    mono: &SharedString,
    origin: gpui::Point<Pixels>,
    width: Pixels,
    window: &mut Window,
) -> (FrameLayout, Pixels) {
    let mut frame = FrameLayout::default();
    let mut y = px(0.);
    let gap_unit = window.line_height() * style.block_gap();
    for (ix, leaf) in snapshot.iter().enumerate() {
        let m = metrics(leaf.style, window);
        let indent = style.indent() * leaf.depth as f32;
        let pad = if leaf.style == LeafStyle::Code {
            px(8.)
        } else {
            px(0.)
        };
        let text_width = (width - indent - pad * 2.).max(px(16.));
        let runs = text_runs(leaf, style, mono, window);
        let shaped = window
            .text_system()
            .shape_text(
                leaf.text.clone(),
                m.font_size,
                &runs,
                Some(text_width),
                None,
            )
            .unwrap_or_default();
        let mut lines = Vec::with_capacity(shaped.len());
        let mut start = 0;
        let mut top = px(0.);
        for line in shaped {
            let len = line.len();
            let height = line.size(m.line_height).height.max(m.line_height);
            lines.push(LeafLine { start, top, line });
            start += len + 1;
            top += height;
        }
        let height = top.max(m.line_height);
        if ix > 0 {
            y += if leaf.marker.is_some() || leaf.depth > 0 {
                gap_unit * 0.5
            } else {
                gap_unit
            };
        }
        frame.push(LeafFrame {
            block: leaf.block,
            origin: origin + point(indent + pad, y + pad),
            width: text_width,
            height: height + pad * 2.,
            line_height: m.line_height,
            atomic: matches!(
                leaf.style,
                LeafStyle::Code | LeafStyle::Atom | LeafStyle::Rule
            ),
            lines,
        });
        y += height + pad * 2.;
    }
    frame.bounds = Bounds::new(origin, size(width, y));
    (frame, y)
}

pub(crate) struct Prepaint {
    hitbox: Hitbox,
}

impl Element for BlockEditorElement {
    type RequestLayoutState = ();
    type PrepaintState = Prepaint;

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        window: &mut Window,
        _: &mut App,
    ) -> (LayoutId, ()) {
        let mut style = Style::default();
        style.size.width = relative(1.).into();
        let snapshot = self.snapshot.clone();
        let editor_style = self.style.clone();
        let mono = self.mono.clone();
        let layout_id =
            window.request_measured_layout(style, move |known, available, window, _| {
                let width = known.width.unwrap_or(match available.width {
                    gpui::AvailableSpace::Definite(w) => w,
                    _ => px(640.),
                });
                let (_, height) = layout_leaves(
                    &snapshot,
                    &editor_style,
                    &mono,
                    point(px(0.), px(0.)),
                    width,
                    window,
                );
                size(width, height)
            });
        (layout_id, ())
    }

    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _: &mut (),
        window: &mut Window,
        _: &mut App,
    ) -> Prepaint {
        let (frame, _) = layout_leaves(
            &self.snapshot,
            &self.style,
            &self.mono,
            bounds.origin,
            bounds.size.width,
            window,
        );
        // Swap in the complete frame; handlers read it until the next one.
        *self.frame_sink.borrow_mut() = frame;
        Prepaint {
            hitbox: window.insert_hitbox(bounds, HitboxBehavior::Normal),
        }
    }

    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _: &mut (),
        prepaint: &mut Prepaint,
        window: &mut Window,
        cx: &mut App,
    ) {
        let state = self.state.read(cx);
        let focus = state.focus_handle.clone();
        let selection = state.selection_ranges();
        let marked = state.marked_range();
        let caret = state.model.caret();
        let show_caret = state.show_caret(window, cx) && selection.is_empty();
        let frame = self.frame_sink.clone();
        let frame = frame.borrow();
        let style = &self.style;

        window.handle_input(
            &focus,
            ElementInputHandler::new(bounds, self.state.clone()),
            cx,
        );

        // Block chrome: code backgrounds, rules, list markers.
        for (leaf, proj) in frame.leaves.iter().zip(self.snapshot.iter()) {
            match proj.style {
                LeafStyle::Code => {
                    let pad = px(8.);
                    let b = Bounds::new(
                        leaf.origin - point(pad, pad),
                        size(leaf.width + pad * 2., leaf.height),
                    );
                    window.paint_quad(fill(b, style.code_background()).corner_radii(px(4.)));
                }
                LeafStyle::Rule => {
                    let y = leaf.origin.y + leaf.line_height / 2.;
                    let b = Bounds::new(point(leaf.origin.x, y), size(leaf.width, px(1.)));
                    window.paint_quad(fill(b, style.muted_foreground()));
                }
                _ => {}
            }
            if proj.quote {
                let b = Bounds::new(
                    leaf.origin - point(style.indent() * 0.6, px(0.)),
                    size(px(3.), leaf.height),
                );
                window.paint_quad(fill(b, style.muted_foreground().opacity(0.4)));
            }
            if let Some(marker) = &proj.marker {
                let m = metrics(proj.style, window);
                let run = TextRun {
                    len: marker.len(),
                    font: window.text_style().font(),
                    color: style.muted_foreground(),
                    background_color: None,
                    underline: None,
                    strikethrough: None,
                };
                let line =
                    window
                        .text_system()
                        .shape_line(marker.clone(), m.font_size, &[run], None);
                let x = leaf.origin.x - line.width - px(6.);
                let _ = line.paint(
                    point(x, leaf.origin.y),
                    m.line_height,
                    TextAlign::Left,
                    None,
                    window,
                    cx,
                );
            }
        }

        // Selection backgrounds, under the glyphs.
        for (block, range) in &selection {
            let Some(leaf) = frame.leaf(*block) else {
                continue;
            };
            paint_range(leaf, range.clone(), style.selection(), window);
        }

        for leaf in &frame.leaves {
            for line in &leaf.lines {
                let _ = line.line.paint(
                    leaf.origin + point(px(0.), line.top),
                    leaf.line_height,
                    TextAlign::Left,
                    None,
                    window,
                    cx,
                );
            }
        }

        if let Some((block, range)) = marked
            && let Some(leaf) = frame.leaf(block)
        {
            let a = leaf.origin + leaf.local_point(range.start);
            let b = leaf.origin + leaf.local_point(range.end);
            let u = Bounds::new(
                point(a.x, a.y + leaf.line_height - px(2.)),
                size((b.x - a.x).max(px(1.)), px(1.)),
            );
            window.paint_quad(fill(u, style.composition_underline()));
        }

        if show_caret
            && let Some(pos) = caret
            && let Some(b) = frame.caret_bounds(pos, style.caret_width())
        {
            window.paint_quad(fill(b, style.caret()));
        }
        drop(frame);

        // Mouse: handlers update the entity at event time, never in paint.
        let hitbox = prepaint.hitbox.clone();
        let entity = self.state.clone();
        window.on_mouse_event(move |event: &MouseDownEvent, phase, window, cx| {
            if phase != DispatchPhase::Bubble
                || event.button != MouseButton::Left
                || !hitbox.is_hovered(window)
            {
                return;
            }
            entity.update(cx, |state, cx| {
                state.focus(window, cx);
                state.dragging = true;
                state.mouse_at(event.position, event.modifiers.shift, cx);
            });
        });
        let entity = self.state.clone();
        window.on_mouse_event(move |event: &MouseMoveEvent, phase, _, cx| {
            if phase != DispatchPhase::Bubble || event.pressed_button != Some(MouseButton::Left) {
                return;
            }
            entity.update(cx, |state, cx| {
                if state.dragging {
                    state.mouse_at(event.position, true, cx);
                }
            });
        });
        let entity = self.state.clone();
        window.on_mouse_event(move |_: &MouseUpEvent, phase, _, cx| {
            if phase == DispatchPhase::Bubble {
                entity.update(cx, |state, _| state.dragging = false);
            }
        });
    }
}

fn paint_range(
    leaf: &LeafFrame,
    range: std::ops::Range<usize>,
    color: gpui::Hsla,
    window: &mut Window,
) {
    let lh = leaf.line_height;
    let a = leaf.local_point(range.start);
    let b = leaf.local_point(range.end);
    let rect = |x0: Pixels, x1: Pixels, y: Pixels| {
        Bounds::new(leaf.origin + point(x0, y), size((x1 - x0).max(px(4.)), lh))
    };
    if a.y == b.y {
        window.paint_quad(fill(rect(a.x, b.x, a.y), color));
        return;
    }
    window.paint_quad(fill(rect(a.x, leaf.width, a.y), color));
    let mut y = a.y + lh;
    while y < b.y {
        window.paint_quad(fill(rect(px(0.), leaf.width, y), color));
        y += lh;
    }
    window.paint_quad(fill(rect(px(0.), b.x, b.y), color));
}
