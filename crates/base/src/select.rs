use std::{
    rc::Rc,
    time::{Duration, Instant},
};

use gpui::{
    AccessibleAction, AnyElement, App, ElementId, FocusHandle, InteractiveElement as _,
    IntoElement, KeyBinding, KeyDownEvent, MouseMoveEvent, ParentElement, Pixels, Point,
    RenderOnce, Role, ScrollHandle, SharedString, StatefulInteractiveElement as _, StyleRefinement,
    Styled, Window, actions, canvas, div, prelude::FluentBuilder as _, px,
};

use crate::actions::{
    Cancel, Confirm, SelectDown, SelectFirst, SelectLast, SelectPageDown, SelectPageUp, SelectUp,
};
use crate::input::{MoveDown, MovePageDown, MovePageUp, MoveUp};
use crate::root::{Tab, TabPrev};
use crate::{StyledExt as _, TestSupportExt as _, a11y_disabled::A11yDisabledExt as _};

const CONTEXT: &str = "Select";

actions!(
    select,
    [
        /// Space in a select: a letter of a typeahead search in progress,
        /// and Confirm otherwise.
        Space
    ]
);

#[doc(hidden)]
pub fn init(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("space", Space, Some("Select && !Input")),
        KeyBinding::new("up", SelectUp, Some(CONTEXT)),
        KeyBinding::new("down", SelectDown, Some(CONTEXT)),
        KeyBinding::new("home", SelectFirst, Some(CONTEXT)),
        KeyBinding::new("end", SelectLast, Some(CONTEXT)),
        KeyBinding::new("pageup", SelectPageUp, Some(CONTEXT)),
        KeyBinding::new("pagedown", SelectPageDown, Some(CONTEXT)),
        KeyBinding::new("enter", Confirm { secondary: false }, Some(CONTEXT)),
        KeyBinding::new(
            "secondary-enter",
            Confirm { secondary: true },
            Some(CONTEXT),
        ),
        KeyBinding::new("escape", Cancel, Some(CONTEXT)),
    ]);
}

type OpenChangeHandler = Rc<dyn Fn(bool, &mut Window, &mut App)>;
type ActionHandler = Rc<dyn Fn(&mut Window, &mut App)>;
type HighlightHandler = Rc<dyn Fn(usize, &mut Window, &mut App)>;

/// How long typed letters keep adding to one typeahead search.
const TYPEAHEAD_TIMEOUT: Duration = Duration::from_secs(1);

/// A keyboard move of the highlighted option.
#[derive(Clone, Copy)]
enum Step {
    Previous,
    Next,
    PreviousPage,
    NextPage,
    First,
    Last,
}

/// The option list a select navigates, as the application supplied it.
struct Options {
    labels: Rc<[SharedString]>,
    selected_ix: Option<usize>,
    highlighted_ix: Option<usize>,
    on_highlight: Option<HighlightHandler>,
    scroll_handle: Option<ScrollHandle>,
}

impl Options {
    /// The option a list opening with `step` highlights: the selected one for
    /// an arrow, as a macOS pop-up button opens on its selection.
    fn opening_target(&self, step: Step) -> Option<usize> {
        let last = self.labels.len().checked_sub(1)?;
        Some(match step {
            Step::Previous | Step::Next | Step::PreviousPage | Step::NextPage => {
                self.selected_ix.unwrap_or(0).min(last)
            }
            Step::First => 0,
            Step::Last => last,
        })
    }

    /// The option `step` moves to from the highlighted one, or from the
    /// selected one while nothing is highlighted. Moves stop at either end.
    fn target(&self, step: Step) -> Option<usize> {
        let last = self.labels.len().checked_sub(1)?;
        let current = self.highlighted_ix.or(self.selected_ix);
        Some(match step {
            Step::Previous => current.map_or(last, |ix| ix.saturating_sub(1)),
            Step::Next => current.map_or(0, |ix| (ix + 1).min(last)),
            Step::PreviousPage => current.map_or(last, |ix| ix.saturating_sub(self.page())),
            Step::NextPage => current.map_or(0, |ix| (ix + self.page()).min(last)),
            Step::First => 0,
            Step::Last => last,
        })
    }

    /// How many options a page key moves by: as many as the list shows, as
    /// last laid out, or all of them without a scroll handle to tell.
    fn page(&self) -> usize {
        self.scroll_handle
            .as_ref()
            .and_then(|scroll_handle| {
                let row = scroll_handle.bounds_for_item(0)?.size.height;
                (row > px(0.)).then(|| (scroll_handle.bounds().size.height / row).floor() as usize)
            })
            .unwrap_or(self.labels.len())
            .max(1)
    }

    fn highlight(&self, ix: usize, window: &mut Window, cx: &mut App) {
        if let Some(scroll_handle) = self.scroll_handle.as_ref() {
            scroll_handle.scroll_to_item(ix);
        }
        if let Some(handler) = self.on_highlight.as_ref() {
            handler(ix, window, cx);
        }
    }

    /// The option under `position` in the list tracking the scroll handle,
    /// as last laid out: inside the list's visible bounds and one option's.
    fn option_at(&self, position: Point<Pixels>) -> Option<usize> {
        let scroll_handle = self.scroll_handle.as_ref()?;
        if !scroll_handle.bounds().contains(&position) {
            return None;
        }
        // Option bounds are laid out before the list scrolls them.
        let offset = scroll_handle.offset();
        (0..self.labels.len())
            .map_while(|ix| Some((ix, scroll_handle.bounds_for_item(ix)?)))
            .find(|(_, bounds)| {
                let mut bounds = *bounds;
                bounds.origin += offset;
                bounds.contains(&position)
            })
            .map(|(ix, _)| ix)
    }
}

/// The letters typed in quick succession to find an option by its label.
#[derive(Default)]
struct Typeahead {
    query: String,
    typed_at: Option<Instant>,
    /// Whether the list was open: closing it ends the search, while a
    /// search that opened it goes on in the open list.
    open: bool,
}

impl Typeahead {
    /// Whether a search is going on at `now`: a space then belongs to it.
    fn is_searching(&self, now: Instant) -> bool {
        !self.query.is_empty()
            && self
                .typed_at
                .is_some_and(|typed_at| now.duration_since(typed_at) <= TYPEAHEAD_TIMEOUT)
    }

    fn sync(&mut self, open: bool) {
        if self.open && !open {
            self.query.clear();
        }
        self.open = open;
    }

    /// Adds `text` typed at `now`, starting over after a pause.
    fn push(&mut self, text: &str, now: Instant) -> &str {
        if self
            .typed_at
            .is_none_or(|typed_at| now.duration_since(typed_at) > TYPEAHEAD_TIMEOUT)
        {
            self.query.clear();
        }
        self.typed_at = Some(now);
        self.query.push_str(&text.to_lowercase());
        &self.query
    }
}

/// The option whose label starts with `query`, ignoring case, searching
/// from `current` and wrapping around. Repeating one letter steps through the
/// options starting with it, as Windows and macOS lists do.
fn typeahead_match(labels: &[SharedString], query: &str, current: Option<usize>) -> Option<usize> {
    let first = query.chars().next()?;
    let (prefix, start) = if query.chars().all(|ch| ch == first) {
        (&query[..first.len_utf8()], current.map_or(0, |ix| ix + 1))
    } else {
        (query, current.unwrap_or(0))
    };
    let count = labels.len();
    (0..count)
        .map(|offset| (start + offset) % count)
        .find(|&ix| labels[ix].to_lowercase().starts_with(prefix))
}

/// An unstyled controlled select root.
///
/// Applications own the trigger and popup presentation, the option collection,
/// and the selected value. This root owns combobox accessibility semantics,
/// keyboard opening and dismissal, and focus transfer between the trigger and
/// popup content.
///
/// GPUI marks the active option on the option element itself rather than on the
/// container, so the application marks its highlighted option with
/// `aria_active_descendant()`; this root cannot do it on the caller's behalf.
///
/// Tab and Shift-Tab close an open select without choosing, as Escape does,
/// and move the focus on from the trigger.
///
/// A select that supplies its [`options`](Self::options) lets this root own
/// the list's keyboard navigation too: the arrow keys, Home, End, Page Up and
/// Page Down move the highlighted option, and typing the start of a label
/// highlights it. The
/// application keeps the highlighted index, as it keeps the open state, and
/// commits it from [`on_confirm`](Self::on_confirm).
#[derive(IntoElement)]
pub struct Select {
    id: ElementId,
    base: crate::ObservedElement<gpui::Stateful<gpui::Div>>,
    open: bool,
    disabled: bool,
    focus_handle: Option<FocusHandle>,
    content_focus_handle: Option<FocusHandle>,
    accessibility_label: Option<SharedString>,
    accessibility_value: Option<SharedString>,
    style: StyleRefinement,
    children: Vec<AnyElement>,
    on_open_change: Option<OpenChangeHandler>,
    key_context: &'static str,
    on_dismiss: Option<ActionHandler>,
    on_confirm: Option<ActionHandler>,
    options: Option<Rc<[SharedString]>>,
    selected_ix: Option<usize>,
    highlighted_ix: Option<usize>,
    on_highlight: Option<HighlightHandler>,
    scroll_handle: Option<ScrollHandle>,
    typeahead: bool,
}

impl Select {
    pub fn new(id: impl Into<ElementId>) -> Self {
        let id = id.into();
        Self {
            base: div().id(id.clone()).test_support(),
            id,
            open: false,
            disabled: false,
            focus_handle: None,
            content_focus_handle: None,
            accessibility_label: None,
            accessibility_value: None,
            style: StyleRefinement::default(),
            children: Vec::new(),
            on_open_change: None,
            key_context: CONTEXT,
            on_dismiss: None,
            on_confirm: None,
            options: None,
            selected_ix: None,
            highlighted_ix: None,
            on_highlight: None,
            scroll_handle: None,
            typeahead: true,
        }
    }

    /// Sets the application-controlled open state.
    pub fn open(mut self, open: bool) -> Self {
        self.open = open;
        self
    }

    /// Prevents keyboard and accessible activation and removes the trigger from tab traversal.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Supplies the focus handle for the select trigger.
    pub fn focus_handle(mut self, focus_handle: &FocusHandle) -> Self {
        self.focus_handle = Some(focus_handle.clone());
        self
    }

    /// Supplies the focus handle that receives keyboard navigation while open.
    pub fn content_focus_handle(mut self, focus_handle: &FocusHandle) -> Self {
        self.content_focus_handle = Some(focus_handle.clone());
        self
    }

    /// Sets the accessible name exposed by the controlled root.
    pub fn accessibility_label(mut self, label: impl Into<SharedString>) -> Self {
        self.accessibility_label = Some(label.into());
        self
    }

    /// Sets the committed value exposed by the controlled root.
    ///
    /// Supply a readable selection title, not the current search query or cursor.
    pub fn accessibility_value(mut self, value: impl Into<SharedString>) -> Self {
        self.accessibility_value = Some(value.into());
        self
    }

    /// Handles requests to update the controlled open state.
    pub fn on_open_change(
        mut self,
        handler: impl Fn(bool, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_open_change = Some(Rc::new(handler));
        self
    }

    #[doc(hidden)]
    pub fn key_context(mut self, key_context: &'static str) -> Self {
        self.key_context = key_context;
        self
    }

    /// Handles a dismissal, however it was requested: the Cancel action, or
    /// the accessible activation that closes an open control.
    ///
    /// This runs before the controlled open state is asked to close, so a
    /// caller that commits its pending value on dismissal can still read that
    /// value here.
    pub fn on_dismiss(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_dismiss = Some(Rc::new(handler));
        self
    }

    /// Handles the Confirm action while the select is open.
    ///
    /// Confirming a closed select opens it instead, so this never runs for
    /// that case. The content has the focus when this runs, so a handler that
    /// commits and closes the list can move the focus back to the trigger.
    pub fn on_confirm(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_confirm = Some(Rc::new(handler));
        self
    }

    /// Supplies the labels of the options the open list shows, in order.
    ///
    /// With options, the arrow keys, Home, End, Page Up and Page Down move the
    /// highlighted option (opening a closed select first; a page is as many
    /// options as the list shows), and typing the start of a label highlights
    /// the next option it matches. Space goes on with a search typed within
    /// the last second, and activates otherwise. Without options, those keys
    /// reach the focused popup content, which navigates on its own.
    pub fn options(mut self, labels: impl IntoIterator<Item = impl Into<SharedString>>) -> Self {
        self.options = Some(labels.into_iter().map(Into::into).collect());
        self
    }

    /// Sets the index of the option holding the committed value.
    ///
    /// Opening with an arrow key highlights it, and moves start from it while
    /// nothing is highlighted.
    pub fn selected_index(mut self, ix: Option<usize>) -> Self {
        self.selected_ix = ix;
        self
    }

    /// Sets the application-controlled index of the highlighted option: the
    /// one the keyboard is on, which Confirm commits.
    pub fn highlighted_index(mut self, ix: Option<usize>) -> Self {
        self.highlighted_ix = ix;
        self
    }

    /// Handles requests to highlight the option at an index.
    pub fn on_highlight(
        mut self,
        handler: impl Fn(usize, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_highlight = Some(Rc::new(handler));
        self
    }

    /// Scrolls the highlighted option into view in the element tracking this
    /// handle, whose children must be the options in order.
    ///
    /// With options and this handle, the pointer moving over an option of the
    /// open list highlights it too, through [`on_highlight`](Self::on_highlight).
    pub fn scroll_handle(mut self, scroll_handle: &ScrollHandle) -> Self {
        self.scroll_handle = Some(scroll_handle.clone());
        self
    }

    #[doc(hidden)]
    pub fn typeahead(mut self, typeahead: bool) -> Self {
        self.typeahead = typeahead;
        self
    }
}

impl Styled for Select {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl ParentElement for Select {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl RenderOnce for Select {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let open = self.open;
        let disabled = self.disabled;
        let focus_handle = self.focus_handle;
        let content_focus_handle = self.content_focus_handle;
        let on_open_change = self.on_open_change;
        let on_dismiss = self.on_dismiss;
        let on_confirm = self.on_confirm;
        let options = self.options.map(|labels| {
            Rc::new(Options {
                labels,
                selected_ix: self.selected_ix,
                highlighted_ix: self.highlighted_ix,
                on_highlight: self.on_highlight,
                scroll_handle: self.scroll_handle,
            })
        });
        let typeahead = (self.typeahead && !disabled)
            .then_some(options.clone())
            .flatten()
            .map(|options| {
                let state = window.use_keyed_state((self.id.clone(), "typeahead"), cx, |_, _| {
                    Typeahead::default()
                });
                state.update(cx, |state, _| state.sync(open));
                (options, state)
            });

        // Every way of closing runs the same steps. A caller that tracks
        // dismissal has to see one however the popup was closed, and the
        // accessible activation closes exactly what Escape closes.
        let close: ActionHandler = Rc::new({
            let on_open_change = on_open_change.clone();
            let on_dismiss = on_dismiss.clone();
            let focus_handle = focus_handle.clone();
            move |window: &mut Window, cx: &mut App| {
                if let Some(handler) = on_dismiss.as_ref() {
                    handler(window, cx);
                }
                if let Some(handler) = on_open_change.as_ref() {
                    handler(false, window, cx);
                }
                if let Some(handle) = focus_handle.as_ref() {
                    handle.focus(window, cx);
                }
            }
        });
        // Every way of opening asks to open and gives the content the focus.
        let request_open: ActionHandler = Rc::new({
            let on_open_change = on_open_change.clone();
            let content_focus_handle = content_focus_handle.clone();
            move |window: &mut Window, cx: &mut App| {
                if let Some(handler) = on_open_change.as_ref() {
                    handler(true, window, cx);
                }
                if let Some(handle) = content_focus_handle.as_ref() {
                    handle.focus(window, cx);
                }
            }
        });
        // Typed text extends the typeahead search and highlights its match,
        // opening a closed select.
        let type_text: Option<Rc<dyn Fn(&str, &mut Window, &mut App)>> =
            typeahead.clone().map(|(options, typeahead)| {
                let request_open = request_open.clone();
                Rc::new(move |text: &str, window: &mut Window, cx: &mut App| {
                    let current = options.highlighted_ix.or(options.selected_ix);
                    let Some(ix) = typeahead.update(cx, |typeahead, _| {
                        typeahead_match(
                            &options.labels,
                            typeahead.push(text, Instant::now()),
                            current,
                        )
                    }) else {
                        return;
                    };
                    if !open {
                        request_open(window, cx);
                    }
                    options.highlight(ix, window, cx);
                }) as Rc<dyn Fn(&str, &mut Window, &mut App)>
            });
        // An arrow, Home or End. With options, it moves the highlight, opening
        // a closed select first. Without them, an arrow opens a closed select
        // and focuses the content, and every key goes on to the content.
        let step = |step: Step| {
            let options = options.clone();
            let request_open = request_open.clone();
            let content_focus_handle = content_focus_handle.clone();
            move |window: &mut Window, cx: &mut App| {
                if disabled {
                    cx.propagate();
                    return;
                }
                let Some(options) = options.as_ref() else {
                    if matches!(step, Step::Previous | Step::Next) {
                        if !open {
                            request_open(window, cx);
                        } else if let Some(handle) = content_focus_handle.as_ref() {
                            handle.focus(window, cx);
                        }
                    }
                    cx.propagate();
                    return;
                };
                let target = if open {
                    options.target(step)
                } else {
                    request_open(window, cx);
                    options.opening_target(step)
                };
                if let Some(ix) = target {
                    options.highlight(ix, window, cx);
                }
            }
        };

        self.base
            .role(Role::ComboBox)
            .aria_expanded(open)
            .when_some(self.accessibility_label, |this, label| {
                this.aria_label(label)
            })
            .when_some(self.accessibility_value, |this, value| {
                this.aria_value(value)
            })
            .key_context(self.key_context)
            .when_some(
                focus_handle.clone().filter(|_| !disabled),
                |this, handle| this.track_focus(&handle.tab_stop(true)),
            )
            .when(!disabled, |this| {
                let request_open = request_open.clone();
                let close = close.clone();

                // Platform adapters may flatten the trigger child.
                // Expose activation on the semantic root itself.
                this.on_a11y_action(AccessibleAction::Click, move |_, window, cx| {
                    if open {
                        close(window, cx);
                    } else {
                        request_open(window, cx);
                    }
                })
            })
            .on_action({
                let step = step(Step::Previous);
                move |_: &SelectUp, window, cx| step(window, cx)
            })
            .on_action({
                let step = step(Step::Next);
                move |_: &SelectDown, window, cx| step(window, cx)
            })
            .on_action({
                let step = step(Step::First);
                move |_: &SelectFirst, window, cx| step(window, cx)
            })
            .on_action({
                let step = step(Step::Last);
                move |_: &SelectLast, window, cx| step(window, cx)
            })
            .on_action({
                let step = step(Step::PreviousPage);
                move |_: &SelectPageUp, window, cx| step(window, cx)
            })
            .on_action({
                let step = step(Step::NextPage);
                move |_: &SelectPageDown, window, cx| step(window, cx)
            })
            // A query field inside an open list keeps the focus while the
            // arrows move the highlight, so they must not reach it as caret
            // moves: catch them on the way down.
            .when(open && !disabled && options.is_some(), |this| {
                let previous = step(Step::Previous);
                let next = step(Step::Next);
                let previous_page = step(Step::PreviousPage);
                let next_page = step(Step::NextPage);
                this.capture_action(move |_: &MoveUp, window, cx| {
                    previous(window, cx);
                    cx.stop_propagation();
                })
                .capture_action(move |_: &MoveDown, window, cx| {
                    next(window, cx);
                    cx.stop_propagation();
                })
                .capture_action(move |_: &MovePageUp, window, cx| {
                    previous_page(window, cx);
                    cx.stop_propagation();
                })
                .capture_action(move |_: &MovePageDown, window, cx| {
                    next_page(window, cx);
                    cx.stop_propagation();
                })
            })
            .when_some(type_text.clone(), |this, type_text| {
                let focus_handle = focus_handle.clone();
                let content_focus_handle = content_focus_handle.clone();
                this.on_key_down(move |event: &KeyDownEvent, window, cx| {
                    let keystroke = &event.keystroke;
                    let modifiers = keystroke.modifiers;
                    if modifiers.control
                        || modifiers.platform
                        || modifiers.function
                        || modifiers.alt
                    {
                        return;
                    }
                    let Some(text) = keystroke
                        .key_char
                        .as_deref()
                        .filter(|text| !text.is_empty() && !text.chars().any(char::is_control))
                    else {
                        return;
                    };
                    // Typing into a field inside the popup stays text.
                    let focused = |handle: &Option<FocusHandle>| {
                        handle
                            .as_ref()
                            .is_some_and(|handle| handle.is_focused(window))
                    };
                    if !focused(&focus_handle) && !focused(&content_focus_handle) {
                        return;
                    }
                    cx.stop_propagation();
                    type_text(text, window, cx);
                })
            })
            // Space goes on with a search being typed, and activates
            // otherwise, reaching the focused content as Confirm.
            .on_action({
                let typeahead = typeahead.as_ref().map(|(_, state)| state.clone());
                move |_: &Space, window, cx| {
                    if let (Some(type_text), Some(typeahead)) = (type_text.as_ref(), &typeahead)
                        && typeahead.read(cx).is_searching(Instant::now())
                    {
                        type_text(" ", window, cx);
                        return;
                    }
                    window.dispatch_action(Box::new(Confirm { secondary: false }), cx);
                }
            })
            // Tab leaves an open list as Escape does, choosing nothing, and
            // goes on to move the focus from the trigger.
            .when(open && !disabled, |this| {
                let close_on_tab = close.clone();
                let close_on_tab_prev = close.clone();
                this.capture_action(move |_: &Tab, window, cx| close_on_tab(window, cx))
                    .capture_action(move |_: &TabPrev, window, cx| close_on_tab_prev(window, cx))
            })
            .on_action({
                let on_open_change = on_open_change.clone();
                move |_: &Confirm, window, cx| {
                    if disabled {
                        cx.propagate();
                        return;
                    }

                    // The content takes the focus first, so a confirmation
                    // that closes the list can hand it back to the trigger.
                    if let Some(handle) = content_focus_handle.as_ref() {
                        handle.focus(window, cx);
                    }

                    if open {
                        if let Some(handler) = on_confirm.as_ref() {
                            handler(window, cx);
                        }
                    } else if let Some(handler) = on_open_change.as_ref() {
                        handler(true, window, cx);
                    }
                }
            })
            .on_action(move |_: &Cancel, window, cx| {
                if !open {
                    cx.propagate();
                    return;
                }

                cx.stop_propagation();
                close(window, cx);
            })
            .children(self.children)
            // The pointer moving over an option highlights it, as in a menu.
            // Only a move does: the list scrolling under a still pointer
            // leaves the keyboard's highlight alone.
            .when_some(
                options
                    .clone()
                    .filter(|options| open && !disabled && options.scroll_handle.is_some()),
                |this, options| {
                    this.child(
                        canvas(
                            |_, _, _| {},
                            move |_, _, window, _| {
                                window.on_mouse_event(
                                    move |event: &MouseMoveEvent, phase, window, cx| {
                                        if !phase.bubble() {
                                            return;
                                        }
                                        if let Some(ix) = options
                                            .option_at(event.position)
                                            .filter(|ix| options.highlighted_ix != Some(*ix))
                                            && let Some(handler) = options.on_highlight.as_ref()
                                        {
                                            handler(ix, window, cx);
                                        }
                                    },
                                );
                            },
                        )
                        .absolute()
                        .size_0(),
                    )
                },
            )
            .refine_style(&self.style)
            .a11y_disabled(disabled)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::InputState;
    use gpui::{
        AppContext as _, Context, Element as _, Entity, Focusable, Render, TestAppContext,
        VisualTestContext, accesskit, px,
    };
    use std::sync::{Arc, Mutex};

    struct SelectHarness {
        open: bool,
        disabled: bool,
        focus_handle: FocusHandle,
        content_focus_handle: FocusHandle,
        search_input: Option<Entity<InputState>>,
        changes: Arc<Mutex<Vec<bool>>>,
        parent_confirms: Arc<Mutex<usize>>,
        confirmations: Arc<Mutex<usize>>,
        /// Every step of a close, in the order it ran.
        closing: Arc<Mutex<Vec<&'static str>>>,
    }

    impl SelectHarness {
        fn new(disabled: bool, cx: &mut Context<Self>) -> Self {
            Self {
                open: false,
                disabled,
                focus_handle: cx.focus_handle(),
                content_focus_handle: cx.focus_handle(),
                search_input: None,
                changes: Arc::new(Mutex::new(Vec::new())),
                closing: Arc::new(Mutex::new(Vec::new())),
                parent_confirms: Arc::new(Mutex::new(0)),
                confirmations: Arc::new(Mutex::new(0)),
            }
        }
    }

    impl Focusable for SelectHarness {
        fn focus_handle(&self, _: &App) -> FocusHandle {
            self.focus_handle.clone()
        }
    }

    impl Render for SelectHarness {
        fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            let state = cx.entity();
            let changes = self.changes.clone();
            let opened = self.closing.clone();
            let dismissed = self.closing.clone();
            let confirmed = self.closing.clone();

            let parent_confirms = self.parent_confirms.clone();
            let confirmations = self.confirmations.clone();
            div()
                .on_action(move |_: &Confirm, _, _| *parent_confirms.lock().unwrap() += 1)
                .child(
                    Select::new("select")
                        .open(self.open)
                        .disabled(self.disabled)
                        .focus_handle(&self.focus_handle)
                        .content_focus_handle(&self.content_focus_handle)
                        .on_open_change(move |open, _, cx| {
                            changes.lock().unwrap().push(open);
                            opened
                                .lock()
                                .unwrap()
                                .push(if open { "open" } else { "close" });
                            state.update(cx, |state, cx| {
                                state.open = open;
                                cx.notify();
                            });
                        })
                        .on_dismiss(move |_, _| dismissed.lock().unwrap().push("dismiss"))
                        .on_confirm(move |_, _| {
                            confirmed.lock().unwrap().push("confirm");
                            *confirmations.lock().unwrap() += 1;
                        })
                        .when_some(self.search_input.clone(), |this, input| {
                            this.child(crate::Input::new(&input))
                        })
                        .when(self.search_input.is_none(), |this| {
                            this.child(div().track_focus(&self.content_focus_handle).size(px(20.)))
                        }),
                )
        }
    }

    fn harness(
        cx: &mut TestAppContext,
        disabled: bool,
    ) -> (&mut VisualTestContext, gpui::Entity<SelectHarness>) {
        cx.update(crate::init);
        let (state, cx) = cx.add_window_view(move |_, cx| SelectHarness::new(disabled, cx));
        cx.update(|window, cx| {
            state.focus_handle(cx).focus(window, cx);
            window.draw(cx).clear(cx);
        });
        (cx, state)
    }

    #[gpui::test]
    fn arrows_open_and_transfer_focus_to_content(cx: &mut TestAppContext) {
        let (cx, state) = harness(cx, false);

        cx.simulate_keystrokes("down");
        cx.update(|window, cx| {
            assert!(state.read(cx).open);
            assert!(state.read(cx).content_focus_handle.is_focused(window));
        });
        assert_eq!(
            &*state
                .read_with(cx, |state, _| state.changes.clone())
                .lock()
                .unwrap(),
            &[true]
        );
    }

    #[gpui::test]
    fn confirm_opens_a_closed_select(cx: &mut TestAppContext) {
        let (cx, state) = harness(cx, false);

        cx.simulate_keystrokes("enter");
        cx.update(|window, cx| {
            assert!(state.read(cx).open);
            assert!(state.read(cx).content_focus_handle.is_focused(window));
        });
    }

    #[gpui::test]
    fn space_opens_and_confirms_a_non_searchable_select(cx: &mut TestAppContext) {
        let (cx, state) = harness(cx, false);
        cx.simulate_keystrokes("space");
        cx.update(|window, cx| {
            assert!(state.read(cx).open);
            assert!(state.read(cx).content_focus_handle.is_focused(window));
            window.draw(cx).clear(cx);
        });
        cx.simulate_keystrokes("space");
        assert_eq!(
            &*state
                .read_with(cx, |state, _| state.closing.clone())
                .lock()
                .unwrap(),
            &["open", "confirm"]
        );
    }

    #[gpui::test]
    fn space_in_a_select_search_input_remains_text(cx: &mut TestAppContext) {
        let (cx, state) = harness(cx, false);
        let input = cx.update(|window, cx| {
            let input = cx.new(|cx| InputState::new(window, cx));
            state.update(cx, |state, cx| {
                state.content_focus_handle = input.focus_handle(cx);
                state.search_input = Some(input.clone());
                cx.notify();
            });
            window.draw(cx).clear(cx);
            input
        });
        cx.simulate_keystrokes("space");
        cx.update(|window, cx| {
            assert!(state.read(cx).open);
            assert!(input.focus_handle(cx).is_focused(window));
            window.draw(cx).clear(cx);
        });
        cx.simulate_keystrokes("t w o space w o r d s");
        assert_eq!(input.read_with(cx, |input, _| input.value()), "two words");
        assert_eq!(
            &*state
                .read_with(cx, |state, _| state.closing.clone())
                .lock()
                .unwrap(),
            &["open"]
        );
        assert!(state.read_with(cx, |state, _| state.open));
    }

    #[gpui::test]
    fn escape_closes_and_restores_trigger_focus(cx: &mut TestAppContext) {
        let (cx, state) = harness(cx, false);

        cx.simulate_keystrokes("down escape");
        cx.update(|window, cx| {
            assert!(!state.read(cx).open);
            assert!(state.read(cx).focus_handle.is_focused(window));
        });
        assert_eq!(
            &*state
                .read_with(cx, |state, _| state.changes.clone())
                .lock()
                .unwrap(),
            &[true, false]
        );
    }

    /// Closing runs `on_dismiss`, and runs it before the open state is asked
    /// to close, so a caller that commits a pending value on dismissal can
    /// still read that value.
    ///
    /// Every close shares one path, which is the point: the accessible
    /// activation used to close by calling `on_open_change` alone, so a
    /// consumer wiring `on_dismiss` — `crates/shell` forwards it to JS as
    /// `onDismiss` — saw Escape but not a screen reader pressing the same
    /// control. GPUI exposes no way to dispatch an accessibility action in a
    /// test (`Window::handle_a11y_action` is `pub(crate)`), so this covers the
    /// shared path through the route a test can reach.
    #[gpui::test]
    fn every_close_dismisses_before_it_closes(cx: &mut TestAppContext) {
        let (cx, state) = harness(cx, false);

        cx.simulate_keystrokes("down escape");
        assert_eq!(
            &*state
                .read_with(cx, |state, _| state.closing.clone())
                .lock()
                .unwrap(),
            &["open", "dismiss", "close"]
        );
    }

    #[gpui::test]
    fn disabled_select_is_not_keyboard_interactive(cx: &mut TestAppContext) {
        let (cx, state) = harness(cx, true);

        cx.simulate_keystrokes("down enter space");
        assert!(!state.read_with(cx, |state, _| state.open));
        assert!(
            state
                .read_with(cx, |state, _| state.changes.clone())
                .lock()
                .unwrap()
                .is_empty()
        );
    }

    #[gpui::test]
    fn projects_application_owned_accessible_state(cx: &mut TestAppContext) {
        let window = cx.add_empty_window();
        window.update(|window, cx| {
            let mut info = |select: Select| {
                let mut node = accesskit::Node::new(Role::ComboBox);
                select
                    .render(window, cx)
                    .into_element()
                    .write_a11y_info(&mut node);
                node
            };
            let enabled = info(
                Select::new("enabled")
                    .open(true)
                    .accessibility_label("Programming language")
                    .accessibility_value("Rust"),
            );
            // Open, so the expanded assertion below says something about
            // `disabled` rather than about the default open state.
            let disabled = info(Select::new("disabled").open(true).disabled(true));

            assert_eq!(enabled.label(), Some("Programming language"));
            assert_eq!(enabled.value(), Some("Rust"));
            assert_eq!(enabled.is_expanded(), Some(true));
            assert_eq!(
                disabled.is_expanded(),
                Some(true),
                "a disabled control still reports the state it is in"
            );
            assert!(enabled.supports_action(accesskit::Action::Click));
            assert!(!disabled.supports_action(accesskit::Action::Click));
        });
    }

    #[gpui::test]
    fn confirm_is_consumed_when_opening_and_accepting(cx: &mut TestAppContext) {
        let (cx, state) = harness(cx, false);
        cx.simulate_keystrokes("enter");
        cx.update(|window, cx| window.draw(cx).clear(cx));
        cx.simulate_keystrokes("enter");
        state.read_with(cx, |state, _| {
            assert!(state.open);
            assert_eq!(*state.confirmations.lock().unwrap(), 1);
            assert_eq!(*state.parent_confirms.lock().unwrap(), 0);
        });
    }
    /// A select whose root navigates its options.
    struct ListHarness {
        open: bool,
        selected: Option<usize>,
        highlighted: Option<usize>,
        focus_handle: FocusHandle,
        content_focus_handle: FocusHandle,
        scroll_handle: ScrollHandle,
        labels: &'static [&'static str],
        confirmed: Arc<Mutex<Vec<usize>>>,
    }

    const FRUITS: [&str; 5] = ["Apple", "Apricot", "Banana", "Blueberry", "Cherry"];

    impl Render for ListHarness {
        fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            let open_state = cx.entity();
            let highlight_state = cx.entity();
            let confirm_state = cx.entity();
            Select::new("select")
                .open(self.open)
                .focus_handle(&self.focus_handle)
                .content_focus_handle(&self.content_focus_handle)
                .options(self.labels.iter().copied())
                .selected_index(self.selected)
                .highlighted_index(self.highlighted)
                .on_open_change(move |open, _, cx| {
                    open_state.update(cx, |state, cx| {
                        state.open = open;
                        state.highlighted = None;
                        cx.notify();
                    });
                })
                .on_highlight(move |ix, _, cx| {
                    highlight_state.update(cx, |state, cx| {
                        state.highlighted = Some(ix);
                        cx.notify();
                    });
                })
                .on_confirm(move |_, cx| {
                    confirm_state.update(cx, |state, _| {
                        if let Some(ix) = state.highlighted {
                            state.confirmed.lock().unwrap().push(ix);
                        }
                    });
                })
                .scroll_handle(&self.scroll_handle)
                .child(
                    div()
                        .id("list")
                        .track_focus(&self.content_focus_handle)
                        .track_scroll(&self.scroll_handle)
                        .overflow_y_scroll()
                        // Three rows show: a page.
                        .h(px(60.))
                        .children(
                            self.labels
                                .iter()
                                .map(|label| div().h(px(20.)).child(*label)),
                        ),
                )
        }
    }

    fn list_harness(
        cx: &mut TestAppContext,
        selected: Option<usize>,
    ) -> (&mut VisualTestContext, Entity<ListHarness>) {
        labeled_list_harness(cx, &FRUITS, selected)
    }

    fn labeled_list_harness<'a>(
        cx: &'a mut TestAppContext,
        labels: &'static [&'static str],
        selected: Option<usize>,
    ) -> (&'a mut VisualTestContext, Entity<ListHarness>) {
        cx.update(crate::init);
        let (state, cx) = cx.add_window_view(move |_, cx| ListHarness {
            labels,
            open: false,
            selected,
            highlighted: None,
            focus_handle: cx.focus_handle(),
            content_focus_handle: cx.focus_handle(),
            scroll_handle: ScrollHandle::new(),
            confirmed: Arc::new(Mutex::new(Vec::new())),
        });
        cx.update(|window, cx| {
            state.read(cx).focus_handle.clone().focus(window, cx);
            window.draw(cx).clear(cx);
        });
        (cx, state)
    }

    fn highlighted(cx: &mut VisualTestContext, state: &Entity<ListHarness>) -> Option<usize> {
        state.read_with(cx, |state, _| state.highlighted)
    }

    /// The arrows open a closed select on its selected option, then move
    /// the highlight one option at a time, stopping at either end; Home and
    /// End jump to the ends, and Enter confirms the highlighted option.
    #[gpui::test]
    fn arrows_home_and_end_move_the_highlight(cx: &mut TestAppContext) {
        let (cx, state) = list_harness(cx, Some(2));

        cx.simulate_keystrokes("down");
        cx.update(|window, cx| {
            assert!(state.read(cx).open);
            assert!(state.read(cx).content_focus_handle.is_focused(window));
        });
        assert_eq!(highlighted(cx, &state), Some(2));
        cx.simulate_keystrokes("down down down");
        assert_eq!(highlighted(cx, &state), Some(4), "stops at the last");
        cx.simulate_keystrokes("up");
        assert_eq!(highlighted(cx, &state), Some(3));
        cx.simulate_keystrokes("home");
        assert_eq!(highlighted(cx, &state), Some(0));
        cx.simulate_keystrokes("up");
        assert_eq!(highlighted(cx, &state), Some(0), "stops at the first");
        cx.simulate_keystrokes("end enter");
        assert_eq!(
            *state
                .read_with(cx, |state, _| state.confirmed.clone())
                .lock()
                .unwrap(),
            [4]
        );
    }

    /// While nothing is highlighted, moves start from the selected option.
    #[gpui::test]
    fn moves_start_from_the_selection(cx: &mut TestAppContext) {
        let (cx, state) = list_harness(cx, Some(1));
        cx.simulate_keystrokes("enter");
        assert!(state.read_with(cx, |state, _| state.open));
        assert_eq!(highlighted(cx, &state), None);
        cx.simulate_keystrokes("down");
        assert_eq!(highlighted(cx, &state), Some(2));
    }

    /// Typing the start of a label highlights it, opening a closed select;
    /// repeating a letter steps through the labels starting with it.
    #[gpui::test]
    fn typing_highlights_a_matching_option(cx: &mut TestAppContext) {
        let (cx, state) = list_harness(cx, None);

        cx.simulate_keystrokes("b");
        assert!(state.read_with(cx, |state, _| state.open));
        assert_eq!(highlighted(cx, &state), Some(2));
        cx.simulate_keystrokes("b");
        assert_eq!(highlighted(cx, &state), Some(3));
        cx.simulate_keystrokes("b");
        assert_eq!(highlighted(cx, &state), Some(2), "wraps around");
    }

    #[gpui::test]
    fn typed_letters_build_one_search(cx: &mut TestAppContext) {
        let (cx, state) = list_harness(cx, None);
        cx.simulate_keystrokes("a p r");
        assert_eq!(highlighted(cx, &state), Some(1));
    }

    /// The pointer moving over an option of the open list highlights it, and
    /// the keys go on from there.
    #[gpui::test]
    fn the_pointer_highlights_the_option_it_moves_over(cx: &mut TestAppContext) {
        let (cx, state) = list_harness(cx, Some(0));
        cx.simulate_keystrokes("down");
        cx.update(|window, cx| window.draw(cx).clear(cx));
        let option = |cx: &mut VisualTestContext, ix| {
            state.read_with(cx, |state, _| {
                state.scroll_handle.bounds_for_item(ix).unwrap().center()
            })
        };

        let position = option(cx, 2);
        cx.simulate_mouse_move(position, None, gpui::Modifiers::default());
        assert_eq!(highlighted(cx, &state), Some(2));
        cx.simulate_keystrokes("up");
        assert_eq!(highlighted(cx, &state), Some(1));

        // Outside the list, the highlight stays.
        cx.simulate_mouse_move(
            gpui::point(px(300.), px(300.)),
            None,
            gpui::Modifiers::default(),
        );
        assert_eq!(highlighted(cx, &state), Some(1));
    }

    /// Page Up and Page Down move by the rows the list shows, stopping at
    /// either end.
    #[gpui::test]
    fn page_keys_move_the_highlight_by_a_page(cx: &mut TestAppContext) {
        let (cx, state) = list_harness(cx, Some(0));
        cx.simulate_keystrokes("down");
        cx.update(|window, cx| window.draw(cx).clear(cx));
        cx.simulate_keystrokes("pagedown");
        assert_eq!(highlighted(cx, &state), Some(3));
        cx.simulate_keystrokes("pagedown");
        assert_eq!(highlighted(cx, &state), Some(4));
        cx.simulate_keystrokes("pageup");
        assert_eq!(highlighted(cx, &state), Some(1));
        cx.simulate_keystrokes("pageup");
        assert_eq!(highlighted(cx, &state), Some(0));
    }

    /// Space typed within a search belongs to it, so a label with a space
    /// can be typed out; it confirms nothing.
    #[gpui::test]
    fn space_continues_a_search(cx: &mut TestAppContext) {
        let (cx, state) = labeled_list_harness(cx, &["Newark", "New York", "Paris"], None);
        cx.simulate_keystrokes("n e w");
        assert_eq!(highlighted(cx, &state), Some(0));
        cx.simulate_keystrokes("space");
        assert_eq!(highlighted(cx, &state), Some(1));
        assert!(state.read_with(cx, |state, _| state.open));
        assert!(
            state
                .read_with(cx, |state, _| state.confirmed.clone())
                .lock()
                .unwrap()
                .is_empty()
        );
    }

    /// Without a search going on, Space confirms the highlighted option.
    #[gpui::test]
    fn space_confirms_without_a_search(cx: &mut TestAppContext) {
        let (cx, state) = list_harness(cx, Some(2));
        cx.simulate_keystrokes("down down space");
        assert_eq!(
            *state
                .read_with(cx, |state, _| state.confirmed.clone())
                .lock()
                .unwrap(),
            [3]
        );
    }

    /// A select between two other Tab stops, in a Base Root.
    struct TabHarness {
        open: bool,
        before: FocusHandle,
        trigger: FocusHandle,
        content: FocusHandle,
        after: FocusHandle,
        closing: Arc<Mutex<Vec<&'static str>>>,
    }

    impl Render for TabHarness {
        fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            let state = cx.entity();
            let dismissed = self.closing.clone();
            let changed = self.closing.clone();
            div()
                .child(
                    div()
                        .track_focus(&self.before.clone().tab_stop(true))
                        .size(px(10.)),
                )
                .child(
                    Select::new("select")
                        .open(self.open)
                        .focus_handle(&self.trigger)
                        .content_focus_handle(&self.content)
                        .options(FRUITS)
                        .on_dismiss(move |_, _| dismissed.lock().unwrap().push("dismiss"))
                        .on_open_change(move |open, _, cx| {
                            changed
                                .lock()
                                .unwrap()
                                .push(if open { "open" } else { "close" });
                            state.update(cx, |state, cx| {
                                state.open = open;
                                cx.notify();
                            });
                        })
                        .size(px(10.))
                        .when(self.open, |this| {
                            this.child(div().track_focus(&self.content).size(px(10.)))
                        }),
                )
                .child(
                    div()
                        .track_focus(&self.after.clone().tab_stop(true))
                        .size(px(10.)),
                )
        }
    }

    /// Tab and Shift-Tab close an open list without choosing, as Escape
    /// does, and move the focus on from the select as usual.
    #[gpui::test]
    fn tab_closes_the_list_and_moves_the_focus_on(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let mut harness = None;
        let (_, cx) = cx.add_window_view(|window, cx| {
            let content = cx.new(|cx| TabHarness {
                open: false,
                before: cx.focus_handle(),
                trigger: cx.focus_handle(),
                content: cx.focus_handle(),
                after: cx.focus_handle(),
                closing: Arc::new(Mutex::new(Vec::new())),
            });
            harness = Some(content.clone());
            crate::Root::new(content, window, cx)
        });
        let state = harness.unwrap();
        cx.update(|window, cx| {
            window.activate_window();
            state.read(cx).trigger.clone().focus(window, cx);
            window.draw(cx).clear(cx);
        });

        cx.simulate_keystrokes("down");
        cx.update(|window, cx| assert!(state.read(cx).content.is_focused(window)));
        cx.simulate_keystrokes("tab");
        cx.update(|window, cx| {
            assert!(!state.read(cx).open);
            assert!(state.read(cx).after.is_focused(window));
        });

        cx.update(|window, cx| state.read(cx).trigger.clone().focus(window, cx));
        cx.simulate_keystrokes("down shift-tab");
        cx.update(|window, cx| {
            assert!(!state.read(cx).open);
            assert!(state.read(cx).before.is_focused(window));
        });
        assert_eq!(
            *state
                .read_with(cx, |state, _| state.closing.clone())
                .lock()
                .unwrap(),
            ["open", "dismiss", "close", "open", "dismiss", "close"]
        );
    }

    #[test]
    fn typeahead_starts_over_after_a_pause() {
        let mut typeahead = Typeahead::default();
        let now = Instant::now();
        assert_eq!(typeahead.push("A", now), "a");
        assert_eq!(typeahead.push("p", now + Duration::from_millis(500)), "ap");
        assert_eq!(typeahead.push("c", now + Duration::from_secs(2)), "c");
        // Closing the list ends the search; opening it does not.
        typeahead.sync(true);
        assert_eq!(typeahead.push("h", now + Duration::from_secs(2)), "ch");
        typeahead.sync(false);
        assert_eq!(typeahead.push("a", now + Duration::from_secs(2)), "a");
    }

    #[test]
    fn typeahead_matches_by_prefix_from_the_current_option() {
        let labels: Vec<SharedString> = FRUITS.into_iter().map(Into::into).collect();
        assert_eq!(typeahead_match(&labels, "ap", None), Some(0));
        assert_eq!(typeahead_match(&labels, "apr", Some(0)), Some(1));
        // A longer search keeps a current option that still matches.
        assert_eq!(typeahead_match(&labels, "bl", Some(3)), Some(3));
        assert_eq!(typeahead_match(&labels, "c", Some(4)), Some(4));
        assert_eq!(typeahead_match(&labels, "z", None), None);
    }
}
