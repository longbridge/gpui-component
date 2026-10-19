//! Roving keyboard focus for the item groups: [`RadioGroup`](crate::RadioGroup),
//! [`ToggleGroup`](crate::ToggleGroup) and [`Tabs`](crate::Tabs).
//!
//! A group is one Tab stop. Its items register while the group lays them
//! out, wherever they sit below it, so a styled skin can wrap each item in
//! its own containers. Once every item has registered, the group picks the
//! one that holds the group's Tab stop: the focused item, otherwise the
//! selected one, otherwise the item focused last, otherwise the first. The
//! arrow keys move focus between the items, wrapping at either end.

use std::{
    cell::{Cell, RefCell},
    panic::Location,
    rc::Rc,
};

use gpui::{
    A11ySubtreeBuilder, AnyElement, App, Axis, Bounds, ClickEvent, Div, Element, ElementId,
    FocusHandle, GlobalElementId, InspectorElementId, InteractiveElement, IntoElement,
    KeyDownEvent, KeyboardButton, KeyboardClickEvent, LayoutId, Pixels, Role, Stateful, Window,
    accesskit, prelude::FluentBuilder as _,
};

/// The item type a group roves over, so an item registers with the innermost
/// group of its own kind rather than, say, a tab list it happens to sit in.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Member {
    Radio,
    Toggle,
    Tab,
}

/// Activates an item as a keyboard click would.
pub(crate) type Activate = Rc<dyn Fn(&ClickEvent, &mut Window, &mut App)>;

/// How a group answers the keyboard.
#[derive(Clone, Copy)]
pub(crate) struct Keys {
    member: Member,
    /// The axis whose arrow keys move focus; `None` takes all four.
    axis: Option<Axis>,
    home_end: bool,
    /// Whether moving focus to an item also activates it.
    activate_on_focus: bool,
    /// Whether the item focused last holds the Tab stop ahead of the selected one.
    remember_focus: bool,
}

impl Keys {
    /// A radio group: every arrow key moves, and the checked radio follows
    /// focus, as platform radio groups do.
    pub(crate) fn radios() -> Self {
        Self {
            member: Member::Radio,
            axis: None,
            home_end: false,
            activate_on_focus: true,
            remember_focus: false,
        }
    }

    /// A toggle group: the arrow keys of its axis and Home/End move focus
    /// without pressing anything, and Tab returns to the toggle focused last.
    pub(crate) fn toggles(axis: Axis) -> Self {
        Self {
            member: Member::Toggle,
            axis: Some(axis),
            home_end: true,
            activate_on_focus: false,
            remember_focus: true,
        }
    }

    /// A tab list: the arrow keys of its axis and Home/End move focus, and
    /// select the focused tab when `activate_on_focus`.
    pub(crate) fn tabs(axis: Axis, activate_on_focus: bool) -> Self {
        Self {
            member: Member::Tab,
            axis: Some(axis),
            home_end: true,
            activate_on_focus,
            remember_focus: false,
        }
    }

    fn step(&self, key: &str) -> Option<Step> {
        let horizontal = self.axis != Some(Axis::Vertical);
        let vertical = self.axis != Some(Axis::Horizontal);
        match key {
            "left" if horizontal => Some(Step::Previous),
            "right" if horizontal => Some(Step::Next),
            "up" if vertical => Some(Step::Previous),
            "down" if vertical => Some(Step::Next),
            "home" if self.home_end => Some(Step::First),
            "end" if self.home_end => Some(Step::Last),
            _ => None,
        }
    }
}

enum Step {
    Previous,
    Next,
    First,
    Last,
}

/// One enabled item, as registered while the group lays it out.
struct Entry {
    focus: FocusHandle,
    selected: bool,
    bounds: Cell<Bounds<Pixels>>,
    activate: Option<Activate>,
}

#[derive(Default)]
struct GroupState {
    entries: Vec<Rc<Entry>>,
    tab_stop: Option<FocusHandle>,
    last_focused: Option<FocusHandle>,
}

thread_local! {
    /// The groups laying out their items, innermost last.
    static GROUPS: RefCell<Vec<RovingFocus>> = const { RefCell::new(Vec::new()) };
}

/// A group's roving focus, kept across frames under the group's id.
#[derive(Clone)]
pub(crate) struct RovingFocus {
    keys: Keys,
    state: Rc<RefCell<GroupState>>,
    /// The group's own handle, which hands focus on to its Tab stop item.
    focus: Option<FocusHandle>,
}

impl RovingFocus {
    pub(crate) fn new(id: &ElementId, keys: Keys, window: &mut Window, cx: &mut App) -> Self {
        let state = window
            .use_keyed_state(id.clone(), cx, |_, _| Rc::<RefCell<GroupState>>::default())
            .read(cx)
            .clone();
        Self {
            keys,
            state,
            focus: None,
        }
    }

    /// Roves focus among the items laid out inside `group`. Focusing `focus`,
    /// the group's own handle, moves focus on to the item holding the Tab stop.
    pub(crate) fn group(mut self, group: Stateful<Div>, focus: Option<FocusHandle>) -> RovingGroup {
        self.focus = focus;
        let roving = self.clone();
        RovingGroup {
            child: group
                .when_some(self.focus.as_ref(), |group, focus| {
                    group.track_focus(&focus.clone().tab_stop(false))
                })
                .on_key_down(move |event, window, cx| roving.key_down(event, window, cx))
                .into_any_element(),
            roving: self,
        }
    }

    fn key_down(&self, event: &KeyDownEvent, window: &mut Window, cx: &mut App) {
        let modifiers = event.keystroke.modifiers;
        if modifiers.control || modifiers.alt || modifiers.shift || modifiers.platform {
            return;
        }
        let Some(step) = self.keys.step(&event.keystroke.key) else {
            return;
        };
        let target = {
            let state = self.state.borrow();
            let entries = &state.entries;
            let Some(current) = entries.iter().position(|e| e.focus.is_focused(window)) else {
                return;
            };
            let last = entries.len() - 1;
            let target = match step {
                Step::Previous => current.checked_sub(1).unwrap_or(last),
                Step::Next if current == last => 0,
                Step::Next => current + 1,
                Step::First => 0,
                Step::Last => last,
            };
            entries[target].clone()
        };

        cx.stop_propagation();
        window.focus(&target.focus, cx);
        if self.keys.activate_on_focus
            && let Some(activate) = &target.activate
        {
            let event = ClickEvent::Keyboard(KeyboardClickEvent {
                button: KeyboardButton::Space,
                bounds: target.bounds.get(),
            });
            activate(&event, window, cx);
        }
    }

    /// Picks the item holding the group's Tab stop, once all have registered,
    /// and hands it focus the group's own handle took.
    fn settle(&self, window: &mut Window, cx: &mut App) {
        let mut state = self.state.borrow_mut();
        let focused = state
            .entries
            .iter()
            .find(|entry| entry.focus.is_focused(window))
            .map(|entry| entry.focus.clone());
        if focused.is_some() {
            state.last_focused = focused.clone();
        }
        let selected = || {
            state
                .entries
                .iter()
                .find(|entry| entry.selected)
                .map(|entry| entry.focus.clone())
        };
        let remembered = || {
            state
                .last_focused
                .clone()
                .filter(|last| state.entries.iter().any(|entry| entry.focus == *last))
        };
        let tab_stop = focused
            .or_else(|| {
                if self.keys.remember_focus {
                    remembered().or_else(selected)
                } else {
                    selected().or_else(remembered)
                }
            })
            .or_else(|| state.entries.first().map(|entry| entry.focus.clone()));
        state.tab_stop = tab_stop.clone();
        drop(state);

        // Before the items prepaint, so this frame already shows the focus.
        if let Some((own, tab_stop)) = self.focus.as_ref().zip(tab_stop)
            && own.is_focused(window)
        {
            window.focus(&tab_stop, cx);
        }
    }

    /// The innermost group of `member` items laying out, if any.
    fn current(member: Member) -> Option<Self> {
        GROUPS.with_borrow(|groups| {
            groups
                .iter()
                .rev()
                .find(|group| group.keys.member == member)
                .cloned()
        })
    }
}

/// A group root that lets its items register while it lays them out.
pub(crate) struct RovingGroup {
    roving: RovingFocus,
    child: AnyElement,
}

impl IntoElement for RovingGroup {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for RovingGroup {
    type RequestLayoutState = ();
    type PrepaintState = ();

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, ()) {
        self.roving.state.borrow_mut().entries.clear();
        GROUPS.with_borrow_mut(|groups| groups.push(self.roving.clone()));
        let layout_id = self.child.request_layout(window, cx);
        GROUPS.with_borrow_mut(|groups| groups.pop());
        (layout_id, ())
    }

    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        _: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) {
        self.roving.settle(window, cx);
        self.child.prepaint(window, cx);
    }

    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        _: &mut (),
        _: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) {
        self.child.paint(window, cx);
    }
}

/// An item's root, which takes its Tab stop from its group once the group has
/// seen every item.
pub(crate) struct RovingItem<E> {
    child: Option<E>,
    registration: Option<(Rc<Entry>, RovingFocus)>,
}

impl<E: Element + InteractiveElement> RovingItem<E> {
    /// Wraps an item root that tracks `focus` (`None` when disabled), and
    /// registers it with the innermost group of `member` items.
    pub(crate) fn new(
        member: Member,
        child: E,
        focus: Option<FocusHandle>,
        selected: bool,
        activate: Option<Activate>,
    ) -> Self {
        let registration = focus
            .zip(RovingFocus::current(member))
            .map(|(focus, group)| {
                let entry = Rc::new(Entry {
                    focus,
                    selected,
                    bounds: Cell::default(),
                    activate,
                });
                group.state.borrow_mut().entries.push(entry.clone());
                (entry, group)
            });
        Self {
            child: Some(child),
            registration,
        }
    }

    fn child(&mut self) -> &mut E {
        self.child.as_mut().expect("roving item child")
    }
}

impl<E: Element + InteractiveElement> IntoElement for RovingItem<E> {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl<E: Element + InteractiveElement> Element for RovingItem<E> {
    type RequestLayoutState = E::RequestLayoutState;
    type PrepaintState = E::PrepaintState;

    fn id(&self) -> Option<ElementId> {
        self.child.as_ref().and_then(Element::id)
    }

    fn source_location(&self) -> Option<&'static Location<'static>> {
        self.child.as_ref().and_then(Element::source_location)
    }

    fn request_layout(
        &mut self,
        id: Option<&GlobalElementId>,
        inspector_id: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        self.child().request_layout(id, inspector_id, window, cx)
    }

    fn prepaint(
        &mut self,
        id: Option<&GlobalElementId>,
        inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        request_layout: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        if let Some((entry, _)) = &self.registration {
            entry.bounds.set(bounds);
        }
        self.child()
            .prepaint(id, inspector_id, bounds, request_layout, window, cx)
    }

    fn paint(
        &mut self,
        id: Option<&GlobalElementId>,
        inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        request_layout: &mut Self::RequestLayoutState,
        prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        // The window records Tab stops while painting, from the handle the
        // item tracks, so only the group's chosen item stays a Tab stop.
        if let Some((entry, group)) = &self.registration {
            let holds_tab_stop = group.state.borrow().tab_stop.as_ref() == Some(&entry.focus);
            if !holds_tab_stop && let Some(child) = self.child.take() {
                self.child = Some(child.track_focus(&entry.focus.clone().tab_stop(false)));
            }
        }
        self.child().paint(
            id,
            inspector_id,
            bounds,
            request_layout,
            prepaint,
            window,
            cx,
        )
    }

    fn a11y_role(&self) -> Option<Role> {
        self.child.as_ref().and_then(Element::a11y_role)
    }

    fn write_a11y_info(&self, node: &mut accesskit::Node) {
        if let Some(child) = &self.child {
            child.write_a11y_info(node);
        }
    }

    fn a11y_synthetic_children(
        &mut self,
        prepaint: &mut Self::PrepaintState,
        builder: &mut A11ySubtreeBuilder,
    ) {
        self.child().a11y_synthetic_children(prepaint, builder);
    }
}
