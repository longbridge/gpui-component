use gpui::{
    AnyElement, App, Axis, Div, ElementId, FocusHandle, InteractiveElement, Interactivity,
    IntoElement, ParentElement, RenderOnce, Role, Stateful, StatefulInteractiveElement,
    StyleRefinement, Styled, Window, accesskit, div,
};
use smallvec::SmallVec;

use crate::{
    StyledExt as _,
    roving_focus::{Keys, RovingFocus},
};

/// An unstyled container for a set of toggle elements.
///
/// The group owns the keyboard for the [`Toggle`](crate::Toggle)s below it,
/// wherever they sit: it is one Tab stop, held by the focused toggle,
/// otherwise the toggle focused last, otherwise the first pressed one,
/// otherwise the first. The arrow keys of its [axis](Self::axis) move focus to
/// the previous or next enabled toggle, wrapping at either end, and Home and
/// End move to the first and last. Moving focus presses nothing; Space and
/// Enter press the focused toggle.
#[derive(IntoElement)]
pub struct ToggleGroup {
    id: ElementId,
    base: Stateful<Div>,
    style: StyleRefinement,
    axis: Axis,
    focus_handle: Option<FocusHandle>,
    children: SmallVec<[AnyElement; 4]>,
}

impl ToggleGroup {
    pub fn new(id: impl Into<ElementId>) -> Self {
        let id = id.into();
        Self {
            base: div().id(id.clone()),
            id,
            style: StyleRefinement::default(),
            axis: Axis::Horizontal,
            focus_handle: None,
            children: SmallVec::new(),
        }
    }

    /// Sets the semantic axis of the group.
    pub fn axis(mut self, axis: Axis) -> Self {
        self.axis = axis;
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

impl Styled for ToggleGroup {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl ParentElement for ToggleGroup {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl InteractiveElement for ToggleGroup {
    fn interactivity(&mut self) -> &mut Interactivity {
        self.base.interactivity()
    }
}

impl StatefulInteractiveElement for ToggleGroup {}

impl RenderOnce for ToggleGroup {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let keys = Keys::toggles(self.axis);
        let group = self
            .base
            .role(Role::Toolbar)
            .aria_orientation(match self.axis {
                Axis::Horizontal => accesskit::Orientation::Horizontal,
                Axis::Vertical => accesskit::Orientation::Vertical,
            })
            .children(self.children)
            .refine_style(&self.style);
        RovingFocus::new(&self.id, keys, window, cx).group(group, self.focus_handle)
    }
}

#[cfg(test)]
mod tests {
    use gpui::{Context, FocusHandle, Render, TestAppContext, VisualTestContext, div, px};

    use super::*;
    use crate::Toggle;

    /// Four toggles, the second disabled, along `axis`, then a focusable element.
    struct Harness {
        axis: Axis,
        pressed: Vec<bool>,
        toggles: Vec<FocusHandle>,
        after: FocusHandle,
    }

    impl Render for Harness {
        fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            let view = cx.entity().downgrade();
            div()
                .child(ToggleGroup::new("group").axis(self.axis).children(
                    self.toggles.iter().enumerate().map(|(index, focus)| {
                        let view = view.clone();
                        Toggle::new(index)
                            .track_focus(focus)
                            .pressed(self.pressed[index])
                            .disabled(index == 1)
                            .size(px(20.))
                            .on_change(move |pressed, _, _, cx| {
                                view.update(cx, |this, cx| {
                                    this.pressed[index] = pressed;
                                    cx.notify();
                                })
                                .ok();
                            })
                    }),
                ))
                .child(div().id("after").size(px(20.)).track_focus(&self.after))
        }
    }

    fn harness(
        cx: &mut TestAppContext,
        axis: Axis,
        pressed: Vec<bool>,
    ) -> (gpui::Entity<Harness>, &mut VisualTestContext) {
        let (view, cx) = cx.add_window_view(|_, cx| Harness {
            axis,
            pressed,
            toggles: (0..4).map(|_| cx.focus_handle()).collect(),
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

    fn focused(view: &gpui::Entity<Harness>, cx: &mut VisualTestContext) -> Option<usize> {
        cx.update(|window, cx| {
            view.read(cx)
                .toggles
                .iter()
                .position(|focus| focus.is_focused(window))
        })
    }

    #[gpui::test]
    fn arrows_of_the_axis_and_home_end_move_focus_without_pressing(cx: &mut TestAppContext) {
        let (view, cx) = harness(cx, Axis::Horizontal, vec![false; 4]);
        tab(cx, true);
        assert_eq!(focused(&view, cx), Some(0));

        // The second toggle is disabled and skipped; the ends wrap.
        cx.simulate_keystrokes("right");
        assert_eq!(focused(&view, cx), Some(2));
        cx.simulate_keystrokes("right right");
        assert_eq!(focused(&view, cx), Some(0));
        cx.simulate_keystrokes("left");
        assert_eq!(focused(&view, cx), Some(3));
        cx.simulate_keystrokes("home");
        assert_eq!(focused(&view, cx), Some(0));
        cx.simulate_keystrokes("end");
        assert_eq!(focused(&view, cx), Some(3));
        // A horizontal group leaves Up and Down alone.
        cx.simulate_keystrokes("up down");
        assert_eq!(focused(&view, cx), Some(3));
        assert_eq!(view.read_with(cx, |h, _| h.pressed.clone()), vec![false; 4]);
    }

    #[gpui::test]
    fn tab_returns_to_the_toggle_focused_last(cx: &mut TestAppContext) {
        let (view, cx) = harness(cx, Axis::Vertical, vec![false, false, true, false]);
        // Without a remembered toggle, the pressed one holds the Tab stop.
        tab(cx, true);
        assert_eq!(focused(&view, cx), Some(2));
        cx.simulate_keystrokes("down");
        assert_eq!(focused(&view, cx), Some(3));
        tab(cx, true);
        assert_eq!(focused(&view, cx), None);
        tab(cx, false);
        assert_eq!(focused(&view, cx), Some(3));
    }
}
