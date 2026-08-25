//! Pieces shared by every scripting engine.
//!
//! Callback storage and the failure surface are the same problem whatever the
//! VM is: handlers belong to exactly one render snapshot, and a script error has
//! to land on screen rather than take the host down. Only the type of the stored
//! handler differs, so it is a type parameter.

use std::{
    cell::RefCell,
    path::{Path, PathBuf},
    rc::Rc,
};

use anyhow::{Result, anyhow};
use gpui::{
    AnyElement, App, ClipboardItem, Entity, Hsla, InteractiveElement, IntoElement, ParentElement,
    SharedString, Styled, Window, div, px, relative, rems, rgb,
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
///
/// `pub(crate)` throughout: an engine builds one and the dispatcher reads it,
/// and nothing outside this crate has any use for either half. Narrowing it is
/// what keeps a later field from being a breaking change to somebody else.
pub(crate) struct CallbackEntry<T> {
    pub(crate) value: T,
    pub(crate) view: Option<Entity<ScriptView>>,
}

impl<T: Clone> Clone for CallbackEntry<T> {
    fn clone(&self) -> Self {
        Self {
            value: self.value.clone(),
            view: self.view.clone(),
        }
    }
}

/// Callbacks live for exactly as long as the snapshot that produced them.
///
/// A script render publishes one [`crate::snapshot::RenderSnapshot`], and that
/// snapshot may be materialized by many GPUI frames. So a handler cannot be
/// retired when a frame ends — only when the snapshot it belongs to is dropped,
/// which is what [`retire`](Self::retire) does.
///
/// Building is staged: [`begin`](Self::begin) opens a generation, handlers
/// accumulate into it, and it becomes reachable only on
/// [`commit`](Self::commit). A script render that fails half-way calls
/// [`abort`](Self::abort) instead, so a failed build leaves no trace — the same
/// transactional rule the snapshot itself follows.
pub(crate) struct CallbackArena<T> {
    next_generation: u32,
    /// The generation currently being recorded, if a build is open.
    building: Option<(u32, Vec<CallbackEntry<T>>)>,
    /// Committed generations, one per live snapshot. A view keeps two — the
    /// published snapshot and the one it just replaced — so this stays short
    /// enough that a scan beats a map.
    live: Vec<(u32, Vec<CallbackEntry<T>>)>,
}

impl<T> Default for CallbackArena<T> {
    fn default() -> Self {
        Self {
            next_generation: 0,
            building: None,
            live: Vec::new(),
        }
    }
}

impl<T: Clone> CallbackArena<T> {
    /// Opens a generation. Any generation left open by an earlier failed build
    /// is discarded rather than committed.
    pub(crate) fn begin(&mut self) -> u32 {
        let generation = self.next_generation & GENERATION_MASK;
        self.next_generation = self.next_generation.wrapping_add(1);
        self.building = Some((generation, Vec::new()));
        generation
    }

    /// Publishes the open generation, so its handlers become callable.
    pub(crate) fn commit(&mut self) {
        if let Some(entry) = self.building.take() {
            self.live.push(entry);
        }
    }

    /// Drops the open generation. A failed script render must not leave
    /// callable handlers behind.
    pub(crate) fn abort(&mut self) {
        self.building = None;
    }

    /// Empties the open generation without closing it.
    ///
    /// The diagnostic retry runs the same render a second time to produce a
    /// better message; the second run must start from an empty index space
    /// rather than stack its handlers on the abandoned ones. The generation
    /// number survives, because the caller is already holding it.
    pub(crate) fn rollback(&mut self) {
        if let Some((_, entries)) = self.building.as_mut() {
            entries.clear();
        }
    }

    /// Releases the handlers of one committed generation, called when the
    /// snapshot that owns them is dropped.
    pub(crate) fn retire(&mut self, generation: u32) {
        self.live.retain(|(live, _)| *live != generation);
    }

    pub(crate) fn push(&mut self, entry: CallbackEntry<T>) -> CallbackId {
        let Some((generation, entries)) = self.building.as_mut() else {
            // Reached only if a handler is registered outside a script render,
            // which is a host bug. An id no lookup can match is the harmless
            // answer.
            tracing::error!("a callback was registered outside a snapshot build");
            return CallbackId::MAX;
        };
        let index = entries.len() as u32;
        entries.push(entry);
        (*generation << GENERATION_SHIFT) | (index & INDEX_MASK)
    }

    pub(crate) fn get(&self, id: CallbackId) -> Option<CallbackEntry<T>> {
        let generation = id >> GENERATION_SHIFT;
        let index = (id & INDEX_MASK) as usize;
        self.live
            .iter()
            .find(|(live, _)| *live == generation)
            .and_then(|(_, entries)| entries.get(index))
            .cloned()
    }

    /// Releases every stored handler.
    ///
    /// Engines whose values must outlive nothing — QuickJS `Persistent` handles
    /// in particular — call this before tearing the VM down, because a handle
    /// released after its runtime aborts the process.
    pub(crate) fn clear(&mut self) {
        self.building = None;
        self.live.clear();
    }
}

/// A [`CallbackId`] packs the generation into its high bits and the index into
/// its low ones, so a handler from a retired snapshot resolves to `None`
/// instead of to whatever now sits at that index.
const GENERATION_SHIFT: u32 = 16;
const INDEX_MASK: u32 = 0xffff;
const GENERATION_MASK: u32 = 0xffff;

/// What `process.exit(code)` means, decided by the host.
///
/// The script only ever *asks*: one plugin must not be able to take down an
/// application somebody is working in, so the runtime never calls `exit(2)`
/// itself. But a request nobody answers is a lie told in the flattering
/// direction — the script gets a success and nothing happens — so a host that
/// grants the capability installs what it should do, and a host that grants it
/// without installing one is told so at the call rather than never.
///
/// A standalone CLI ends the process. An embedded host might close the plugin's
/// panel, or its window, or refuse.
pub type ExitHandler = Rc<dyn Fn(i32, &mut Window, &mut App)>;

thread_local! {
    static EXIT_HANDLER: RefCell<Option<ExitHandler>> = const { RefCell::new(None) };
}

/// Installs what an exit request does. Replaces any previous handler.
pub fn on_exit_request(handler: impl Fn(i32, &mut Window, &mut App) + 'static) {
    EXIT_HANDLER.with(|installed| *installed.borrow_mut() = Some(Rc::new(handler)));
}

/// The installed handler, if the host installed one.
///
/// Cloned out rather than borrowed across the call: a handler that reinstalls
/// itself — or that tears down a panel which drops something that does — would
/// otherwise panic on the outstanding borrow.
pub fn exit_handler() -> Option<ExitHandler> {
    EXIT_HANDLER.with(|installed| installed.borrow().clone())
}

/// Forgets the installed handler. A host that goes away should not leave one
/// behind pointing at state it owned.
pub fn clear_exit_handler() {
    EXIT_HANDLER.with(|installed| *installed.borrow_mut() = None);
}

/// A failure reported over an interface that still works.
///
/// A render that throws does not take the last valid description with it — the
/// snapshot is only replaced after a build succeeds — so there is usually still
/// a working interface to show. Blanking it would lose the reader's scroll
/// position, their focus, and whatever they were reading, in exchange for a
/// message that fits in a strip.
///
/// So the strip is what they get: it sits over the interface, says what broke
/// and what to do, and hands over the detail for pasting elsewhere. The
/// interface underneath is one render behind, which the banner says out loud
/// rather than leaving the reader to discover.
pub fn error_banner(message: &str, window: &mut Window, cx: &mut App) -> AnyElement {
    let surface = token("surface", rgb(0x171d26).into());
    let foreground = token("foreground", rgb(0xe6ebf2).into());
    let muted = token("muted_foreground", rgb(0x93a1b3).into());
    let border = token("border", rgb(0x2a3240).into());
    let accent = token("destructive", rgb(0xd05050).into());

    let copied =
        window.use_keyed_state(SharedString::from("shell-banner-copied"), cx, |_, _| false);
    let is_copied = copied.read(cx).to_owned();
    let payload = format!("This view could not be re-rendered\n\n{message}");

    div()
        .absolute()
        .top_0()
        .left_0()
        .right_0()
        .flex()
        .flex_col()
        .bg(surface)
        .border_b_1()
        .border_color(border)
        .child(div().h(px(2.)).w_full().bg(accent))
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .gap(rems(1.))
                .px(rems(1.))
                .py(rems(0.625))
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(rems(0.125))
                        .child(
                            div()
                                .text_size(rems(0.75))
                                .line_height(relative(1.4))
                                .text_color(foreground)
                                .child(SharedString::from(
                                    "This view could not be re-rendered; showing the last \
                                     version that worked",
                                )),
                        )
                        .child(
                            div()
                                .text_size(rems(0.6875))
                                .line_height(relative(1.45))
                                .text_color(muted)
                                .child(SharedString::from(first_line(message))),
                        ),
                )
                .child(copy_button(
                    copied, is_copied, payload, foreground, border, muted,
                )),
        )
        .into_any_element()
}

/// A banner has one line for the detail, so it shows the first one and the copy
/// action carries the rest. A stack trace truncated mid-frame reads as noise.
fn first_line(message: &str) -> String {
    message.lines().next().unwrap_or(message).to_owned()
}

/// A visible, non-fatal failure surface.
///
/// Used when there is nothing to keep — a view whose very first render failed
/// has no last good interface to put a banner over. The message belongs where
/// the interface was supposed to be.
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
        .p(rems(2.))
        .child(
            div()
                .flex()
                .flex_col()
                .w_full()
                .max_w(rems(35.))
                .bg(surface)
                .border_1()
                .border_color(border)
                .child(div().h(px(2.)).w_full().bg(accent))
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(rems(0.75))
                        .p(rems(1.5))
                        .child(
                            div()
                                .text_size(rems(1.))
                                .line_height(relative(1.4))
                                .text_color(foreground)
                                .child(SharedString::from(heading.to_owned())),
                        )
                        .child(
                            div()
                                .text_size(rems(0.8125))
                                .line_height(relative(1.55))
                                .text_color(muted)
                                .child(SharedString::from(message.to_owned())),
                        )
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .justify_between()
                                .gap(rems(1.))
                                .pt(rems(0.75))
                                .border_t_1()
                                .border_color(border)
                                .child(
                                    div()
                                        .text_size(rems(0.75))
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
        .h(rems(1.625))
        .px(rems(0.75))
        .border_1()
        .border_color(border)
        .text_size(rems(0.75))
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
