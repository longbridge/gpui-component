use std::ops::Range;

use gpui::{
    AnyElement, App, Context, ElementId, Entity, FollowMode, InteractiveElement as _, IntoElement,
    ListAlignment, ListOffset, ListState, ParentElement as _, RenderOnce, SharedString,
    StyleRefinement, Styled, Window, div, list, prelude::FluentBuilder as _, px,
};

use crate::{ActiveTheme as _, IconName, Sizable as _, StyledExt as _, button::Button};
use crate::{button::ButtonVariants as _, scroll::ScrollableElement as _};

const LIST_OVERDRAW: gpui::Pixels = px(400.);

/// The entity-owned scrolling state for a [`MessageScroller`].
///
/// The state owns only GPUI's virtual-list bookkeeping. Message data remains
/// with the caller and is read by the row renderer passed to
/// [`MessageScroller::new`].
pub struct MessageScrollerState {
    list_state: ListState,
}

impl MessageScrollerState {
    /// Create a state for `item_count` rows and enable tail following.
    ///
    /// The constructor receives the entity context so the list's scroll
    /// handler can safely defer its entity update until GPUI has released the
    /// list's internal borrow.
    pub fn new(item_count: usize, cx: &mut Context<Self>) -> Self {
        let list_state = ListState::new(item_count, ListAlignment::Top, LIST_OVERDRAW);
        list_state.set_follow_mode(FollowMode::Tail);

        let weak_state = cx.weak_entity();
        list_state.set_scroll_handler(move |_, _, cx| {
            let weak_state = weak_state.clone();

            cx.defer(move |cx| {
                let _ = weak_state.update(cx, |_, cx| cx.notify());
            });
        });

        Self { list_state }
    }

    /// Return the current number of rows known by the virtual list.
    pub fn item_count(&self) -> usize {
        self.list_state.item_count()
    }

    /// Return whether the user has scrolled away from the latest content.
    pub fn is_scrolled_up(&self) -> bool {
        self.list_state.max_offset_for_scrollbar().y > px(0.)
            && !self.list_state.is_following_tail()
            && !self.list_state.is_scrolled_to_end().unwrap_or(false)
    }

    /// Return whether the list is actively following its tail.
    pub fn is_following_tail(&self) -> bool {
        self.list_state.is_following_tail()
    }

    /// Reset the list to `item_count` rows.
    pub fn reset(&mut self, item_count: usize, cx: &mut Context<Self>) {
        self.list_state.reset(item_count);
        self.list_state.set_follow_mode(FollowMode::Tail);
        cx.notify();
    }

    /// Replace `old_range` with `count` new rows.
    ///
    /// Returns `false` when the range is outside the current list and leaves
    /// the state unchanged.
    pub fn splice(
        &mut self,
        old_range: Range<usize>,
        count: usize,
        cx: &mut Context<Self>,
    ) -> bool {
        if !self.valid_range(&old_range) {
            return false;
        }

        self.list_state.splice(old_range, count);
        cx.notify();
        true
    }

    /// Append `count` rows to the end of the list.
    pub fn append(&mut self, count: usize, cx: &mut Context<Self>) -> bool {
        let item_count = self.list_state.item_count();
        self.splice(item_count..item_count, count, cx)
    }

    /// Prepend `count` rows while preserving the current scroll anchor.
    pub fn prepend(&mut self, count: usize, cx: &mut Context<Self>) -> bool {
        self.splice(0..0, count, cx)
    }

    /// Mark all rows for remeasurement while preserving a proportional anchor.
    pub fn remeasure(&mut self, cx: &mut Context<Self>) {
        self.list_state.remeasure();
        cx.notify();
    }

    /// Mark rows in `range` for remeasurement while preserving an item anchor.
    ///
    /// Returns `false` when the range is outside the current list.
    pub fn remeasure_items(&mut self, range: Range<usize>, cx: &mut Context<Self>) -> bool {
        if !self.valid_range(&range) {
            return false;
        }

        self.list_state.remeasure_items(range);
        cx.notify();
        true
    }

    /// Scroll to the row at `index`, if it exists.
    pub fn scroll_to_item(&mut self, index: usize, cx: &mut Context<Self>) -> bool {
        if index >= self.list_state.item_count() {
            return false;
        }

        self.list_state.scroll_to(ListOffset {
            item_ix: index,
            offset_in_item: px(0.),
        });
        cx.notify();
        true
    }

    /// Scroll to the row that represents an unread boundary.
    pub fn scroll_to_unread(&mut self, index: usize, cx: &mut Context<Self>) -> bool {
        self.scroll_to_item(index, cx)
    }

    /// Resume tail following and scroll to the latest row.
    pub fn scroll_to_end(&mut self, cx: &mut Context<Self>) {
        self.list_state.set_follow_mode(FollowMode::Tail);
        self.list_state.scroll_to_end();
        cx.notify();
    }

    fn valid_range(&self, range: &Range<usize>) -> bool {
        range.start <= range.end && range.end <= self.list_state.item_count()
    }
}

/// A virtualized message list with optional scrollbar and jump-to-latest UI.
#[derive(IntoElement)]
pub struct MessageScroller {
    id: ElementId,
    state: Entity<MessageScrollerState>,
    renderer: Box<dyn FnMut(usize, &mut Window, &mut App) -> AnyElement + 'static>,
    style: StyleRefinement,
    content_style: StyleRefinement,
    list_style: StyleRefinement,
    scrollbar: bool,
    jump_button: bool,
    jump_button_label: SharedString,
}

impl MessageScroller {
    /// Create a message scroller with a renderer for each row.
    pub fn new<E>(
        id: impl Into<ElementId>,
        state: Entity<MessageScrollerState>,
        renderer: impl FnMut(usize, &mut Window, &mut App) -> E + 'static,
    ) -> Self
    where
        E: IntoElement,
    {
        let mut renderer = renderer;
        Self {
            id: id.into(),
            state,
            renderer: Box::new(move |index, window, cx| {
                renderer(index, window, cx).into_any_element()
            }),
            style: StyleRefinement::default(),
            content_style: StyleRefinement::default(),
            list_style: StyleRefinement::default(),
            scrollbar: true,
            jump_button: true,
            jump_button_label: "Jump to latest".into(),
        }
    }

    /// Enable or disable the virtual-list scrollbar.
    pub fn scrollbar(mut self, scrollbar: bool) -> Self {
        self.scrollbar = scrollbar;
        self
    }

    /// Enable or disable the built-in jump-to-latest button.
    pub fn jump_button(mut self, jump_button: bool) -> Self {
        self.jump_button = jump_button;
        self
    }

    /// Set the label used by the built-in jump-to-latest button.
    pub fn with_jump_button_label(mut self, label: impl Into<SharedString>) -> Self {
        self.jump_button_label = label.into();
        self
    }

    /// Refine the viewport that contains the list and scrollbar.
    pub fn with_content_style(mut self, style: StyleRefinement) -> Self {
        self.content_style = style;
        self
    }

    /// Refine the GPUI list element used to render rows.
    pub fn with_list_style(mut self, style: StyleRefinement) -> Self {
        self.list_style = style;
        self
    }
}

impl Styled for MessageScroller {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for MessageScroller {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let root_id = self.id.clone();
        let (list_state, show_jump_button) = {
            let state = self.state.read(cx);
            (
                state.list_state.clone(),
                self.jump_button && state.is_scrolled_up(),
            )
        };
        let tokens = cx.theme().semantic_tokens();

        let list = list(list_state.clone(), self.renderer)
            .size_full()
            .min_h_0()
            .gap(tokens.spacing.md)
            .px(tokens.spacing.md)
            .py(tokens.spacing.sm)
            .refine_style(&self.list_style);

        let viewport = div()
            .id((root_id.clone(), "viewport"))
            .size_full()
            .min_h_0()
            .min_w_0()
            .child(list)
            .when(self.scrollbar, |this| this.vertical_scrollbar(&list_state))
            .refine_style(&self.content_style);

        div()
            .id(root_id.clone())
            .relative()
            .size_full()
            .min_h_0()
            .overflow_hidden()
            .child(viewport)
            .when(show_jump_button, |this| {
                let state = self.state.clone();

                this.child(
                    div()
                        .absolute()
                        .left_0()
                        .right_0()
                        .bottom(tokens.spacing.lg)
                        .flex()
                        .justify_center()
                        .child(
                            Button::new((root_id, "jump-to-latest"))
                                .ghost()
                                .xsmall()
                                .icon(IconName::ArrowDown)
                                .label(self.jump_button_label)
                                .rounded(tokens.radius.full)
                                .border_1()
                                .border_color(tokens.colors.border)
                                .bg(tokens.colors.background)
                                .on_click(move |_, _, cx| {
                                    state.update(cx, |state, cx| state.scroll_to_end(cx));
                                }),
                        ),
                )
            })
            .refine_style(&self.style)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::AppContext as _;

    #[gpui::test]
    fn test_message_scroller_state_builder(cx: &mut gpui::TestAppContext) {
        let state = cx.new(|cx| MessageScrollerState::new(3, cx));

        cx.update(|cx| {
            assert_eq!(state.read(cx).item_count(), 3);
            assert!(!state.read(cx).is_scrolled_up());
            assert!(state.read(cx).is_following_tail());

            state.update(cx, |state, cx| {
                assert!(!state.scroll_to_item(3, cx));
                assert!(state.append(2, cx));
                assert_eq!(state.item_count(), 5);
                assert!(state.prepend(1, cx));
                assert_eq!(state.item_count(), 6);
                assert!(!state.splice(5..7, 0, cx));
                assert!(state.remeasure_items(0..6, cx));
                assert!(!state.remeasure_items(6..7, cx));
                assert!(state.scroll_to_unread(2, cx));
                assert!(!state.is_scrolled_up());
                assert!(!state.is_following_tail());
                state.scroll_to_end(cx);
                assert!(state.is_following_tail());
                state.reset(2, cx);
                assert_eq!(state.item_count(), 2);
                assert!(state.is_following_tail());
            });
        });
    }

    #[gpui::test]
    fn test_message_scroller_builder(cx: &mut gpui::TestAppContext) {
        let state = cx.new(|cx| MessageScrollerState::new(0, cx));
        let scroller = MessageScroller::new("message-scroller", state, |_, _, _| div())
            .scrollbar(false)
            .jump_button(false)
            .with_jump_button_label("Latest")
            .with_content_style(StyleRefinement::default())
            .with_list_style(StyleRefinement::default());

        assert!(!scroller.scrollbar);
        assert!(!scroller.jump_button);
        assert_eq!(scroller.jump_button_label, "Latest");
    }
}
