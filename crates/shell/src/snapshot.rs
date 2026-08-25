//! What a script render produces, and what many GPUI frames consume.
//!
//! A GPUI render is not a script render. GPUI repaints for reasons the script
//! knows nothing about — a cursor blink, a hover, a scroll, an animation frame —
//! and none of those are a reason to enter the VM. So a script `render()` no
//! longer describes *this frame*; it describes the current interface, once, into
//! a [`RenderSnapshot`] that stays valid until script state says otherwise.
//!
//! ```text
//! script state changes ──▶ build snapshot ──▶ ┌───────────┐
//!                                             │ snapshot  │
//!                                             └───────────┘
//!                                                  │  │  │
//!                          many GPUI frames ◀──────┘  │  │
//!                                       materialize ◀─┘  │
//!                                              (no VM) ◀─┘
//! ```
//!
//! The snapshot owns everything materialization needs: the element descriptions,
//! the root, and — indirectly, through its generation — the handlers the script
//! registered while building it. That ownership is the point. When the snapshot
//! is dropped its callbacks are retired with it, which is what lets several
//! views share one runtime without one view's render invalidating another's
//! buttons.

use std::rc::{Rc, Weak};

use crate::{
    engine::ShellRuntime,
    spec::{SpecArena, SpecId},
};

/// One frozen description of a script view's interface.
///
/// Built by the engine and read by [`crate::materialize`]; nothing mutates one
/// after it is published. A replacement is built beside it and swapped in whole,
/// so a script render that fails leaves the previous snapshot untouched.
pub struct RenderSnapshot {
    /// Identifies the callbacks registered while this snapshot was built.
    generation: u32,
    root: SpecId,
    arena: SpecArena,
    /// Weak so a snapshot never keeps the VM alive; a snapshot outliving its
    /// runtime has nothing to retire, and says so by failing to upgrade.
    runtime: Weak<ShellRuntime>,
}

impl RenderSnapshot {
    pub(crate) fn new(
        runtime: &Rc<ShellRuntime>,
        generation: u32,
        root: SpecId,
        arena: SpecArena,
    ) -> Self {
        Self {
            generation,
            root,
            arena,
            runtime: Rc::downgrade(runtime),
        }
    }

    pub fn root(&self) -> SpecId {
        self.root
    }

    pub fn arena(&self) -> &SpecArena {
        &self.arena
    }

    pub fn generation(&self) -> u32 {
        self.generation
    }

    /// The description as text. Rendering never needs a GPU to be verified, and
    /// reading a published snapshot never needs the VM.
    pub fn debug_tree(&self) -> String {
        self.arena.debug_tree(self.root)
    }

    /// How many nodes the script described. Used by benchmarks to report cost
    /// per node rather than per view.
    pub fn len(&self) -> usize {
        self.arena.len()
    }

    pub fn is_empty(&self) -> bool {
        self.arena.is_empty()
    }
}

/// Retiring on drop is what keeps callback lifetime tied to snapshot lifetime
/// rather than to a frame or to a global render counter.
impl Drop for RenderSnapshot {
    fn drop(&mut self) {
        if let Some(runtime) = self.runtime.upgrade() {
            runtime.retire_callbacks(self.generation);
        }
    }
}
