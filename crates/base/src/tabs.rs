use std::rc::Rc;

use gpui::{
    AnyElement, App, Axis, ClickEvent, Div, ElementId, FocusHandle, InteractiveElement,
    Interactivity, IntoElement, MouseButton, ParentElement, Refineable as _, RenderOnce, Role,
    SharedString, StatefulInteractiveElement, StyleRefinement, Styled, Window, accesskit, div,
    prelude::FluentBuilder as _, relative,
};
use smallvec::SmallVec;

use crate::{
    StateStyle, StyledExt as _, TestSupportExt as _,
    a11y_disabled::A11yDisabledExt as _,
    roving_focus::{Keys, Member, RovingFocus, RovingItem},
};

type ClickHandler = Rc<dyn Fn(&ClickEvent, &mut Window, &mut App)>;

/// An unstyled tab that owns activation, focus and accessibility behavior.
///
/// A tab takes keyboard focus but not pointer focus: clicking a tab selects
/// it and leaves focus where it was, so a document tab bar never takes focus
/// from the document. Enter and Space activate the focused tab. Inside
/// [`Tabs`], the tab list is one Tab stop and the arrow keys move between
/// its tabs.
#[derive(IntoElement)]
pub struct Tab {
    id: ElementId,
    base: Div,
    style: StyleRefinement,
    semantic_styles: TabStyles,
    selected: bool,
    disabled: bool,
    children: SmallVec<[AnyElement; 2]>,
    on_click: Option<ClickHandler>,
    accessibility_label: Option<SharedString>,
    position_in_set: Option<usize>,
    size_of_set: Option<usize>,
    tab_index: isize,
    tab_stop: bool,
    provided_focus_handle: Option<FocusHandle>,
}

impl Tab {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            base: div(),
            style: StyleRefinement::default(),
            semantic_styles: TabStyles::default(),
            selected: false,
            disabled: false,
            children: SmallVec::new(),
            on_click: None,
            accessibility_label: None,
            position_in_set: None,
            size_of_set: None,
            tab_index: 0,
            tab_stop: true,
            provided_focus_handle: None,
        }
    }

    /// Updates the element identity used when the tab is rendered.
    pub fn id(mut self, id: impl Into<ElementId>) -> Self {
        self.id = id.into();
        self
    }

    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn accessibility_label(mut self, label: impl Into<SharedString>) -> Self {
        self.accessibility_label = Some(label.into());
        self
    }

    /// Sets this tab's one-based position and the tab list's total size, so
    /// assistive technology can announce "tab 2 of 5".
    pub fn set_position(mut self, position: usize, size: usize) -> Self {
        self.position_in_set = Some(position);
        self.size_of_set = Some(size);
        self
    }

    pub fn on_click(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_click = Some(Rc::new(handler));
        self
    }

    pub fn styles(mut self, build: impl FnOnce(TabStyles) -> TabStyles) -> Self {
        self.semantic_styles = build(self.semantic_styles);
        self
    }

    /// Uses a caller-owned focus handle instead of creating keyed state.
    ///
    /// A styled tab needs this to draw its own focus ring from the same
    /// handle the primitive tracks.
    pub fn track_focus(mut self, focus_handle: &FocusHandle) -> Self {
        self.provided_focus_handle = Some(focus_handle.clone());
        self
    }

    pub fn tab_index(mut self, tab_index: isize) -> Self {
        self.tab_index = tab_index;
        self
    }

    /// Sets whether the tab is a Tab stop. Inside [`Tabs`], the list keeps a
    /// single Tab stop on one of the tabs that allow it.
    pub fn tab_stop(mut self, tab_stop: bool) -> Self {
        self.tab_stop = tab_stop;
        self
    }

    fn focus_handle(&self, window: &mut Window, cx: &mut App) -> FocusHandle {
        self.provided_focus_handle.clone().unwrap_or_else(|| {
            window
                .use_keyed_state(self.id.clone(), cx, |_, cx| cx.focus_handle())
                .read(cx)
                .clone()
        })
    }

    fn resolved_style(&self) -> StyleRefinement {
        crate::state_style::resolve_style(
            &self.style,
            [
                self.selected.then_some(&self.semantic_styles.selected),
                self.disabled.then_some(&self.semantic_styles.disabled),
            ]
            .into_iter()
            .flatten(),
        )
    }
}

#[derive(Default)]
pub struct TabStyles {
    selected: StyleRefinement,
    disabled: StyleRefinement,
}

impl TabStyles {
    pub fn selected(mut self, build: impl FnOnce(StateStyle) -> StateStyle) -> Self {
        self.selected
            .refine(&build(StateStyle::default()).into_refinement());
        self
    }

    pub fn disabled(mut self, build: impl FnOnce(StateStyle) -> StateStyle) -> Self {
        self.disabled
            .refine(&build(StateStyle::default()).into_refinement());
        self
    }
}

impl Styled for Tab {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl ParentElement for Tab {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl InteractiveElement for Tab {
    fn interactivity(&mut self) -> &mut Interactivity {
        self.base.interactivity()
    }
}

impl StatefulInteractiveElement for Tab {}

impl RenderOnce for Tab {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let disabled = self.disabled;
        let selected = self.selected;
        let style = self.resolved_style();
        // Keep the keyed handle alive while disabled, so focus state survives.
        let focus_handle = self
            .focus_handle(window, cx)
            .tab_index(self.tab_index)
            .tab_stop(self.tab_stop);
        let focus_handle = (!disabled).then_some(focus_handle);
        let on_click = (!disabled).then_some(self.on_click).flatten();

        let tab = self
            .base
            .id(self.id)
            .test_support()
            .role(Role::Tab)
            // Match Button's neutral control geometry: a fixed-size tab
            // centers ordinary content, while callers still own its size,
            // spacing and visual treatment.
            .flex()
            .items_center()
            .justify_center()
            .line_height(relative(1.))
            .when_some(self.accessibility_label, |this, label| {
                this.aria_label(label)
            })
            .aria_selected(self.selected)
            .when_some(self.position_in_set, |this, position| {
                this.aria_position_in_set(position)
            })
            .when_some(self.size_of_set, |this, size| this.aria_size_of_set(size))
            .when_some(focus_handle.as_ref(), |this, focus_handle| {
                this.track_focus(focus_handle)
            })
            // Stopping the press here also keeps the window from moving focus
            // to the tab.
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .when_some(on_click.clone(), |this, on_click| {
                this.on_click(move |event, window, cx| on_click(event, window, cx))
            })
            .children(self.children)
            .refine_style(&style)
            .a11y_disabled(disabled);

        RovingItem::new(
            Member::Tab,
            tab,
            focus_handle,
            selected,
            on_click.map(|on_click| on_click as _),
        )
    }
}

/// An unstyled collection root for [`Tab`] elements.
///
/// Controlled selection and activation are expressed directly by its tabs,
/// which may sit anywhere below the list. The list owns the keyboard: it is
/// one Tab stop, held by the focused tab, otherwise the selected tab,
/// otherwise the first. The arrow keys of its axis move focus to the previous
/// or next enabled tab, wrapping at either end, and Home and End move to the
/// first and last. Moving focus selects the tab, as platform tab views do,
/// unless [`Tabs::activate_on_focus`] is turned off; then Enter or Space
/// selects the focused tab.
#[derive(IntoElement)]
pub struct Tabs {
    id: ElementId,
    base: gpui::Stateful<Div>,
    style: StyleRefinement,
    axis: Axis,
    activate_on_focus: bool,
    focus_handle: Option<FocusHandle>,
    children: SmallVec<[AnyElement; 2]>,
}

impl Tabs {
    pub fn new(id: impl Into<ElementId>) -> Self {
        let id = id.into();
        Self {
            base: div().id(id.clone()),
            id,
            style: StyleRefinement::default(),
            axis: Axis::Horizontal,
            activate_on_focus: true,
            focus_handle: None,
            children: SmallVec::new(),
        }
    }

    /// Sets the axis the tabs run along, which picks the arrow keys that move
    /// between them. Defaults to horizontal.
    pub fn axis(mut self, axis: Axis) -> Self {
        self.axis = axis;
        self
    }

    /// Sets whether moving focus with the keyboard also selects the tab.
    /// Defaults to `true`; turn it off when showing a tab's content is slow.
    pub fn activate_on_focus(mut self, activate_on_focus: bool) -> Self {
        self.activate_on_focus = activate_on_focus;
        self
    }

    /// Gives the group a focus handle of its own. Focusing it moves focus on
    /// to the item holding the group's Tab stop, and it contains the focus
    /// while any item has it, so an owner that keeps one focus handle per
    /// control can treat the group as one.
    pub fn track_focus(mut self, focus_handle: &FocusHandle) -> Self {
        self.focus_handle = Some(focus_handle.clone());
        self
    }
}

impl Styled for Tabs {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl ParentElement for Tabs {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl InteractiveElement for Tabs {
    fn interactivity(&mut self) -> &mut Interactivity {
        self.base.interactivity()
    }
}

impl StatefulInteractiveElement for Tabs {}

impl RenderOnce for Tabs {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let keys = Keys::tabs(self.axis, self.activate_on_focus);
        RovingFocus::new(&self.id, keys, window, cx).group(
            self.base
                .role(Role::TabList)
                .aria_orientation(match self.axis {
                    Axis::Horizontal => accesskit::Orientation::Horizontal,
                    Axis::Vertical => accesskit::Orientation::Vertical,
                })
                .children(self.children)
                .refine_style(&self.style),
            self.focus_handle,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ElementExt as _;
    use std::{
        cell::Cell,
        rc::Rc,
        sync::{Arc, Mutex},
    };

    use gpui::{
        Context, Element as _, Modifiers, Render, Role, VisualTestContext, accesskit, canvas, hsla,
        point, px,
    };

    struct TabHarness {
        disabled: bool,
        clicks: Rc<Cell<usize>>,
    }

    impl Render for TabHarness {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            let clicks = self.clicks.clone();
            Tab::new("tab")
                .disabled(self.disabled)
                .size(px(100.))
                .on_click(move |_, _, _| clicks.set(clicks.get() + 1))
        }
    }

    fn harness(
        cx: &mut gpui::TestAppContext,
        disabled: bool,
    ) -> (&mut VisualTestContext, Rc<Cell<usize>>) {
        let clicks = Rc::new(Cell::new(0));
        let (_, cx) = cx.add_window_view({
            let clicks = clicks.clone();
            move |_, _| TabHarness { disabled, clicks }
        });
        cx.update(|window, cx| window.draw(cx).clear(cx));
        (cx, clicks)
    }

    #[gpui::test]
    fn pointer_activation_and_disabled_gating_match_tabs(cx: &mut gpui::TestAppContext) {
        let (cx, clicks) = harness(cx, false);
        cx.simulate_click(point(px(10.), px(10.)), Modifiers::default());
        assert_eq!(clicks.get(), 1);

        let (cx, clicks) = harness(cx, true);
        cx.simulate_click(point(px(10.), px(10.)), Modifiers::default());
        assert_eq!(clicks.get(), 0);
    }

    #[gpui::test]
    fn fixed_height_tab_centers_ordinary_child_geometry(cx: &mut gpui::TestAppContext) {
        type Captured = Arc<
            Mutex<(
                Option<gpui::Bounds<gpui::Pixels>>,
                Option<gpui::Bounds<gpui::Pixels>>,
            )>,
        >;

        struct AlignmentProbe(Captured);

        impl Render for AlignmentProbe {
            fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
                let root_capture = self.0.clone();
                let child_capture = self.0.clone();
                Tab::new("alignment-tab")
                    .w(px(120.))
                    .h(px(40.))
                    .child(
                        div()
                            .w(px(48.))
                            .h(px(12.))
                            .on_prepaint(move |bounds, _, _| {
                                child_capture.lock().unwrap().1 = Some(bounds);
                            }),
                    )
                    .on_prepaint(move |bounds, _, _| {
                        root_capture.lock().unwrap().0 = Some(bounds);
                    })
            }
        }

        let captured = Arc::new(Mutex::new((None, None)));
        let (_, context) = cx.add_window_view({
            let captured = captured.clone();
            move |_, _| AlignmentProbe(captured)
        });
        context.update(|window, cx| window.draw(cx).clear(cx));

        let (root, child) = *captured.lock().unwrap();
        assert_eq!(
            child.expect("child bounds").center(),
            root.expect("tab bounds").center()
        );
    }

    #[gpui::test]
    fn exposes_tab_accessibility_state(cx: &mut gpui::TestAppContext) {
        type Captured = Arc<Mutex<Option<(accesskit::Node, accesskit::Node)>>>;

        struct Probe(Captured);

        impl Render for Probe {
            fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
                let captured = self.0.clone();
                canvas(
                    move |_, window, cx| {
                        let mut info = |tab: Tab| {
                            let mut node = accesskit::Node::new(Role::Tab);
                            tab.render(window, cx)
                                .into_element()
                                .write_a11y_info(&mut node);
                            node
                        };
                        let selected = info(
                            Tab::new("selected")
                                .selected(true)
                                .accessibility_label("Account")
                                .on_click(|_, _, _| {}),
                        );
                        let disabled =
                            info(Tab::new("disabled").disabled(true).on_click(|_, _, _| {}));
                        *captured.lock().unwrap() = Some((selected, disabled));
                    },
                    |_, _, _, _| {},
                )
            }
        }

        let captured: Captured = Arc::new(Mutex::new(None));
        let result = captured.clone();
        let (_, cx) = cx.add_window_view(move |_, _| Probe(captured));
        cx.update(|window, cx| window.draw(cx).clear(cx));
        let (selected, disabled) = result.lock().unwrap().take().unwrap();

        assert_eq!(selected.role(), Role::Tab);
        assert_eq!(selected.label(), Some("Account"));
        assert_eq!(selected.is_selected(), Some(true));
        assert!(selected.supports_action(accesskit::Action::Click));
        assert!(!disabled.supports_action(accesskit::Action::Click));
    }

    #[gpui::test]
    fn semantic_styles_preserve_the_legacy_state_priority(_cx: &mut gpui::TestAppContext) {
        let expected = hsla(0.3, 0.4, 0.5, 1.0);
        let mut tab = Tab::new("tab")
            .selected(true)
            .disabled(true)
            .styles(|styles| {
                styles
                    .selected(|style| style.opacity(0.8))
                    .disabled(|style| style.opacity(0.5))
            })
            .opacity(0.9);

        assert_eq!(tab.resolved_style().opacity, Some(0.5));
        tab.style().background = Some(expected.into());
        assert_eq!(tab.resolved_style().background, Some(expected.into()));
    }

    #[gpui::test]
    fn tabs_exposes_tab_list_role(cx: &mut gpui::TestAppContext) {
        type Captured = Arc<Mutex<Option<accesskit::Node>>>;
        struct Probe(Captured);

        impl Render for Probe {
            fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
                let captured = self.0.clone();
                canvas(
                    move |_, window, cx| {
                        let mut node = accesskit::Node::new(Role::TabList);
                        Tabs::new("tabs")
                            .render(window, cx)
                            .into_element()
                            .write_a11y_info(&mut node);
                        *captured.lock().unwrap() = Some(node);
                    },
                    |_, _, _, _| {},
                )
            }
        }

        let captured: Captured = Arc::new(Mutex::new(None));
        let result = captured.clone();
        let (_, cx) = cx.add_window_view(move |_, _| Probe(captured));
        cx.update(|window, cx| window.draw(cx).clear(cx));
        assert_eq!(result.lock().unwrap().take().unwrap().role(), Role::TabList);
    }

    /// Four tabs, the third disabled, along `axis`, then a focusable element.
    struct ListHarness {
        axis: gpui::Axis,
        activate_on_focus: bool,
        selected: usize,
        tabs: Vec<gpui::FocusHandle>,
        after: gpui::FocusHandle,
    }

    impl Render for ListHarness {
        fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            let view = cx.entity().downgrade();
            div()
                .child(
                    Tabs::new("tabs")
                        .axis(self.axis)
                        .activate_on_focus(self.activate_on_focus)
                        .children(self.tabs.iter().enumerate().map(|(index, focus)| {
                            let view = view.clone();
                            Tab::new(index)
                                .track_focus(focus)
                                .selected(self.selected == index)
                                .disabled(index == 2)
                                .size(px(20.))
                                .on_click(move |_, _, cx| {
                                    view.update(cx, |this, cx| {
                                        this.selected = index;
                                        cx.notify();
                                    })
                                    .ok();
                                })
                        })),
                )
                .child(div().id("after").size(px(20.)).track_focus(&self.after))
        }
    }

    fn list(
        cx: &mut gpui::TestAppContext,
        axis: gpui::Axis,
        activate_on_focus: bool,
    ) -> (gpui::Entity<ListHarness>, &mut VisualTestContext) {
        let (view, cx) = cx.add_window_view(|_, cx| ListHarness {
            axis,
            activate_on_focus,
            selected: 1,
            tabs: (0..4).map(|_| cx.focus_handle()).collect(),
            after: cx.focus_handle().tab_stop(true),
        });
        cx.update(|window, cx| window.draw(cx).clear(cx));
        (view, cx)
    }

    /// Moves focus as Tab (or Shift-Tab) would, which `Root` binds in an app.
    fn tab(cx: &mut VisualTestContext, forward: bool) {
        cx.update(|window, cx| {
            if forward {
                window.focus_next(cx);
            } else {
                window.focus_prev(cx);
            }
        });
        cx.run_until_parked();
    }

    /// The focused tab and the selected one.
    fn state(
        view: &gpui::Entity<ListHarness>,
        cx: &mut VisualTestContext,
    ) -> (Option<usize>, usize) {
        cx.update(|window, cx| {
            let harness = view.read(cx);
            (
                harness
                    .tabs
                    .iter()
                    .position(|focus| focus.is_focused(window)),
                harness.selected,
            )
        })
    }

    #[gpui::test]
    fn the_list_is_one_tab_stop_and_arrows_select(cx: &mut gpui::TestAppContext) {
        let (view, cx) = list(cx, gpui::Axis::Horizontal, true);
        tab(cx, true);
        assert_eq!(state(&view, cx), (Some(1), 1));

        // The disabled third tab is skipped, and the ends wrap.
        cx.simulate_keystrokes("right");
        assert_eq!(state(&view, cx), (Some(3), 3));
        cx.simulate_keystrokes("right");
        assert_eq!(state(&view, cx), (Some(0), 0));
        cx.simulate_keystrokes("end");
        assert_eq!(state(&view, cx), (Some(3), 3));
        cx.simulate_keystrokes("home");
        assert_eq!(state(&view, cx), (Some(0), 0));
        // A horizontal list leaves Up and Down alone.
        cx.simulate_keystrokes("down up");
        assert_eq!(state(&view, cx), (Some(0), 0));

        tab(cx, true);
        assert!(cx.update(|window, cx| view.read(cx).after.is_focused(window)));
        tab(cx, false);
        assert_eq!(state(&view, cx), (Some(0), 0));
    }

    #[gpui::test]
    fn a_vertical_list_without_activation_on_focus_selects_on_enter(cx: &mut gpui::TestAppContext) {
        let (view, cx) = list(cx, gpui::Axis::Vertical, false);
        tab(cx, true);
        cx.simulate_keystrokes("down");
        assert_eq!(state(&view, cx), (Some(3), 1));
        cx.simulate_keystrokes("right");
        assert_eq!(state(&view, cx), (Some(3), 1));

        let keystroke = gpui::Keystroke::parse("enter").unwrap();
        cx.simulate_event(gpui::KeyDownEvent {
            keystroke: keystroke.clone(),
            is_held: false,
            prefer_character_input: false,
        });
        cx.simulate_event(gpui::KeyUpEvent { keystroke });
        assert_eq!(state(&view, cx), (Some(3), 3));
    }

    #[gpui::test]
    fn clicking_a_tab_selects_it_without_taking_focus(cx: &mut gpui::TestAppContext) {
        let (view, cx) = list(cx, gpui::Axis::Horizontal, true);
        cx.simulate_click(point(px(5.), px(5.)), Modifiers::default());
        assert_eq!(state(&view, cx), (None, 0));
    }
}
