use gpui::prelude::FluentBuilder as _;
use gpui::{
    div, px, relative, AnyElement, App, DefiniteLength, Entity, InteractiveElement as _,
    IntoElement, MouseButton, ParentElement as _, Rems, RenderOnce, Styled as _, Window,
};

use crate::button::{Button, ButtonVariants as _};
use crate::indicator::Indicator;
use crate::input::clear_button;
use crate::scroll::{Scrollbar, ScrollbarAxis};
use crate::ActiveTheme;
use crate::{h_flex, StyledExt};
use crate::{IconName, Size};
use crate::{Sizable, StyleSized};

use super::InputState;

#[derive(IntoElement)]
pub struct TextInput {
    state: Entity<InputState>,
    size: Size,
    no_gap: bool,
    prefix: Option<AnyElement>,
    suffix: Option<AnyElement>,
    height: Option<DefiniteLength>,
    appearance: bool,
    cleanable: bool,
    mask_toggle: bool,
    disabled: bool,
}

impl Sizable for TextInput {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }
}

impl TextInput {
    /// Create a new [`TextInput`] element bind to the [`InputState`].
    pub fn new(state: &Entity<InputState>) -> Self {
        Self {
            state: state.clone(),
            size: Size::default(),
            no_gap: false,
            prefix: None,
            suffix: None,
            height: None,
            appearance: true,
            cleanable: false,
            mask_toggle: false,
            disabled: false,
        }
    }

    pub fn prefix(mut self, prefix: impl IntoElement) -> Self {
        self.prefix = Some(prefix.into_any_element());
        self
    }

    pub fn suffix(mut self, suffix: impl IntoElement) -> Self {
        self.suffix = Some(suffix.into_any_element());
        self
    }

    /// Set full height of the input (Multi-line only).
    pub fn h_full(mut self) -> Self {
        self.height = Some(relative(1.));
        self
    }

    /// Set height of the input (Multi-line only).
    pub fn h(mut self, height: impl Into<DefiniteLength>) -> Self {
        self.height = Some(height.into());
        self
    }

    /// Set the appearance of the input field.
    pub fn appearance(mut self, appearance: bool) -> Self {
        self.appearance = appearance;
        self
    }

    /// Set true to show the clear button when the input field is not empty.
    pub fn cleanable(mut self) -> Self {
        self.cleanable = true;
        self
    }

    /// Set to enable toggle button for password mask state.
    pub fn mask_toggle(mut self) -> Self {
        self.mask_toggle = true;
        self
    }

    /// Set to disable the input field.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Set true to not use gap between input and prefix, suffix, and clear button.
    ///
    /// Default: false
    pub(super) fn no_gap(mut self) -> Self {
        self.no_gap = true;
        self
    }

    fn render_toggle_mask_button(state: Entity<InputState>) -> impl IntoElement {
        Button::new("toggle-mask")
            .icon(IconName::Eye)
            .xsmall()
            .ghost()
            .on_mouse_down(MouseButton::Left, {
                let state = state.clone();
                move |_, window, cx| {
                    state.update(cx, |state, cx| {
                        state.set_masked(false, window, cx);
                    })
                }
            })
            .on_mouse_up(MouseButton::Left, {
                let state = state.clone();
                move |_, window, cx| {
                    state.update(cx, |state, cx| {
                        state.set_masked(true, window, cx);
                    })
                }
            })
    }
}

impl RenderOnce for TextInput {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        const LINE_HEIGHT: Rems = Rems(1.25);

        self.state.update(cx, |state, _| {
            state.height = self.height;
            state.disabled = self.disabled;
        });

        let state = self.state.read(cx);
        let focused = state.focus_handle.is_focused(window);
        let mut gap_x = match self.size {
            Size::Small => px(4.),
            Size::Large => px(8.),
            _ => px(4.),
        };
        if self.no_gap {
            gap_x = px(0.);
        }

        let prefix = self.prefix;
        let suffix = self.suffix;
        let show_clear_button =
            self.cleanable && !state.loading && !state.text.is_empty() && state.is_single_line();
        let bg = if state.disabled {
            cx.theme().muted
        } else {
            cx.theme().background
        };

        div()
            .id(("input", self.state.entity_id()))
            .flex()
            .key_context(crate::input::CONTEXT)
            .track_focus(&state.focus_handle)
            .when(!state.disabled, |this| {
                this.on_action(window.listener_for(&self.state, InputState::backspace))
                    .on_action(window.listener_for(&self.state, InputState::delete))
                    .on_action(
                        window.listener_for(&self.state, InputState::delete_to_beginning_of_line),
                    )
                    .on_action(window.listener_for(&self.state, InputState::delete_to_end_of_line))
                    .on_action(window.listener_for(&self.state, InputState::delete_previous_word))
                    .on_action(window.listener_for(&self.state, InputState::delete_next_word))
                    .on_action(window.listener_for(&self.state, InputState::enter))
                    .on_action(window.listener_for(&self.state, InputState::escape))
            })
            .on_action(window.listener_for(&self.state, InputState::left))
            .on_action(window.listener_for(&self.state, InputState::right))
            .on_action(window.listener_for(&self.state, InputState::select_left))
            .on_action(window.listener_for(&self.state, InputState::select_right))
            .when(state.multi_line, |this| {
                this.on_action(window.listener_for(&self.state, InputState::up))
                    .on_action(window.listener_for(&self.state, InputState::down))
                    .on_action(window.listener_for(&self.state, InputState::select_up))
                    .on_action(window.listener_for(&self.state, InputState::select_down))
            })
            .on_action(window.listener_for(&self.state, InputState::select_all))
            .on_action(window.listener_for(&self.state, InputState::select_to_start_of_line))
            .on_action(window.listener_for(&self.state, InputState::select_to_end_of_line))
            .on_action(window.listener_for(&self.state, InputState::select_to_previous_word))
            .on_action(window.listener_for(&self.state, InputState::select_to_next_word))
            .on_action(window.listener_for(&self.state, InputState::home))
            .on_action(window.listener_for(&self.state, InputState::end))
            .on_action(window.listener_for(&self.state, InputState::move_to_start))
            .on_action(window.listener_for(&self.state, InputState::move_to_end))
            .on_action(window.listener_for(&self.state, InputState::move_to_previous_word))
            .on_action(window.listener_for(&self.state, InputState::move_to_next_word))
            .on_action(window.listener_for(&self.state, InputState::select_to_start))
            .on_action(window.listener_for(&self.state, InputState::select_to_end))
            .on_action(window.listener_for(&self.state, InputState::show_character_palette))
            .on_action(window.listener_for(&self.state, InputState::copy))
            .on_action(window.listener_for(&self.state, InputState::paste))
            .on_action(window.listener_for(&self.state, InputState::cut))
            .on_action(window.listener_for(&self.state, InputState::undo))
            .on_action(window.listener_for(&self.state, InputState::redo))
            .on_key_down(window.listener_for(&self.state, InputState::on_key_down))
            .on_mouse_down(
                MouseButton::Left,
                window.listener_for(&self.state, InputState::on_mouse_down),
            )
            .on_mouse_up(
                MouseButton::Left,
                window.listener_for(&self.state, InputState::on_mouse_up),
            )
            .on_scroll_wheel(window.listener_for(&self.state, InputState::on_scroll_wheel))
            .size_full()
            .line_height(LINE_HEIGHT)
            .input_py(self.size)
            .input_h(self.size)
            .cursor_text()
            .when(state.multi_line, |this| {
                this.h_auto()
                    .when_some(self.height, |this, height| this.h(height))
            })
            .when(self.appearance, |this| {
                this.bg(bg)
                    .border_color(cx.theme().input)
                    .border_1()
                    .rounded(cx.theme().radius)
                    .when(cx.theme().shadow, |this| this.shadow_sm())
                    .when(focused, |this| this.focused_border(cx))
            })
            .when(prefix.is_none(), |this| this.input_pl(self.size))
            .input_pr(self.size)
            .items_center()
            .gap(gap_x)
            .children(prefix)
            // TODO: Define height here, and use it in the input element
            .child(self.state.clone())
            .child(
                h_flex()
                    .id("suffix")
                    .absolute()
                    .gap(gap_x)
                    .when(self.appearance, |this| this.bg(bg))
                    .items_center()
                    .when(suffix.is_none(), |this| this.pr_1())
                    .right_0()
                    .when(state.loading, |this| {
                        this.child(Indicator::new().color(cx.theme().muted_foreground))
                    })
                    .when(self.mask_toggle, |this| {
                        this.child(Self::render_toggle_mask_button(self.state.clone()))
                    })
                    .when(show_clear_button, |this| {
                        this.child(clear_button(cx).on_click({
                            let state = self.state.clone();
                            move |_, window, cx| {
                                state.update(cx, |state, cx| {
                                    state.clean(window, cx);
                                })
                            }
                        }))
                    })
                    .children(suffix),
            )
            .when(state.is_multi_line(), |this| {
                let entity_id = self.state.entity_id();
                if state.last_layout.is_some() {
                    let scroll_size = state.scroll_size;

                    this.relative().child(
                        div()
                            .absolute()
                            .top_0()
                            .left_0()
                            .right(px(1.))
                            .bottom_0()
                            .child(
                                Scrollbar::vertical(
                                    entity_id,
                                    state.scrollbar_state.clone(),
                                    state.scroll_handle.clone(),
                                    scroll_size,
                                )
                                .axis(ScrollbarAxis::Vertical),
                            ),
                    )
                } else {
                    this
                }
            })
    }
}
