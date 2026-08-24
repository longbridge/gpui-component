//! Pieces shared by every scripting engine.
//!
//! Callback storage and the failure surface are the same problem whatever the
//! VM is: handlers belong to exactly one render pass, and a script error has to
//! land on screen rather than take the host down. Only the type of the stored
//! handler differs, so it is a type parameter.

use std::path::{Path, PathBuf};

use anyhow::{Result, anyhow};
use gpui::{
    AnyElement, App, ClipboardItem, Entity, Hsla, InteractiveElement, IntoElement, ParentElement,
    SharedString, Styled, Window, div, px, relative, rgb,
};
use gpui_base::Button;

use crate::{spec::CallbackId, view::ScriptView};

/// Finds the directory an application is rooted at.
///
/// Being pointed at the entry file itself, or at the parent of the real
/// application directory, is the most common way to start — so both are handled
/// here rather than failing with a bare "no such file". The error tells the
/// author what was expected and, when it can tell, where the application
/// actually is.
pub fn resolve_app_root(path: &Path, entry: &str) -> Result<PathBuf> {
    let candidate = if path.is_file() {
        path.parent().map(Path::to_path_buf).unwrap_or_default()
    } else {
        path.to_path_buf()
    };

    if !candidate.exists() {
        return Err(anyhow!("`{}` does not exist", path.display()));
    }

    let root = candidate
        .canonicalize()
        .map_err(|error| anyhow!("cannot read `{}`: {error}", candidate.display()))?;

    if root.join(entry).is_file() {
        return Ok(root);
    }

    Err(anyhow!("{}", missing_entry_message(&root, entry)))
}

fn missing_entry_message(root: &Path, entry: &str) -> String {
    let mut message = format!(
        "no `{entry}` in {}

An application directory must contain {entry},          which default-exports a view class.",
        root.display()
    );

    let nested: Vec<PathBuf> = std::fs::read_dir(root)
        .into_iter()
        .flatten()
        .flatten()
        .map(|item| item.path())
        .filter(|path| path.join(entry).is_file())
        .collect();

    match nested.as_slice() {
        [] => {}
        [only] => {
            message.push_str(&format!(
                "

Did you mean `{}`?",
                only.display()
            ));
        }
        several => {
            message.push_str(
                "

Applications found below this directory:",
            );
            for path in several {
                message.push_str(&format!(
                    "
  {}",
                    path.display()
                ));
            }
        }
    }

    message
}

/// A script callback together with the view it was registered from. The view is
/// what a later notify has to reach.
pub struct CallbackEntry<T> {
    pub value: T,
    pub view: Option<Entity<ScriptView>>,
}

impl<T: Clone> Clone for CallbackEntry<T> {
    fn clone(&self) -> Self {
        Self {
            value: self.value.clone(),
            view: self.view.clone(),
        }
    }
}

/// Callbacks live for exactly one render pass.
///
/// The previous pass is kept one generation longer because an event can be
/// dispatched between a render and its paint; the generation stored in each id
/// is what keeps a stale handler from being called by mistake.
pub struct CallbackArena<T> {
    generation: u32,
    current: Vec<CallbackEntry<T>>,
    previous: Vec<CallbackEntry<T>>,
    previous_generation: u32,
}

impl<T> Default for CallbackArena<T> {
    fn default() -> Self {
        Self {
            generation: 0,
            current: Vec::new(),
            previous: Vec::new(),
            previous_generation: u32::MAX,
        }
    }
}

impl<T: Clone> CallbackArena<T> {
    pub fn push(&mut self, entry: CallbackEntry<T>) -> CallbackId {
        let index = self.current.len() as u32;
        self.current.push(entry);
        (self.generation << 16) | (index & 0xffff)
    }

    pub fn get(&self, id: CallbackId) -> Option<CallbackEntry<T>> {
        let generation = id >> 16;
        let index = (id & 0xffff) as usize;
        if generation == self.generation {
            self.current.get(index).cloned()
        } else if generation == self.previous_generation {
            self.previous.get(index).cloned()
        } else {
            None
        }
    }

    /// Releases every stored handler.
    ///
    /// Engines whose values must outlive nothing — QuickJS `Persistent` handles
    /// in particular — call this before tearing the VM down, because a handle
    /// released after its runtime aborts the process.
    pub fn clear(&mut self) {
        self.current.clear();
        self.previous.clear();
    }

    pub fn swap(&mut self) {
        self.previous = std::mem::take(&mut self.current);
        self.previous_generation = self.generation;
        self.generation = self.generation.wrapping_add(1) & 0xffff;
    }
}

/// A visible, non-fatal failure surface.
///
/// A script error must never blank the window with no explanation: the message
/// belongs where the interface was supposed to be.
pub fn error_overlay(message: &str, window: &mut Window, cx: &mut App) -> AnyElement {
    failure_surface(
        "This view could not be rendered",
        message,
        "Fix the script and save; the view re-renders on the next change.",
        window,
        cx,
    )
}

/// The one place a failure becomes an interface.
///
/// Design Guides asks an error to say what happened and what to do next, and to
/// take its colors from semantic roles rather than literals — a failure surface
/// that hardcodes red is unreadable in half the themes it will be seen in. So
/// this is a normal composed surface: one heading, the detail, one recovery
/// line, on the same tokens every other screen uses. `destructive` appears once,
/// as a hairline rule, because emphasis is a budget and the message itself is
/// already the focal point.
///
/// The panel has square corners on purpose: it is not a card floating in the
/// window, it *is* the window's content for as long as the failure lasts.
///
/// A stack trace exists to be pasted somewhere else, so copying it is a first
/// class action rather than something the reader retypes.
pub fn failure_surface(
    heading: &str,
    message: &str,
    recovery: &str,
    window: &mut Window,
    cx: &mut App,
) -> AnyElement {
    let background = token("background", rgb(0x11161d).into());
    let surface = token("surface", rgb(0x171d26).into());
    let foreground = token("foreground", rgb(0xe6ebf2).into());
    let muted = token("muted_foreground", rgb(0x93a1b3).into());
    let border = token("border", rgb(0x2a3240).into());
    let accent = token("destructive", rgb(0xd05050).into());

    let copied =
        window.use_keyed_state(SharedString::from("shell-failure-copied"), cx, |_, _| false);
    let is_copied = copied.read(cx).to_owned();
    let payload = format!("{heading}\n\n{message}");

    div()
        .size_full()
        .flex()
        .items_center()
        .justify_center()
        .bg(background)
        .p(px(32.))
        .child(
            div()
                .flex()
                .flex_col()
                .w_full()
                .max_w(px(560.))
                .bg(surface)
                .border_1()
                .border_color(border)
                .child(div().h(px(2.)).w_full().bg(accent))
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(12.))
                        .p(px(24.))
                        .child(
                            div()
                                .text_size(px(16.))
                                .line_height(relative(1.4))
                                .text_color(foreground)
                                .child(SharedString::from(heading.to_owned())),
                        )
                        .child(
                            div()
                                .text_size(px(13.))
                                .line_height(relative(1.55))
                                .text_color(muted)
                                .child(SharedString::from(message.to_owned())),
                        )
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .justify_between()
                                .gap(px(16.))
                                .pt(px(12.))
                                .border_t_1()
                                .border_color(border)
                                .child(
                                    div()
                                        .text_size(px(12.))
                                        .line_height(relative(1.5))
                                        .text_color(muted)
                                        .child(SharedString::from(recovery.to_owned())),
                                )
                                .child(copy_button(
                                    copied, is_copied, payload, foreground, border, muted,
                                )),
                        ),
                ),
        )
        .into_any_element()
}

/// Copies the failure, and says so — a copy leaves no visible trace otherwise,
/// which is exactly when confirmation is worth its space.
fn copy_button(
    state: Entity<bool>,
    copied: bool,
    payload: String,
    foreground: Hsla,
    border: Hsla,
    muted: Hsla,
) -> AnyElement {
    Button::new("shell-failure-copy")
        .flex()
        .items_center()
        .justify_center()
        .h(px(26.))
        .px(px(12.))
        .border_1()
        .border_color(border)
        .text_size(px(12.))
        .line_height(relative(1.))
        .text_color(if copied { muted } else { foreground })
        .hover(|style| style.opacity(0.8))
        .on_click(move |_, _, cx| {
            cx.write_to_clipboard(ClipboardItem::new_string(payload.clone()));
            state.update(cx, |copied, cx| {
                *copied = true;
                cx.notify();
            });
        })
        .child(SharedString::from(if copied {
            "Copied"
        } else {
            "Copy details"
        }))
        .into_any_element()
}

/// Semantic token with a fallback, because a failure surface must render even
/// when the failure is that the theme never got installed.
fn token(name: &str, fallback: Hsla) -> Hsla {
    crate::theme::token_color(name).unwrap_or(fallback)
}
