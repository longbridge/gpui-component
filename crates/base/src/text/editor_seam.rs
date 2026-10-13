//! Minimal editor seam on the shared text renderer (design Phase 0a, D3).
//!
//! A text leaf whose [`InlineState`](super::inline::InlineState) carries an
//! `editor_key` reports its laid-out [`TextLayout`] to the editor sink that
//! is installed for the duration of the editor's prepaint, and takes extra
//! range backgrounds (the editor's selection) from it. `TextView` never sets
//! a key or installs a sink, so read-only rendering is unchanged.
//!
//! The sink is frame-local: it is written during prepaint and read by the
//! editor's own handlers; nothing here reaches back into an entity.

use std::{cell::RefCell, ops::Range, rc::Rc};

use gpui::{Hsla, TextLayout};

pub(crate) trait EditorSink {
    /// The leaf `key` was laid out this frame.
    fn report(&self, key: u64, layout: &TextLayout);
    /// Extra backgrounds to paint behind the leaf `key`, in its text's bytes.
    fn backgrounds(&self, key: u64) -> Vec<(Range<usize>, Hsla)>;
}

thread_local! {
    static SINK: RefCell<Option<Rc<dyn EditorSink>>> = const { RefCell::new(None) };
}

/// Run `f` with `sink` installed; restores the previous sink afterwards.
#[cfg_attr(not(feature = "block-editor"), allow(dead_code))]
pub(crate) fn with_sink<R>(sink: Rc<dyn EditorSink>, f: impl FnOnce() -> R) -> R {
    let previous = SINK.with(|s| s.replace(Some(sink)));
    let result = f();
    SINK.with(|s| *s.borrow_mut() = previous);
    result
}

pub(crate) fn current() -> Option<Rc<dyn EditorSink>> {
    SINK.with(|s| s.borrow().clone())
}
