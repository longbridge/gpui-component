//! The single bridge between a script view and GPUI's render loop.
//!
//! Every script-defined view, panel, or dialog body is carried by a `ScriptView`
//! entity. GPUI calls `render` whenever the view is notified; the runtime turns
//! that into one script call plus one materialization pass.

use std::rc::Rc;

use crate::engine::{ShellRuntime, ViewObject};
use gpui::{Context, IntoElement, Render, Window};

pub struct ScriptView {
    runtime: Rc<ShellRuntime>,
    /// The script-side instance produced by the view type.
    object: ViewObject,
}

impl ScriptView {
    pub fn new(runtime: Rc<ShellRuntime>, object: ViewObject) -> Self {
        Self { runtime, object }
    }

    /// Replaces the script instance behind this view.
    ///
    /// Hot reload keeps the entity — and therefore the window, the focus and
    /// the element identities — and swaps only what the script produced.
    pub fn replace_object(&mut self, object: ViewObject) {
        self.object = object;
    }

    /// The script state behind this view, for host code that needs to read it.
    pub fn object(&self) -> &ViewObject {
        &self.object
    }
}

impl Render for ScriptView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let runtime = self.runtime.clone();
        let object = self.object.clone();
        let entity = cx.entity();
        runtime.render_view(object, entity, window, cx)
    }
}
