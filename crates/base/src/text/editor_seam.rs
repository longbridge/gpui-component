//! Minimal editor seam on the shared text renderer (design Phase 0a, D3).
//!
//! A text leaf whose [`InlineState`](super::inline::InlineState) carries an
//! `editor_key` reports its laid-out [`TextLayout`] to the editor sink that
//! is installed for the duration of the editor's prepaint, and takes extra
//! range backgrounds (the editor's selection) from it. `TextView` never sets
//! a key or installs a sink, so read-only rendering is unchanged. The module
//! (and `InlineState::editor_key`) only exists with the `block-editor`
//! feature; without it there is no per-frame cost at all.
//!
//! The sink is frame-local: it is written during prepaint and read by the
//! editor's own handlers; nothing here reaches back into an entity.

use std::{cell::RefCell, ops::Range, rc::Rc};

use gpui::{Bounds, Hsla, Pixels, TextLayout};

pub(crate) trait EditorSink {
    /// The leaf `key` was laid out this frame as one `TextLayout`.
    fn report(&self, key: u64, layout: &TextLayout);
    /// The leaf `key` is laid out in pieces this frame (a mixed text /
    /// image / custom-inline paragraph rendered by `InlineFlow`); drops
    /// pieces reported for it earlier in the frame.
    fn begin_pieces(&self, key: u64);
    /// One single-line text fragment of `key`, whose index 0 is the leaf's
    /// byte `start`.
    fn report_text_piece(&self, key: u64, start: usize, layout: &TextLayout);
    /// An inline atom (image, custom node) of `key` covering the leaf bytes
    /// `range`, drawn in `bounds`.
    fn report_box_piece(&self, key: u64, range: Range<usize>, bounds: Bounds<Pixels>);
    /// Extra backgrounds to paint behind the leaf `key`, in its text's bytes.
    fn backgrounds(&self, key: u64) -> Vec<(Range<usize>, Hsla)>;
}

thread_local! {
    static SINK: RefCell<Option<Rc<dyn EditorSink>>> = const { RefCell::new(None) };
}

/// Run `f` with `sink` installed; restores the previous sink afterwards.
pub(crate) fn with_sink<R>(sink: Rc<dyn EditorSink>, f: impl FnOnce() -> R) -> R {
    let previous = SINK.with(|s| s.replace(Some(sink)));
    let result = f();
    SINK.with(|s| *s.borrow_mut() = previous);
    result
}

pub(crate) fn current() -> Option<Rc<dyn EditorSink>> {
    SINK.with(|s| s.borrow().clone())
}
