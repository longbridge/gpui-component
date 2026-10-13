//! `BlockEditor` view (design §7).
//!
//! `render` takes the cached projection and renders it with gpui-kit's own
//! `BlockNode` renderer, so the editor looks exactly like `TextView`. The
//! [`EditorRoot`] element installs a frame-local [`FrameSink`] while its
//! children prepaint: keyed text leaves report their layouts into it and take
//! their selection backgrounds from it (`text::editor_seam`, D3). The root
//! then paints the caret and the composition underline from that geometry.
//! Paint never updates the editor entity.

use std::{collections::HashMap, ops::Range, rc::Rc, sync::Arc};

use gpui::{
    AnyElement, App, Bounds, CursorStyle, DispatchPhase, Element, ElementId, ElementInputHandler,
    Entity, GlobalElementId, Hitbox, HitboxBehavior, Hsla, InspectorElementId, InteractiveElement,
    IntoElement, LayoutId, MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent,
    ParentElement, Pixels, RenderOnce, Styled, Window, div, fill, point, px, size,
};

use super::{
    cull::Measured,
    frame::FrameSink,
    id::BlockId,
    projection::{LeafInfo, Snapshot},
    state::*,
    style::BlockEditorStyle,
};
use crate::text::{
    TextViewDefaults, TextViewStyle,
    document::{NodeRenderOptions, PrevBlock},
    node::{BlockNode, NodeContext},
};

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

/// `flow_position` over the flattened top-level nodes.
fn prev_block(nodes: &[&BlockNode], ix: usize) -> (bool, PrevBlock) {
    let prev = nodes[..ix].iter().rev().find(|block| {
        !matches!(
            block,
            BlockNode::Definition { .. } | BlockNode::Break { .. } | BlockNode::Unknown
        )
    });
    match prev {
        None => (true, PrevBlock::Other),
        Some(BlockNode::Heading { level, .. }) => (false, PrevBlock::Heading(*level)),
        Some(BlockNode::HorizontalRule { .. }) => (false, PrevBlock::Rule),
        Some(_) => (false, PrevBlock::Other),
    }
}

impl RenderOnce for BlockEditor {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = crate::Theme::global(cx);
        let defaults = cx.try_global::<TextViewDefaults>();
        let text_style = defaults
            .and_then(|d| d.style().cloned())
            .unwrap_or_else(|| TextViewStyle::from_theme(&theme));
        let highlighter = defaults.and_then(|d| d.code_block_highlighter());
        let mut style = self.state.read(cx).style.resolved(&theme.tokens);
        if style.selection_is_default() {
            style = style.with_selection(text_style.selection());
        }
        let (focus, snapshot, backgrounds, extensions, keep) = self.state.update(cx, |state, _| {
            let snapshot = state.snapshot();
            let mut backgrounds: HashMap<u64, Vec<(Range<usize>, Hsla)>> = HashMap::new();
            for (block, range) in state.selection_ranges() {
                let range = match snapshot.leaves.iter().find(|l| l.block == block) {
                    Some(LeafInfo {
                        atomic: true, len, ..
                    }) => 0..*len,
                    _ => range,
                };
                backgrounds
                    .entry(block.as_u64())
                    .or_default()
                    .push((range, style.selection()));
            }
            (
                state.focus_handle.clone(),
                snapshot,
                backgrounds,
                state.model.doc.extensions.clone(),
                state.pinned_blocks(),
            )
        });

        let node_cx = NodeContext {
            style: Arc::new(text_style),
            code_block_highlighter: highlighter,
            markdown_extensions: extensions,
            body_font_size: window.text_style().font_size.to_pixels(window.rem_size()),
            ..Default::default()
        };
        let nodes: Vec<&BlockNode> = snapshot
            .blocks
            .iter()
            .flat_map(|b| b.nodes.iter())
            .collect();
        let owners: Vec<BlockId> = snapshot
            .blocks
            .iter()
            .flat_map(|b| b.nodes.iter().map(|_| b.id))
            .collect();
        let count = nodes.len();
        let cull = self.state.read(cx).cull.clone();
        let mut children: Vec<AnyElement> = Vec::with_capacity(count.min(256));
        {
            let mut cull_state = cull.borrow_mut();
            cull_state.measured.clear();
            let window_span = cull_state.visible.map(|(top, bottom)| {
                let margin = (bottom - top).max(px(200.));
                (top - margin, bottom + margin)
            });
            let mut y = px(0.);
            let mut spacer = px(0.);
            let mut rendered = 0;
            for (ix, node) in nodes.iter().enumerate() {
                let id = owners[ix];
                let height = cull_state.height(id);
                let visible = match window_span {
                    None => true,
                    Some((top, bottom)) => y + height >= top && y <= bottom,
                };
                y += height;
                if !visible && !keep.contains(&id) {
                    spacer += height;
                    continue;
                }
                if spacer > px(0.) {
                    children.push(div().h(spacer).into_any_element());
                    spacer = px(0.);
                }
                rendered += 1;
                let (is_first, prev) = prev_block(&nodes, ix);
                let element = node.render_block(
                    NodeRenderOptions {
                        ix,
                        is_first,
                        prev,
                        is_last: ix + 1 == count,
                        ..Default::default()
                    },
                    &node_cx,
                    window,
                    cx,
                );
                children.push(
                    Measured {
                        id,
                        index: ix,
                        state: cull.clone(),
                        child: Some(element),
                    }
                    .into_any_element(),
                );
            }
            if spacer > px(0.) {
                children.push(div().h(spacer).into_any_element());
            }
            cull_state.rendered = rendered;
        }

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
            .child(EditorRoot {
                state: self.state.clone(),
                child: Some(div().w_full().children(children).into_any_element()),
                sink: Some(Rc::new(FrameSink::new(backgrounds))),
                snapshot,
                style,
            })
    }
}

struct EditorRoot {
    state: Entity<BlockEditorState>,
    child: Option<AnyElement>,
    sink: Option<Rc<FrameSink>>,
    snapshot: Rc<Snapshot>,
    style: BlockEditorStyle,
}

impl IntoElement for EditorRoot {
    type Element = Self;
    fn into_element(self) -> Self::Element {
        self
    }
}

pub(crate) struct Prepaint {
    hitbox: Hitbox,
}

impl Element for EditorRoot {
    type RequestLayoutState = AnyElement;
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
        cx: &mut App,
    ) -> (LayoutId, AnyElement) {
        let mut child = self
            .child
            .take()
            .unwrap_or_else(|| div().into_any_element());
        let layout_id = child.request_layout(window, cx);
        (layout_id, child)
    }

    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        child: &mut AnyElement,
        window: &mut Window,
        cx: &mut App,
    ) -> Prepaint {
        if let Some(sink) = self.sink.take() {
            crate::text::editor_seam::with_sink(sink.clone(), || child.prepaint(window, cx));
            // Swap in the complete frame; handlers read it until the next one.
            if let Ok(sink) = Rc::try_unwrap(sink) {
                let frame = sink.into_frame(self.snapshot.leaves.clone(), &self.snapshot.tables);
                *self.state.read(cx).frame.borrow_mut() = frame;
            }
            let mask = window.content_mask().bounds;
            let cull = self.state.read(cx).cull.clone();
            if cull.borrow_mut().finish_frame(bounds, mask) {
                // Scrolled past what was laid out: lay out the new span.
                let state = self.state.clone();
                window.on_next_frame(move |_, cx| state.update(cx, |_, cx| cx.notify()));
            }
        } else {
            child.prepaint(window, cx);
        }
        Prepaint {
            hitbox: window.insert_hitbox(bounds, HitboxBehavior::Normal),
        }
    }

    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        child: &mut AnyElement,
        prepaint: &mut Prepaint,
        window: &mut Window,
        cx: &mut App,
    ) {
        let state = self.state.read(cx);
        let focus = state.focus_handle.clone();
        let collapsed = state.selection_ranges().is_empty();
        let marked = state.marked_range();
        let caret = state.model.caret();
        let show_caret = state.show_caret(window, cx) && collapsed;
        let frame = state.frame.clone();

        window.handle_input(
            &focus,
            ElementInputHandler::new(bounds, self.state.clone()),
            cx,
        );
        child.paint(window, cx);

        let frame = frame.borrow();
        let style = &self.style;
        if let Some((block, range)) = marked
            && let Some(leaf) = frame.leaf(block)
        {
            let a = leaf.caret_point(range.start);
            let b = leaf.caret_point(range.end);
            let thickness = style.caret_width();
            let underline = Bounds::new(
                point(a.x, a.y + leaf.line_height() - thickness),
                size((b.x - a.x).max(thickness), thickness),
            );
            window.paint_quad(fill(underline, style.composition_underline()));
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
                let focus = state.focus_handle.clone();
                window.focus(&focus, cx);
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
