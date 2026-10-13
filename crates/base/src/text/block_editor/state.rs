//! `BlockEditorState`: the entity around [`BlockEditorModel`] (design §6).

use std::{cell::RefCell, ops::Range, rc::Rc};

use gpui::{
    App, AppContext, Bounds, ClipboardItem, Context, Entity, EntityInputHandler, EventEmitter,
    FocusHandle, Focusable, KeyBinding, Pixels, Point, SharedString, Subscription, UTF16Selection,
    Window, actions,
};

use super::{
    command::{CommandError, EditorCommand},
    editor::BlockEditorModel,
    frame::FrameLayout,
    id::BlockId,
    kind::BlockKind,
    marks::MarkKind,
    position::BlockPosition,
    projection::{ProjectionCache, Snapshot},
    selection::BlockSelection,
    style::BlockEditorStyle,
    transaction::EditOrigin,
};
use crate::{input::blink_cursor::BlinkCursor, text::MarkdownExtensions};

pub(crate) const CONTEXT: &str = "BlockEditor";

actions!(
    block_editor,
    [
        Backspace,
        Delete,
        Enter,
        MoveLeft,
        MoveRight,
        MoveUp,
        MoveDown,
        MoveHome,
        MoveEnd,
        SelectLeft,
        SelectRight,
        SelectUp,
        SelectDown,
        SelectHome,
        SelectEnd,
        SelectAll,
        Undo,
        Redo,
        ToggleBold,
        ToggleItalic,
        ToggleCode,
        Indent,
        Outdent,
        Copy,
        Cut,
        Paste,
    ]
);

pub(crate) fn init(cx: &mut App) {
    let ctx = Some(CONTEXT);
    #[cfg(target_os = "macos")]
    let m = "cmd";
    #[cfg(not(target_os = "macos"))]
    let m = "ctrl";
    cx.bind_keys(vec![
        KeyBinding::new("backspace", Backspace, ctx),
        KeyBinding::new("delete", Delete, ctx),
        KeyBinding::new("enter", Enter, ctx),
        KeyBinding::new("left", MoveLeft, ctx),
        KeyBinding::new("right", MoveRight, ctx),
        KeyBinding::new("up", MoveUp, ctx),
        KeyBinding::new("down", MoveDown, ctx),
        KeyBinding::new("home", MoveHome, ctx),
        KeyBinding::new("end", MoveEnd, ctx),
        KeyBinding::new("shift-left", SelectLeft, ctx),
        KeyBinding::new("shift-right", SelectRight, ctx),
        KeyBinding::new("shift-up", SelectUp, ctx),
        KeyBinding::new("shift-down", SelectDown, ctx),
        KeyBinding::new("shift-home", SelectHome, ctx),
        KeyBinding::new("shift-end", SelectEnd, ctx),
        KeyBinding::new("tab", Indent, ctx),
        KeyBinding::new("shift-tab", Outdent, ctx),
        KeyBinding::new(&format!("{m}-a"), SelectAll, ctx),
        KeyBinding::new(&format!("{m}-z"), Undo, ctx),
        KeyBinding::new(&format!("{m}-shift-z"), Redo, ctx),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-y", Redo, ctx),
        KeyBinding::new(&format!("{m}-b"), ToggleBold, ctx),
        KeyBinding::new(&format!("{m}-i"), ToggleItalic, ctx),
        KeyBinding::new(&format!("{m}-e"), ToggleCode, ctx),
        KeyBinding::new(&format!("{m}-c"), Copy, ctx),
        KeyBinding::new(&format!("{m}-x"), Cut, ctx),
        KeyBinding::new(&format!("{m}-v"), Paste, ctx),
    ]);
}

/// Events emitted by [`BlockEditorState`].
#[non_exhaustive]
#[derive(Clone, Debug)]
pub enum BlockEditorEvent {
    /// Once per committed transaction, including undo and redo.
    Changed {
        revision: u64,
        origin: EditOrigin,
    },
    SelectionChanged,
    Focus,
    Blur,
}

/// IME composition in progress: the marked byte range in one leaf.
#[derive(Clone, Debug)]
struct Marked {
    block: BlockId,
    range: Range<usize>,
}

pub struct BlockEditorState {
    pub(crate) model: BlockEditorModel,
    pub(crate) focus_handle: FocusHandle,
    pub(crate) blink_cursor: Entity<BlinkCursor>,
    pub(crate) style: BlockEditorStyle,
    pub(crate) projection: ProjectionCache,
    /// Last complete frame, written by the element after prepaint.
    pub(crate) frame: Rc<RefCell<FrameLayout>>,
    pub(crate) cull: Rc<RefCell<super::cull::CullState>>,
    /// Scroll container to keep the caret in view (see `set_scroll_handle`).
    pub(crate) scroll_handle: Option<gpui::ScrollHandle>,
    /// The caret moved and should be scrolled into view after layout.
    pub(crate) reveal: std::cell::Cell<bool>,
    marked: Option<Marked>,
    /// An IME composition opened an undo group that is not committed yet.
    composition_open: bool,
    /// Remembered x for repeated up/down.
    goal_x: Option<Pixels>,
    pub(crate) dragging: bool,
    readonly: bool,
    _subscriptions: Vec<Subscription>,
}

impl EventEmitter<BlockEditorEvent> for BlockEditorState {}

impl Focusable for BlockEditorState {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl BlockEditorState {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let focus_handle = cx.focus_handle();
        let blink_cursor = cx.new(|_| BlinkCursor::new());
        // Focus and blink wiring as in `InputBaseState::new_in_mode` (§7.1).
        let subscriptions = vec![
            cx.observe(&blink_cursor, |_, _, cx| cx.notify()),
            cx.on_focus(&focus_handle, window, |this, _, cx| {
                this.blink_cursor.update(cx, |b, cx| b.start(cx));
                cx.emit(BlockEditorEvent::Focus);
            }),
            cx.on_blur(&focus_handle, window, |this, _, cx| {
                this.blink_cursor.update(cx, |b, cx| b.stop(cx));
                this.model.break_coalescing();
                this.end_composition();
                cx.emit(BlockEditorEvent::Blur);
            }),
            cx.observe_window_activation(window, |this, window, cx| {
                if window.is_window_active() && this.focus_handle.is_focused(window) {
                    this.blink_cursor.update(cx, |b, cx| b.start(cx));
                }
            }),
        ];
        let model = BlockEditorModel::from_markdown("", MarkdownExtensions::default())
            .unwrap_or_else(|_| unreachable!("empty source always parses"));
        Self {
            model,
            focus_handle,
            blink_cursor,
            style: BlockEditorStyle::default(),
            projection: ProjectionCache::default(),
            frame: Rc::default(),
            cull: Rc::default(),
            scroll_handle: None,
            reveal: Default::default(),
            marked: None,
            composition_open: false,
            goal_x: None,
            dragging: false,
            readonly: false,
            _subscriptions: subscriptions,
        }
    }

    /// Builder: initial Markdown.
    pub fn with_source(mut self, source: &str) -> Self {
        let _ = self.model.set_source(source);
        self.projection.invalidate();
        self
    }

    /// Replace the content. No `Changed` event; clears history.
    pub fn set_source(&mut self, source: &str, cx: &mut Context<Self>) -> Result<(), SharedString> {
        self.model.set_source(source)?;
        self.marked = None;
        self.projection.invalidate();
        cx.notify();
        Ok(())
    }

    pub fn source(&self) -> String {
        self.model.source()
    }

    pub fn model(&self) -> &BlockEditorModel {
        &self.model
    }

    pub fn revision(&self) -> u64 {
        self.model.revision()
    }

    pub fn selection(&self) -> &BlockSelection {
        self.model.selection()
    }

    pub fn block_kind_at_caret(&self) -> Option<BlockKind> {
        self.model.block_kind_at_caret()
    }

    pub fn set_editor_style(&mut self, style: BlockEditorStyle, cx: &mut Context<Self>) {
        self.style = style;
        cx.notify();
    }

    pub fn editor_style(&self) -> &BlockEditorStyle {
        &self.style
    }

    /// The scroll container the editor sits in (its `track_scroll` handle).
    /// With one set, the caret is scrolled into view after it moves; blocks
    /// outside the container's visible area are culled either way.
    pub fn set_scroll_handle(&mut self, handle: gpui::ScrollHandle) {
        self.scroll_handle = Some(handle);
    }

    pub fn is_readonly(&self) -> bool {
        self.readonly
    }

    pub fn set_readonly(&mut self, readonly: bool, cx: &mut Context<Self>) {
        self.readonly = readonly;
        cx.notify();
    }

    pub fn focus(&self, window: &mut Window, cx: &mut App) {
        window.focus(&self.focus_handle, cx);
    }

    pub fn set_selection(
        &mut self,
        selection: BlockSelection,
        cx: &mut Context<Self>,
    ) -> Result<(), super::SelectionError> {
        self.model.set_selection(selection)?;
        self.selection_changed(cx);
        Ok(())
    }

    /// Run a model command as one transaction.
    pub fn run(
        &mut self,
        command: EditorCommand,
        cx: &mut Context<Self>,
    ) -> Result<bool, CommandError> {
        if self.readonly {
            return Ok(false);
        }
        let name = command_name(&command);
        self.end_composition();
        let revision = self.model.revision();
        let changed = self.model.run(command)?;
        if self.model.revision() != revision {
            self.after_edit(EditOrigin::Command(name), cx);
        } else {
            // Stored-mark toggles change no text but show in the toolbar.
            cx.notify();
        }
        Ok(changed)
    }

    pub fn undo(&mut self, cx: &mut Context<Self>) {
        if self.model.undo() {
            self.after_edit(EditOrigin::Undo, cx);
        }
    }

    pub fn redo(&mut self, cx: &mut Context<Self>) {
        if self.model.redo() {
            self.after_edit(EditOrigin::Redo, cx);
        }
    }

    pub(crate) fn snapshot(&mut self) -> Rc<Snapshot> {
        self.projection.snapshot(&self.model.doc)
    }

    /// Blocks that are always laid out: the selection ends, the IME
    /// composition, and their ancestors.
    pub(crate) fn pinned_blocks(&self) -> Vec<BlockId> {
        let doc = &self.model.doc;
        let mut ids = match self.model.selection() {
            BlockSelection::Text { anchor, head } => vec![anchor.block, head.block],
            BlockSelection::Blocks { anchor, head } => vec![*anchor, *head],
        };
        if let Some((block, _)) = self.marked_range() {
            ids.push(block);
        }
        // Every ancestor, so a chunk of a long list holding the caret's item
        // is pinned too.
        ids.into_iter().flat_map(|id| doc.path(id)).collect()
    }

    /// Per-leaf selected byte ranges, for painting.
    pub(crate) fn selection_ranges(&self) -> Vec<(BlockId, Range<usize>)> {
        let Ok(range) = self.model.doc.resolve_selection(self.model.selection()) else {
            return Vec::new();
        };
        if range.is_collapsed() {
            return Vec::new();
        }
        self.model
            .text_ranges(range)
            .into_iter()
            .map(|(id, lo, hi)| (id, lo..hi))
            .collect()
    }

    pub(crate) fn marked_range(&self) -> Option<(BlockId, Range<usize>)> {
        self.marked.as_ref().map(|m| (m.block, m.range.clone()))
    }

    pub(crate) fn show_caret(&self, window: &Window, cx: &App) -> bool {
        self.focus_handle.is_focused(window) && self.blink_cursor.read(cx).visible()
    }

    // ---- internal edit plumbing -------------------------------------------

    fn after_edit(&mut self, origin: EditOrigin, cx: &mut Context<Self>) {
        self.goal_x = None;
        self.reveal.set(true);
        self.pause_blink(cx);
        cx.emit(BlockEditorEvent::Changed {
            revision: self.model.revision(),
            origin,
        });
        cx.emit(BlockEditorEvent::SelectionChanged);
        cx.notify();
    }

    fn selection_changed(&mut self, cx: &mut Context<Self>) {
        self.reveal.set(true);
        self.pause_blink(cx);
        cx.emit(BlockEditorEvent::SelectionChanged);
        cx.notify();
    }

    fn pause_blink(&mut self, cx: &mut Context<Self>) {
        self.blink_cursor.update(cx, |b, cx| b.pause(cx));
    }

    fn edit(
        &mut self,
        origin: EditOrigin,
        f: impl FnOnce(&mut BlockEditorModel) -> Result<(), CommandError>,
        cx: &mut Context<Self>,
    ) {
        if self.readonly {
            return;
        }
        let revision = self.model.revision();
        // Errors (e.g. typing into a code block, Phase 4) leave the document
        // untouched; the spike ignores them.
        let _ = f(&mut self.model);
        if self.model.revision() != revision {
            self.after_edit(origin, cx);
        } else {
            cx.notify();
        }
    }

    fn end_composition(&mut self) {
        self.marked = None;
        if std::mem::take(&mut self.composition_open) {
            self.model.history.commit_transaction();
        }
    }

    /// Whether an IME undo group is open (tests: it must never leak).
    #[cfg(test)]
    pub(crate) fn composition_open(&self) -> bool {
        self.composition_open
    }

    /// A whole-block selection collapses to its start or end.
    fn collapse_blocks(&mut self, forward: bool, cx: &mut Context<Self>) -> bool {
        if !matches!(self.model.selection(), BlockSelection::Blocks { .. }) {
            return false;
        }
        if let Ok(range) = self.model.doc.resolve_selection(self.model.selection()) {
            let target = if forward { range.end } else { range.start };
            self.move_to(target, false, cx);
        }
        true
    }

    fn head_anchor(&self) -> Option<(BlockPosition, BlockPosition)> {
        match self.model.selection() {
            BlockSelection::Text { anchor, head } => Some((*anchor, *head)),
            BlockSelection::Blocks { .. } => None,
        }
    }

    fn move_to(&mut self, head: BlockPosition, extend: bool, cx: &mut Context<Self>) {
        let anchor = match (extend, self.head_anchor()) {
            (true, Some((anchor, _))) => anchor,
            _ => head,
        };
        self.end_composition();
        let _ = self
            .model
            .set_selection(BlockSelection::Text { anchor, head });
        self.selection_changed(cx);
    }

    /// One grapheme left/right, crossing leaf boundaries in document order.
    pub(crate) fn step_horizontal(&self, pos: BlockPosition, forward: bool) -> BlockPosition {
        let doc = &self.model.doc;
        let len = doc.leaf_len(pos.block).unwrap_or(0);
        if let Some(inline) = doc.inline(pos.block) {
            let next = if forward {
                inline.next_grapheme(pos.offset)
            } else {
                inline.prev_grapheme(pos.offset)
            };
            if let Some(offset) = next.filter(|o| *o != pos.offset) {
                return BlockPosition::new(pos.block, offset);
            }
        } else if forward && pos.offset == 0 {
            return BlockPosition::new(pos.block, 1);
        } else if !forward && pos.offset == 1 {
            return BlockPosition::new(pos.block, 0);
        }
        let leaves = doc.leaves();
        let Some(ix) = leaves.iter().position(|id| *id == pos.block) else {
            return pos;
        };
        let neighbour = if forward {
            leaves.get(ix + 1)
        } else {
            ix.checked_sub(1).and_then(|i| leaves.get(i))
        };
        match neighbour {
            Some(id) if forward => BlockPosition::new(*id, 0),
            Some(id) => BlockPosition::new(*id, doc.leaf_len(*id).unwrap_or(0)),
            None => BlockPosition::new(pos.block, if forward { len } else { 0 }),
        }
    }

    fn horizontal(&mut self, forward: bool, extend: bool, cx: &mut Context<Self>) {
        if self.collapse_blocks(forward, cx) {
            return;
        }
        let Some((anchor, head)) = self.head_anchor() else {
            return;
        };
        self.goal_x = None;
        // Collapsing a range moves to its edge, like every text field.
        if !extend && anchor != head {
            let ord = self.model.doc.cmp_positions(anchor, head).ok();
            let pick_head = matches!(
                (forward, ord),
                (true, Some(std::cmp::Ordering::Less)) | (false, Some(std::cmp::Ordering::Greater))
            );
            let target = if pick_head { head } else { anchor };
            return self.move_to(target, false, cx);
        }
        let next = self.step_horizontal(head, forward);
        self.move_to(next, extend, cx);
    }

    fn vertical(&mut self, down: bool, extend: bool, cx: &mut Context<Self>) {
        if self.collapse_blocks(down, cx) {
            return;
        }
        let Some((_, head)) = self.head_anchor() else {
            return;
        };
        let leaves = self.snapshot().leaves.clone();
        let Some(ix) = leaves.iter().position(|l| l.block == head.block) else {
            return;
        };
        // The model-adjacent caret leaf, which may be culled (no geometry).
        let adjacent = if down {
            leaves.get(ix + 1)
        } else {
            ix.checked_sub(1).and_then(|i| leaves.get(i))
        };
        let frame = self.frame.clone();
        let frame = frame.borrow();
        let laid_out = frame.caret_bounds(head, self.style.caret_width());
        let x = laid_out.map(|caret| *self.goal_x.get_or_insert(caret.origin.x));
        let visual = x.and_then(|x| frame.vertical(head, x, down));
        let target = match (visual, adjacent) {
            // Within the block, or into the laid-out adjacent leaf.
            (Some(t), _) if t.block == head.block => t,
            (Some(t), Some(next)) if t.block == next.block => t,
            // Into a culled (or not yet laid-out) leaf: model fallback.
            (_, Some(next)) => {
                let offset = if down { 0 } else { next.len };
                BlockPosition::new(next.block, offset)
            }
            (_, None) => {
                let doc = &self.model.doc;
                if down {
                    doc.last_caret()
                } else {
                    doc.first_caret()
                }
            }
        };
        drop(frame);
        let goal = self.goal_x;
        self.move_to(self.model.doc.clamp(target), extend, cx);
        self.goal_x = goal;
    }

    /// Home/End: the visual (soft-wrapped) line from the last frame, else
    /// the block edge.
    fn line_edge(&mut self, end: bool, extend: bool, cx: &mut Context<Self>) {
        if self.collapse_blocks(end, cx) {
            return;
        }
        let Some((_, head)) = self.head_anchor() else {
            return;
        };
        let visual = self.frame.borrow().line_edge(head, end);
        let target = visual.unwrap_or_else(|| {
            let offset = if end {
                self.model.doc.leaf_len(head.block).unwrap_or(0)
            } else {
                0
            };
            BlockPosition::new(head.block, offset)
        });
        self.goal_x = None;
        self.move_to(self.model.doc.clamp(target), extend, cx);
    }

    /// Mouse: place or extend the caret at a window point.
    pub(crate) fn mouse_at(&mut self, at: Point<Pixels>, extend: bool, cx: &mut Context<Self>) {
        let table = self.frame.borrow().table_at(at);
        if let Some(table) = table {
            self.end_composition();
            let _ = self.model.set_selection(BlockSelection::Blocks {
                anchor: table,
                head: table,
            });
            self.selection_changed(cx);
            return;
        }
        let hit = self.frame.borrow().hit_test(at);
        if let Some(pos) = hit {
            self.goal_x = None;
            let pos = self.model.doc.clamp(pos);
            self.move_to(pos, extend, cx);
        }
    }

    fn toggle_mark(&mut self, kind: MarkKind, cx: &mut Context<Self>) {
        let _ = self.run(EditorCommand::ToggleMark(kind), cx);
    }

    fn selected_text(&self) -> String {
        if let Some((parent, range)) = self.model.selected_blocks() {
            let ids = &self.model.doc.children(parent)[range];
            return super::serialize::join_blocks(&self.model.doc, ids);
        }
        let Ok(range) = self.model.doc.resolve_selection(self.model.selection()) else {
            return String::new();
        };
        self.model
            .text_ranges(range)
            .into_iter()
            .filter_map(|(id, lo, hi)| {
                let text = self.model.doc.inline(id)?.text();
                text.get(lo..hi).map(str::to_string)
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    // ---- action handlers --------------------------------------------------

    pub(crate) fn on_backspace(&mut self, _: &Backspace, _: &mut Window, cx: &mut Context<Self>) {
        self.end_composition();
        self.edit(EditOrigin::Backspace, |m| m.delete(false), cx);
    }
    pub(crate) fn on_delete(&mut self, _: &Delete, _: &mut Window, cx: &mut Context<Self>) {
        self.end_composition();
        self.edit(EditOrigin::DeleteForward, |m| m.delete(true), cx);
    }
    pub(crate) fn on_enter(&mut self, _: &Enter, _: &mut Window, cx: &mut Context<Self>) {
        self.end_composition();
        self.edit(EditOrigin::Command("split_block"), |m| m.split_block(), cx);
    }
    pub(crate) fn on_move_left(&mut self, _: &MoveLeft, _: &mut Window, cx: &mut Context<Self>) {
        self.horizontal(false, false, cx);
    }
    pub(crate) fn on_move_right(&mut self, _: &MoveRight, _: &mut Window, cx: &mut Context<Self>) {
        self.horizontal(true, false, cx);
    }
    pub(crate) fn on_move_up(&mut self, _: &MoveUp, _: &mut Window, cx: &mut Context<Self>) {
        self.vertical(false, false, cx);
    }
    pub(crate) fn on_move_down(&mut self, _: &MoveDown, _: &mut Window, cx: &mut Context<Self>) {
        self.vertical(true, false, cx);
    }
    pub(crate) fn on_move_home(&mut self, _: &MoveHome, _: &mut Window, cx: &mut Context<Self>) {
        self.line_edge(false, false, cx);
    }
    pub(crate) fn on_move_end(&mut self, _: &MoveEnd, _: &mut Window, cx: &mut Context<Self>) {
        self.line_edge(true, false, cx);
    }
    pub(crate) fn on_select_left(
        &mut self,
        _: &SelectLeft,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.horizontal(false, true, cx);
    }
    pub(crate) fn on_select_right(
        &mut self,
        _: &SelectRight,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.horizontal(true, true, cx);
    }
    pub(crate) fn on_select_up(&mut self, _: &SelectUp, _: &mut Window, cx: &mut Context<Self>) {
        self.vertical(false, true, cx);
    }
    pub(crate) fn on_select_down(
        &mut self,
        _: &SelectDown,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.vertical(true, true, cx);
    }
    pub(crate) fn on_select_home(
        &mut self,
        _: &SelectHome,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.line_edge(false, true, cx);
    }
    pub(crate) fn on_select_end(&mut self, _: &SelectEnd, _: &mut Window, cx: &mut Context<Self>) {
        self.line_edge(true, true, cx);
    }
    pub(crate) fn on_select_all(&mut self, _: &SelectAll, _: &mut Window, cx: &mut Context<Self>) {
        self.end_composition();
        let _ = self.model.run(EditorCommand::SelectAll);
        self.selection_changed(cx);
    }
    pub(crate) fn on_undo(&mut self, _: &Undo, _: &mut Window, cx: &mut Context<Self>) {
        self.end_composition();
        self.undo(cx);
    }
    pub(crate) fn on_redo(&mut self, _: &Redo, _: &mut Window, cx: &mut Context<Self>) {
        self.end_composition();
        self.redo(cx);
    }
    pub(crate) fn on_toggle_bold(
        &mut self,
        _: &ToggleBold,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.toggle_mark(MarkKind::Bold, cx);
    }
    pub(crate) fn on_toggle_italic(
        &mut self,
        _: &ToggleItalic,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.toggle_mark(MarkKind::Italic, cx);
    }
    pub(crate) fn on_toggle_code(
        &mut self,
        _: &ToggleCode,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.toggle_mark(MarkKind::Code, cx);
    }
    pub(crate) fn on_indent(&mut self, _: &Indent, _: &mut Window, cx: &mut Context<Self>) {
        let _ = self.run(EditorCommand::Indent, cx);
    }
    pub(crate) fn on_outdent(&mut self, _: &Outdent, _: &mut Window, cx: &mut Context<Self>) {
        let _ = self.run(EditorCommand::Outdent, cx);
    }
    pub(crate) fn on_copy(&mut self, _: &Copy, _: &mut Window, cx: &mut Context<Self>) {
        let text = self.selected_text();
        if !text.is_empty() {
            cx.write_to_clipboard(ClipboardItem::new_string(text));
        }
    }
    pub(crate) fn on_cut(&mut self, _: &Cut, window: &mut Window, cx: &mut Context<Self>) {
        self.end_composition();
        self.on_copy(&Copy, window, cx);
        self.edit(
            EditOrigin::Cut,
            |m| m.run(EditorCommand::DeleteSelection).map(|_| ()),
            cx,
        );
    }
    pub(crate) fn on_paste(&mut self, _: &Paste, _: &mut Window, cx: &mut Context<Self>) {
        if let Some(text) = cx.read_from_clipboard().and_then(|item| item.text()) {
            self.end_composition();
            self.edit(
                EditOrigin::Paste,
                |m| {
                    if m.paste_blocks(&text)? {
                        return Ok(());
                    }
                    m.insert_text(&text, EditOrigin::Paste)
                },
                cx,
            );
        }
    }

    /// Markdown input rules (design §5.3): `# `..`###### `, `- `/`* `/`+ `
    /// and `1. ` at the start of a plain paragraph. The conversion is its own
    /// undo group after the typing, so one undo reverts it and keeps the
    /// typed text.
    fn apply_input_rule(&mut self, cx: &mut Context<Self>) {
        use super::kind::BlockKindData;
        let Some(caret) = self.model.caret() else {
            return;
        };
        let doc = &self.model.doc;
        if doc.kind(caret.block) != Some(&BlockKindData::Paragraph) {
            return;
        }
        let in_list = doc
            .parent(caret.block)
            .and_then(|parent| doc.kind(parent))
            .is_some_and(|kind| matches!(kind, BlockKindData::ListItem { .. }));
        let Some(text) = doc.inline(caret.block).map(|i| i.text().to_string()) else {
            return;
        };
        let prefix = &text[..caret.offset.min(text.len())];
        let Some(marker) = prefix.strip_suffix(' ') else {
            return;
        };
        let (command, rule) =
            if !marker.is_empty() && marker.len() <= 6 && marker.bytes().all(|b| b == b'#') {
                let level = marker.len() as u8;
                (
                    EditorCommand::SetBlockKind(BlockKind::Heading { level }),
                    "heading",
                )
            } else if !in_list && matches!(marker, "-" | "*" | "+") {
                (EditorCommand::ToggleList { ordered: false }, "bullet_list")
            } else if !in_list
                && marker.len() >= 2
                && marker.ends_with('.')
                && marker[..marker.len() - 1]
                    .bytes()
                    .all(|b| b.is_ascii_digit())
            {
                (EditorCommand::ToggleList { ordered: true }, "ordered_list")
            } else {
                return;
            };
        let block = caret.block;
        let len = prefix.len();
        self.model.break_coalescing();
        self.model.history.begin_transaction();
        let _ = self.model.set_selection(BlockSelection::Text {
            anchor: BlockPosition::new(block, 0),
            head: BlockPosition::new(block, len),
        });
        let _ = self.model.delete(false);
        let _ = self.model.run(command);
        self.model.history.commit_transaction();
        self.model.break_coalescing();
        self.after_edit(EditOrigin::InputRule(rule), cx);
    }

    // ---- IME ---------------------------------------------------------------

    /// The IME sees the caret leaf as its document.
    fn ime_block(&self) -> Option<BlockId> {
        if let Some(m) = &self.marked {
            return Some(m.block);
        }
        let (_, head) = self.head_anchor()?;
        Some(head.block)
    }

    fn ime_text(&self) -> &str {
        self.ime_block()
            .and_then(|id| self.model.doc.inline(id))
            .map(|i| i.text())
            .unwrap_or("")
    }

    fn select_utf16(&mut self, block: BlockId, range16: Range<usize>) {
        let text = self.ime_text();
        let range = utf16_to_byte(text, range16.start)..utf16_to_byte(text, range16.end);
        let _ = self.model.set_selection(BlockSelection::Text {
            anchor: BlockPosition::new(block, range.start),
            head: BlockPosition::new(block, range.end),
        });
    }

    fn select_marked_or(&mut self, range16: Option<Range<usize>>) {
        let Some(block) = self.ime_block() else {
            return;
        };
        if let Some(range16) = range16 {
            self.select_utf16(block, range16);
        } else if let Some(m) = self.marked.clone() {
            let _ = self.model.set_selection(BlockSelection::Text {
                anchor: BlockPosition::new(m.block, m.range.start),
                head: BlockPosition::new(m.block, m.range.end),
            });
        }
    }
}

fn command_name(command: &EditorCommand) -> &'static str {
    match command {
        EditorCommand::ToggleMark(_) => "toggle_mark",
        EditorCommand::ApplyMark(_) => "apply_mark",
        EditorCommand::SetBlockKind(_) => "set_block_kind",
        EditorCommand::ToggleList { .. } => "toggle_list",
        EditorCommand::ToggleTask => "toggle_task",
        EditorCommand::Indent => "indent",
        EditorCommand::Outdent => "outdent",
        EditorCommand::MoveBlocks { .. } => "move_blocks",
        EditorCommand::InsertBlock(_) => "insert_block",
        EditorCommand::SelectAll => "select_all",
        EditorCommand::DeleteSelection => "delete_selection",
    }
}

pub(crate) fn utf16_to_byte(text: &str, utf16: usize) -> usize {
    let mut count = 0;
    for (ix, ch) in text.char_indices() {
        if count >= utf16 {
            return ix;
        }
        count += ch.len_utf16();
    }
    text.len()
}

pub(crate) fn byte_to_utf16(text: &str, byte: usize) -> usize {
    text.get(..byte.min(text.len()))
        .map(|s| s.encode_utf16().count())
        .unwrap_or(0)
}

impl EntityInputHandler for BlockEditorState {
    fn text_for_range(
        &mut self,
        range16: Range<usize>,
        adjusted: &mut Option<Range<usize>>,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<String> {
        let text = self.ime_text();
        let range = utf16_to_byte(text, range16.start)..utf16_to_byte(text, range16.end);
        adjusted.replace(byte_to_utf16(text, range.start)..byte_to_utf16(text, range.end));
        text.get(range).map(str::to_string)
    }

    fn selected_text_range(
        &mut self,
        _: bool,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<UTF16Selection> {
        let (anchor, head) = self.head_anchor()?;
        let text = self.ime_text();
        // Cross-block selections are reported as the caret leaf's part.
        let in_leaf = |p: BlockPosition| {
            if p.block == head.block {
                p.offset
            } else if self.model.doc.cmp_positions(p, head).ok() == Some(std::cmp::Ordering::Less) {
                0
            } else {
                text.len()
            }
        };
        let (a, h) = (in_leaf(anchor), head.offset);
        Some(UTF16Selection {
            range: byte_to_utf16(text, a.min(h))..byte_to_utf16(text, a.max(h)),
            reversed: h < a,
        })
    }

    fn marked_text_range(&self, _: &mut Window, _: &mut Context<Self>) -> Option<Range<usize>> {
        let m = self.marked.as_ref()?;
        let text = self.model.doc.inline(m.block)?.text();
        Some(byte_to_utf16(text, m.range.start)..byte_to_utf16(text, m.range.end))
    }

    fn unmark_text(&mut self, _: &mut Window, _: &mut Context<Self>) {
        self.end_composition();
    }

    fn replace_text_in_range(
        &mut self,
        range16: Option<Range<usize>>,
        text: &str,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.readonly {
            return;
        }
        let composing = self.marked.is_some();
        self.select_marked_or(range16);
        let origin = if composing {
            EditOrigin::Ime
        } else {
            EditOrigin::Typing
        };
        self.edit(origin, |m| m.insert_text(text, origin), cx);
        self.end_composition();
        if text == " " {
            self.apply_input_rule(cx);
        }
    }

    fn replace_and_mark_text_in_range(
        &mut self,
        range16: Option<Range<usize>>,
        new_text: &str,
        new_selected16: Option<Range<usize>>,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.readonly {
            return;
        }
        if new_text.is_empty() {
            // Cancelled composition: remove the marked text, close the group.
            if let Some(marked) = self.marked.clone() {
                self.select_marked_or(None);
                if !marked.range.is_empty() {
                    self.edit(EditOrigin::Ime, |m| m.delete(false), cx);
                }
            }
            self.end_composition();
            return;
        }
        self.select_marked_or(range16);
        let start = match self.model.doc.resolve_selection(self.model.selection()) {
            Ok(range) => range.start,
            Err(_) => return,
        };
        if !self.composition_open {
            // The whole composition is one undo step. Opened only once
            // something is about to be inserted; always closed by
            // `end_composition`.
            self.model.history.begin_transaction();
            self.composition_open = true;
        }
        let revision = self.model.revision();
        self.edit(
            EditOrigin::Ime,
            |m| m.insert_text(new_text, EditOrigin::Ime),
            cx,
        );
        let inserted = self.model.revision() != revision;
        let Some((_, head)) = self.head_anchor().filter(|_| inserted) else {
            // Nothing was inserted (e.g. into a code block): close the group.
            self.end_composition();
            return;
        };
        let block = head.block;
        let begin = if block == start.block {
            start.offset
        } else {
            head.offset.saturating_sub(new_text.len())
        };
        let range = begin..begin + new_text.len();
        self.marked = Some(Marked {
            block,
            range: range.clone(),
        });
        if let Some(sel16) = new_selected16 {
            let sel = utf16_to_byte(new_text, sel16.start)..utf16_to_byte(new_text, sel16.end);
            let _ = self.model.set_selection(BlockSelection::Text {
                anchor: BlockPosition::new(block, range.start + sel.start),
                head: BlockPosition::new(block, range.start + sel.end),
            });
        }
    }

    fn bounds_for_range(
        &mut self,
        range16: Range<usize>,
        _: Bounds<Pixels>,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<Bounds<Pixels>> {
        let block = self.ime_block()?;
        let text = self.ime_text();
        let start = utf16_to_byte(text, range16.start);
        let end = utf16_to_byte(text, range16.end);
        let frame = self.frame.borrow();
        let width = self.style.caret_width();
        let a = frame.caret_bounds(BlockPosition::new(block, start), width)?;
        let b = frame.caret_bounds(BlockPosition::new(block, end), width)?;
        Some(Bounds::from_corners(
            a.origin,
            gpui::point(
                b.origin.x.max(a.origin.x + width),
                a.origin.y + a.size.height,
            ),
        ))
    }

    fn character_index_for_point(
        &mut self,
        at: Point<Pixels>,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<usize> {
        let pos = self.frame.borrow().hit_test(at)?;
        if Some(pos.block) != self.ime_block() {
            return None;
        }
        Some(byte_to_utf16(self.ime_text(), pos.offset))
    }
}
