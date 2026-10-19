use std::rc::Rc;

use gpui::{
    AnyElement, App, ElementId, FocusHandle, IntoElement, KeyBinding, ParentElement, RenderOnce,
    ScrollHandle, SharedString, StyleRefinement, Styled, Window, prelude::FluentBuilder as _,
};

use crate::{
    Select,
    actions::{Cancel, Confirm, SelectDown, SelectPageDown, SelectPageUp, SelectUp},
    styled::StyledExt as _,
};

const CONTEXT: &str = "Combobox";

pub(crate) fn init(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("up", SelectUp, Some(CONTEXT)),
        KeyBinding::new("down", SelectDown, Some(CONTEXT)),
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

/// An unstyled controlled combobox root.
///
/// Applications own the trigger, popup, searchable collection, selection, and
/// appearance. This root owns combobox semantics and keyboard focus transfer.
///
/// A combobox that supplies its matching [`options`](Self::options) lets this
/// root move the highlighted option with Up, Down, Page Up and Page Down while
/// its query field keeps the focus, as a combobox's list highlights through
/// its active descendant. Home, End and typed letters stay with the query
/// field. Tab and Shift-Tab close an open combobox and move the focus on.
#[derive(IntoElement)]
pub struct Combobox {
    id: ElementId,
    open: bool,
    disabled: bool,
    focus_handle: Option<FocusHandle>,
    content_focus_handle: Option<FocusHandle>,
    style: StyleRefinement,
    children: Vec<AnyElement>,
    on_open_change: Option<OpenChangeHandler>,
    on_confirm: Option<ActionHandler>,
    on_dismiss: Option<ActionHandler>,
    options: Option<Vec<SharedString>>,
    selected_ix: Option<usize>,
    highlighted_ix: Option<usize>,
    on_highlight: Option<HighlightHandler>,
    scroll_handle: Option<ScrollHandle>,
}

impl Combobox {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            open: false,
            disabled: false,
            focus_handle: None,
            content_focus_handle: None,
            style: StyleRefinement::default(),
            children: Vec::new(),
            on_open_change: None,
            on_confirm: None,
            on_dismiss: None,
            options: None,
            selected_ix: None,
            highlighted_ix: None,
            on_highlight: None,
            scroll_handle: None,
        }
    }

    pub fn open(mut self, open: bool) -> Self {
        self.open = open;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn focus_handle(mut self, focus_handle: &FocusHandle) -> Self {
        self.focus_handle = Some(focus_handle.clone());
        self
    }

    pub fn content_focus_handle(mut self, focus_handle: &FocusHandle) -> Self {
        self.content_focus_handle = Some(focus_handle.clone());
        self
    }

    pub fn on_open_change(
        mut self,
        handler: impl Fn(bool, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_open_change = Some(Rc::new(handler));
        self
    }

    /// Handles the Confirm action while the combobox is open.
    pub fn on_confirm(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_confirm = Some(Rc::new(handler));
        self
    }

    /// Handles a dismissal requested through the Cancel action.
    ///
    /// This runs before the controlled open state is asked to close, so a
    /// combobox that commits its pending value on dismissal can still read
    /// that value here.
    pub fn on_dismiss(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_dismiss = Some(Rc::new(handler));
        self
    }

    /// Supplies the labels of the options the open list shows, in order: the
    /// ones matching the current query.
    ///
    /// With options, Up, Down, Page Up and Page Down move the highlighted
    /// option, opening a closed combobox first, even while the query field
    /// has the focus.
    pub fn options(mut self, labels: impl IntoIterator<Item = impl Into<SharedString>>) -> Self {
        self.options = Some(labels.into_iter().map(Into::into).collect());
        self
    }

    /// Sets the index of the option holding the committed value.
    pub fn selected_index(mut self, ix: Option<usize>) -> Self {
        self.selected_ix = ix;
        self
    }

    /// Sets the application-controlled index of the highlighted option.
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
    /// handle, whose children must be the options in order. The pointer
    /// moving over an option highlights it too.
    pub fn scroll_handle(mut self, scroll_handle: &ScrollHandle) -> Self {
        self.scroll_handle = Some(scroll_handle.clone());
        self
    }
}

impl Styled for Combobox {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl ParentElement for Combobox {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl RenderOnce for Combobox {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        Select::new(self.id)
            .open(self.open)
            .disabled(self.disabled)
            .key_context(CONTEXT)
            .when_some(self.focus_handle, |this, handle| this.focus_handle(&handle))
            .when_some(self.content_focus_handle, |this, handle| {
                this.content_focus_handle(&handle)
            })
            .when_some(self.on_open_change, |this, handler| {
                this.on_open_change(move |open, window, cx| handler(open, window, cx))
            })
            .when_some(self.on_confirm, |this, handler| {
                this.on_confirm(move |window, cx| handler(window, cx))
            })
            .when_some(self.on_dismiss, |this, handler| {
                this.on_dismiss(move |window, cx| handler(window, cx))
            })
            .when_some(self.options, |this, labels| this.options(labels))
            .selected_index(self.selected_ix)
            .highlighted_index(self.highlighted_ix)
            .when_some(self.on_highlight, |this, handler| {
                this.on_highlight(move |ix, window, cx| handler(ix, window, cx))
            })
            .when_some(self.scroll_handle, |this, handle| {
                this.scroll_handle(&handle)
            })
            // Typed letters belong to the query field.
            .typeahead(false)
            .children(self.children)
            .refine_style(&self.style)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::InputState;
    use gpui::{
        AppContext as _, Context, Entity, Focusable, InteractiveElement as _, Render,
        TestAppContext, VisualTestContext, div, px,
    };
    use std::sync::{Arc, Mutex};

    struct Harness {
        open: bool,
        trigger_focus: FocusHandle,
        content_focus: FocusHandle,
        events: Arc<Mutex<Vec<&'static str>>>,
    }

    impl Harness {
        fn new(cx: &mut Context<Self>) -> Self {
            Self {
                open: true,
                trigger_focus: cx.focus_handle(),
                content_focus: cx.focus_handle(),
                events: Arc::new(Mutex::new(Vec::new())),
            }
        }
    }

    impl Focusable for Harness {
        fn focus_handle(&self, _: &App) -> FocusHandle {
            self.trigger_focus.clone()
        }
    }

    impl Render for Harness {
        fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            let state = cx.entity();
            let confirm_events = self.events.clone();
            let dismiss_events = self.events.clone();
            let open_events = self.events.clone();

            Combobox::new("combobox")
                .open(self.open)
                .focus_handle(&self.trigger_focus)
                .content_focus_handle(&self.content_focus)
                .on_confirm(move |_, _| confirm_events.lock().unwrap().push("confirm"))
                .on_dismiss(move |_, _| dismiss_events.lock().unwrap().push("dismiss"))
                .on_open_change(move |open, _, cx| {
                    open_events.lock().unwrap().push("close");
                    state.update(cx, |state, cx| {
                        state.open = open;
                        cx.notify();
                    });
                })
                .child(div().track_focus(&self.content_focus).size(px(20.)))
        }
    }

    fn harness(cx: &mut TestAppContext) -> (&mut VisualTestContext, gpui::Entity<Harness>) {
        cx.update(crate::init);
        let (state, cx) = cx.add_window_view(|_, cx| Harness::new(cx));
        cx.update(|window, cx| {
            let content_focus = state.read(cx).content_focus.clone();
            content_focus.focus(window, cx);
            window.draw(cx).clear(cx);
        });
        (cx, state)
    }

    #[gpui::test]
    fn escape_dismisses_then_closes_and_restores_trigger_focus(cx: &mut TestAppContext) {
        let (cx, state) = harness(cx);
        cx.simulate_keystrokes("escape");

        cx.update(|window, cx| {
            assert!(!state.read(cx).open);
            assert!(state.read(cx).trigger_focus.is_focused(window));
            // Dismissal runs before the close request so a caller can still
            // commit its pending value.
            assert_eq!(
                state.read(cx).events.lock().unwrap().as_slice(),
                &["dismiss", "close"]
            );
        });
    }

    #[gpui::test]
    fn enter_confirms_without_dismissing_while_open(cx: &mut TestAppContext) {
        let (cx, state) = harness(cx);
        cx.simulate_keystrokes("enter");

        cx.update(|_, cx| {
            assert!(state.read(cx).open);
            assert_eq!(
                state.read(cx).events.lock().unwrap().as_slice(),
                &["confirm"]
            );
        });
    }
    /// An open combobox whose query field keeps the focus.
    struct QueryHarness {
        query: Entity<InputState>,
        highlighted: Option<usize>,
    }

    impl Render for QueryHarness {
        fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            let state = cx.entity();
            Combobox::new("combobox")
                .open(true)
                .content_focus_handle(&self.query.focus_handle(cx))
                .options(["Apple", "Banana", "Cherry"])
                .highlighted_index(self.highlighted)
                .on_highlight(move |ix, _, cx| {
                    state.update(cx, |state, cx| {
                        state.highlighted = Some(ix);
                        cx.notify();
                    });
                })
                .child(crate::Input::new(&self.query))
        }
    }

    /// Up, Down and the page keys move the highlight past the focused query
    /// field, which still takes the typing.
    #[gpui::test]
    fn arrows_move_the_highlight_from_the_query_field(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let (state, cx) = cx.add_window_view(|window, cx| QueryHarness {
            query: cx.new(|cx| InputState::new(window, cx)),
            highlighted: None,
        });
        let query = cx.update(|window, cx| {
            let query = state.read(cx).query.clone();
            query.focus_handle(cx).focus(window, cx);
            window.draw(cx).clear(cx);
            query
        });

        cx.simulate_keystrokes("down down");
        assert_eq!(state.read_with(cx, |state, _| state.highlighted), Some(1));
        cx.simulate_keystrokes("up");
        assert_eq!(state.read_with(cx, |state, _| state.highlighted), Some(0));
        // Without a scroll handle to measure a page, a page is every option.
        cx.simulate_keystrokes("pagedown");
        assert_eq!(state.read_with(cx, |state, _| state.highlighted), Some(2));
        cx.simulate_keystrokes("pageup");
        assert_eq!(state.read_with(cx, |state, _| state.highlighted), Some(0));
        cx.simulate_keystrokes("a n");
        assert_eq!(query.read_with(cx, |query, _| query.value()), "an");
        cx.update(|window, cx| assert!(query.focus_handle(cx).is_focused(window)));
    }
}
