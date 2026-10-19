use gpui::{
    AnyElement, App, Axis, Div, ElementId, FocusHandle, InteractiveElement, Interactivity,
    IntoElement, ParentElement, RenderOnce, Role, Stateful, StatefulInteractiveElement,
    StyleRefinement, Styled, Window, accesskit, div, prelude::FluentBuilder as _,
};
use smallvec::SmallVec;

use crate::{
    StyledExt as _,
    roving_focus::{Keys, RovingFocus},
};

/// An unstyled container for a set of radio elements.
///
/// The group owns the keyboard for the [`Radio`](crate::Radio)s below it,
/// wherever they sit, as platform radio groups do: it is one Tab stop, held
/// by the focused radio, otherwise the checked one, otherwise the first. Every
/// arrow key moves focus to the previous (Left, Up) or next (Right, Down)
/// enabled radio, wrapping at either end, and requests its check through the
/// radio's [`on_change`](crate::Radio::on_change), with a keyboard click event.
#[derive(IntoElement)]
pub struct RadioGroup {
    id: ElementId,
    base: Stateful<Div>,
    style: StyleRefinement,
    axis: Axis,
    keyboard_navigation: bool,
    focus_handle: Option<FocusHandle>,
    children: SmallVec<[AnyElement; 4]>,
}

impl RadioGroup {
    pub fn new(id: impl Into<ElementId>) -> Self {
        let id = id.into();
        Self {
            base: div().id(id.clone()),
            id,
            style: StyleRefinement::default(),
            axis: Axis::Vertical,
            keyboard_navigation: true,
            focus_handle: None,
            children: SmallVec::new(),
        }
    }

    /// Sets the semantic axis of the group.
    pub fn axis(mut self, axis: Axis) -> Self {
        self.axis = axis;
        self
    }

    /// Sets whether the group owns its radios' keyboard navigation. Defaults
    /// to `true`. Turn it off when the owner moves between radios itself;
    /// each enabled radio is then its own Tab stop.
    pub fn keyboard_navigation(mut self, keyboard_navigation: bool) -> Self {
        self.keyboard_navigation = keyboard_navigation;
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

impl Styled for RadioGroup {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl ParentElement for RadioGroup {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl InteractiveElement for RadioGroup {
    fn interactivity(&mut self) -> &mut Interactivity {
        self.base.interactivity()
    }
}

impl StatefulInteractiveElement for RadioGroup {}

impl RenderOnce for RadioGroup {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let keys = Keys::radios();
        let group = self
            .base
            .role(Role::RadioGroup)
            .aria_orientation(match self.axis {
                Axis::Horizontal => accesskit::Orientation::Horizontal,
                Axis::Vertical => accesskit::Orientation::Vertical,
            })
            .children(self.children)
            .refine_style(&self.style);
        if self.keyboard_navigation {
            RovingFocus::new(&self.id, keys, window, cx)
                .group(group, self.focus_handle)
                .into_any_element()
        } else {
            group
                .when_some(self.focus_handle, |group, focus_handle| {
                    group.track_focus(&focus_handle)
                })
                .into_any_element()
        }
    }
}

#[cfg(test)]
mod tests {
    use gpui::{Context, FocusHandle, Render, TestAppContext, VisualTestContext, div, px};

    use super::*;
    use crate::Radio;

    /// Four radios, the third disabled, and a focusable element after them.
    struct Harness {
        checked: usize,
        radios: Vec<FocusHandle>,
        after: FocusHandle,
        own: FocusHandle,
        keyboard_navigation: bool,
    }

    impl Render for Harness {
        fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            let view = cx.entity().downgrade();
            div()
                .child(
                    RadioGroup::new("group")
                        .track_focus(&self.own)
                        .keyboard_navigation(self.keyboard_navigation)
                        .children(self.radios.iter().enumerate().map(|(index, focus)| {
                            let view = view.clone();
                            Radio::new(index)
                                .track_focus(focus)
                                .checked(self.checked == index)
                                .disabled(index == 2)
                                .size(px(20.))
                                .on_change(move |_, _, _, cx| {
                                    view.update(cx, |this, cx| {
                                        this.checked = index;
                                        cx.notify();
                                    })
                                    .ok();
                                })
                        })),
                )
                .child(div().id("after").size(px(20.)).track_focus(&self.after))
        }
    }

    fn harness(
        cx: &mut TestAppContext,
        keyboard_navigation: bool,
    ) -> (gpui::Entity<Harness>, &mut VisualTestContext) {
        let (view, cx) = cx.add_window_view(|_, cx| Harness {
            checked: 1,
            radios: (0..4).map(|_| cx.focus_handle()).collect(),
            after: cx.focus_handle().tab_stop(true),
            own: cx.focus_handle(),
            keyboard_navigation,
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

    /// The focused radio's index, or `None` when focus is elsewhere.
    fn focused(view: &gpui::Entity<Harness>, cx: &mut VisualTestContext) -> Option<usize> {
        cx.update(|window, cx| {
            view.read(cx)
                .radios
                .iter()
                .position(|focus| focus.is_focused(window))
        })
    }

    fn checked(view: &gpui::Entity<Harness>, cx: &mut VisualTestContext) -> usize {
        view.read_with(cx, |harness, _| harness.checked)
    }

    #[gpui::test]
    fn the_group_is_one_tab_stop_on_the_checked_radio(cx: &mut TestAppContext) {
        let (view, cx) = harness(cx, true);
        tab(cx, true);
        assert_eq!(focused(&view, cx), Some(1));
        tab(cx, true);
        assert_eq!(focused(&view, cx), None);
        assert!(cx.update(|window, cx| view.read(cx).after.is_focused(window)));
        tab(cx, false);
        assert_eq!(focused(&view, cx), Some(1));
    }

    #[gpui::test]
    fn every_arrow_moves_focus_and_checks_skipping_disabled(cx: &mut TestAppContext) {
        let (view, cx) = harness(cx, true);
        tab(cx, true);

        // The third radio is disabled, so Down from the second lands on the fourth.
        cx.simulate_keystrokes("down");
        assert_eq!((focused(&view, cx), checked(&view, cx)), (Some(3), 3));
        // Past the last radio, focus wraps to the first.
        cx.simulate_keystrokes("right");
        assert_eq!((focused(&view, cx), checked(&view, cx)), (Some(0), 0));
        cx.simulate_keystrokes("up");
        assert_eq!((focused(&view, cx), checked(&view, cx)), (Some(3), 3));
        cx.simulate_keystrokes("left");
        assert_eq!((focused(&view, cx), checked(&view, cx)), (Some(1), 1));

        // Modified arrows are left to the application.
        cx.simulate_keystrokes("shift-down");
        assert_eq!((focused(&view, cx), checked(&view, cx)), (Some(1), 1));

        // Leaving and coming back returns to the checked radio.
        tab(cx, true);
        tab(cx, false);
        assert_eq!(focused(&view, cx), Some(1));
    }

    #[gpui::test]
    fn without_keyboard_navigation_each_radio_is_a_tab_stop(cx: &mut TestAppContext) {
        let (view, cx) = harness(cx, false);
        tab(cx, true);
        assert_eq!(focused(&view, cx), Some(0));
        cx.simulate_keystrokes("down");
        assert_eq!((focused(&view, cx), checked(&view, cx)), (Some(0), 1));
        tab(cx, true);
        assert_eq!(focused(&view, cx), Some(1));
    }

    #[gpui::test]
    fn focusing_the_group_focuses_its_tab_stop(cx: &mut TestAppContext) {
        let (view, cx) = harness(cx, true);
        cx.update(|window, cx| {
            let own = view.read(cx).own.clone();
            window.focus(&own, cx);
        });
        cx.run_until_parked();
        assert_eq!(focused(&view, cx), Some(1));
        assert!(cx.update(|window, cx| view.read(cx).own.contains_focused(window, cx)));
        cx.simulate_keystrokes("down");
        assert_eq!((focused(&view, cx), checked(&view, cx)), (Some(3), 3));
    }
}
