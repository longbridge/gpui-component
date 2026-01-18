use gpui::{
    App, AppContext, Context, Corner, Div, ElementId, Entity, EventEmitter, FocusHandle, Focusable,
    Hsla, InteractiveElement as _, IntoElement, KeyBinding, ParentElement, Render, RenderOnce,
    SharedString, Stateful, StatefulInteractiveElement as _, StyleRefinement, Styled, Subscription,
    Window, div, prelude::FluentBuilder as _,
};

use crate::{
    ActiveTheme as _, Colorize as _, Icon, Sizable, Size, StyleSized,
    actions::Confirm,
    button::{Button, ButtonVariants},
    divider::Divider,
    h_flex,
    input::{Input, InputEvent, InputState},
    popover::Popover,
    tooltip::Tooltip,
    v_flex,
};

const CONTEXT: &'static str = "ColorPicker";
pub(crate) fn init(cx: &mut App) {
    cx.bind_keys([KeyBinding::new(
        "enter",
        Confirm { secondary: false },
        Some(CONTEXT),
    )])
}

/// Events emitted by the [`ColorPicker`].
#[derive(Clone)]
pub enum ColorPickerEvent {
    Change(Option<Hsla>),
}

fn color_palettes() -> Vec<Vec<Hsla>> {
    use crate::theme::DEFAULT_COLORS;
    use itertools::Itertools as _;

    macro_rules! c {
        ($color:tt) => {
            DEFAULT_COLORS
                .$color
                .keys()
                .sorted()
                .map(|k| DEFAULT_COLORS.$color.get(k).map(|c| c.hsla).unwrap())
                .collect::<Vec<_>>()
        };
    }

    vec![
        c!(stone),
        c!(red),
        c!(orange),
        c!(yellow),
        c!(green),
        c!(cyan),
        c!(blue),
        c!(purple),
        c!(pink),
    ]
}

/// State of the [`ColorPicker`].
pub struct ColorPickerState {
    focus_handle: FocusHandle,
    value: Option<Hsla>,
    hovered_color: Option<Hsla>,
    state: Entity<InputState>,
    open: bool,
    _subscriptions: Vec<Subscription>,
}

impl ColorPickerState {
    /// Create a new [`ColorPickerState`].
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let state = cx.new(|cx| {
            InputState::new(window, cx).pattern(regex::Regex::new(r"^#[0-9a-fA-F]{0,8}$").unwrap())
        });

        let _subscriptions = vec![cx.subscribe_in(
            &state,
            window,
            |this, state, ev: &InputEvent, window, cx| match ev {
                InputEvent::Change => {
                    let value = state.read(cx).value();
                    if let Ok(color) = Hsla::parse_hex(value.as_str()) {
                        this.hovered_color = Some(color);
                    }
                }
                InputEvent::PressEnter { .. } => {
                    let val = this.state.read(cx).value();
                    if let Ok(color) = Hsla::parse_hex(&val) {
                        this.open = false;
                        this.update_value(Some(color), true, window, cx);
                    }
                }
                _ => {}
            },
        )];

        Self {
            focus_handle: cx.focus_handle(),
            value: None,
            hovered_color: None,
            state,
            open: false,
            _subscriptions,
        }
    }

    /// Set default color value.
    pub fn default_value(mut self, value: impl Into<Hsla>) -> Self {
        self.value = Some(value.into());
        self
    }

    /// Set current color value.
    pub fn set_value(
        &mut self,
        value: impl Into<Hsla>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.update_value(Some(value.into()), false, window, cx)
    }

    /// Get current color value.
    pub fn value(&self) -> Option<Hsla> {
        self.value
    }

    fn on_confirm(&mut self, _: &Confirm, _: &mut Window, cx: &mut Context<Self>) {
        self.open = !self.open;
        cx.notify();
    }

    fn update_value(
        &mut self,
        value: Option<Hsla>,
        emit: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.value = value;
        self.hovered_color = value;
        self.state.update(cx, |view, cx| {
            if let Some(value) = value {
                view.set_value(value.to_hex(), window, cx);
            } else {
                view.set_value("", window, cx);
            }
        });
        if emit {
            cx.emit(ColorPickerEvent::Change(value));
        }
        cx.notify();
    }
}

impl EventEmitter<ColorPickerEvent> for ColorPickerState {}

impl Render for ColorPickerState {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        self.state.clone()
    }
}

impl Focusable for ColorPickerState {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

/// A color picker element.
#[derive(IntoElement)]
pub struct ColorPicker {
    id: ElementId,
    style: StyleRefinement,
    state: Entity<ColorPickerState>,
    featured_colors: Option<Vec<Hsla>>,
    label: Option<SharedString>,
    icon: Option<Icon>,
    size: Size,
    anchor: Corner,
}

impl ColorPicker {
    /// Create a new color picker element with the given [`ColorPickerState`].
    pub fn new(state: &Entity<ColorPickerState>) -> Self {
        Self {
            id: ("color-picker", state.entity_id()).into(),
            style: StyleRefinement::default(),
            state: state.clone(),
            featured_colors: None,
            size: Size::Medium,
            label: None,
            icon: None,
            anchor: Corner::TopLeft,
        }
    }

    /// Set the featured colors to be displayed in the color picker.
    ///
    /// This is used to display a set of colors that the user can quickly select from,
    /// for example provided user's last used colors.
    pub fn featured_colors(mut self, colors: Vec<Hsla>) -> Self {
        self.featured_colors = Some(colors);
        self
    }

    /// Set the icon to the color picker button.
    ///
    /// If this is set the color picker button will display the icon.
    /// Else it will display the square color of the current value.
    pub fn icon(mut self, icon: impl Into<Icon>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    /// Set the label to be displayed above the color picker.
    ///
    /// Default is `None`.
    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Set the anchor corner of the color picker.
    ///
    /// Default is `Corner::TopLeft`.
    pub fn anchor(mut self, anchor: Corner) -> Self {
        self.anchor = anchor;
        self
    }

    fn render_item(
        &self,
        color: Hsla,
        clickable: bool,
        window: &mut Window,
        _: &mut App,
    ) -> Stateful<Div> {
        let state = self.state.clone();
        div()
            .id(SharedString::from(format!("color-{}", color.to_hex())))
            .h_5()
            .w_5()
            .bg(color)
            .border_1()
            .border_color(color.darken(0.1))
            .when(clickable, |this| {
                this.hover(|this| {
                    this.border_color(color.darken(0.3))
                        .bg(color.lighten(0.1))
                        .shadow_xs()
                })
                .active(|this| this.border_color(color.darken(0.5)).bg(color.darken(0.2)))
                .on_mouse_move(window.listener_for(&state, move |state, _, window, cx| {
                    state.hovered_color = Some(color);
                    state.state.update(cx, |input, cx| {
                        input.set_value(color.to_hex(), window, cx);
                    });
                    cx.notify();
                }))
                .on_click(window.listener_for(
                    &state,
                    move |state, _, window, cx| {
                        state.open = false;
                        state.update_value(Some(color), true, window, cx);
                        cx.notify();
                    },
                ))
            })
    }

    fn render_colors(&self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let featured_colors = self.featured_colors.clone().unwrap_or(vec![
            cx.theme().red,
            cx.theme().red_light,
            cx.theme().blue,
            cx.theme().blue_light,
            cx.theme().green,
            cx.theme().green_light,
            cx.theme().yellow,
            cx.theme().yellow_light,
            cx.theme().cyan,
            cx.theme().cyan_light,
            cx.theme().magenta,
            cx.theme().magenta_light,
        ]);

        v_flex()
            .p_0p5()
            .gap_3()
            .child(
                h_flex().gap_1().children(
                    featured_colors
                        .iter()
                        .map(|color| self.render_item(*color, true, window, cx)),
                ),
            )
            .child(Divider::horizontal())
            .child(
                v_flex()
                    .gap_1()
                    .children(color_palettes().iter().map(|sub_colors| {
                        h_flex().gap_1().children(
                            sub_colors
                                .iter()
                                .rev()
                                .map(|color| self.render_item(*color, true, window, cx)),
                        )
                    })),
            )
            .when_some(self.state.read(cx).hovered_color, |this, hovered_color| {
                this.child(Divider::horizontal()).child(
                    h_flex()
                        .gap_2()
                        .items_center()
                        .child(
                            div()
                                .bg(hovered_color)
                                .flex_shrink_0()
                                .border_1()
                                .border_color(hovered_color.darken(0.2))
                                .size_5()
                                .rounded(cx.theme().radius),
                        )
                        .child(Input::new(&self.state.read(cx).state).small()),
                )
            })
    }
}

impl Sizable for ColorPicker {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }
}

impl Focusable for ColorPicker {
    fn focus_handle(&self, cx: &App) -> FocusHandle {
        self.state.read(cx).focus_handle.clone()
    }
}

impl Styled for ColorPicker {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for ColorPicker {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let state = self.state.read(cx);
        let display_title: SharedString = if let Some(value) = state.value {
            value.to_hex()
        } else {
            "".to_string()
        }
        .into();

        let focus_handle = state.focus_handle.clone().tab_stop(true);

        div()
            .id(self.id.clone())
            .key_context(CONTEXT)
            .track_focus(&focus_handle)
            .on_action(window.listener_for(&self.state, ColorPickerState::on_confirm))
            .child(
                Popover::new("popover")
                    .open(state.open)
                    .w_72()
                    .on_open_change(
                        window.listener_for(&self.state, |this, open: &bool, _, cx| {
                            this.open = *open;
                            cx.notify();
                        }),
                    )
                    .trigger(
                        Button::new("trigger")
                            .with_size(self.size)
                            .text()
                            .when_some(self.icon.clone(), |this, icon| this.icon(icon.clone()))
                            .when_none(&self.icon, |this| {
                                this.p_0().child(
                                    div()
                                        .id("square")
                                        .bg(cx.theme().background)
                                        .m_1()
                                        .border_1()
                                        .m_1()
                                        .border_color(cx.theme().input)
                                        .when(cx.theme().shadow, |this| this.shadow_xs())
                                        .rounded(cx.theme().radius)
                                        .overflow_hidden()
                                        .size_with(self.size)
                                        .when_some(state.value, |this, value| {
                                            this.bg(value)
                                                .border_color(value.darken(0.3))
                                                .when(state.open, |this| this.border_2())
                                        })
                                        .when(!display_title.is_empty(), |this| {
                                            this.tooltip(move |_, cx| {
                                                cx.new(|_| Tooltip::new(display_title.clone()))
                                                    .into()
                                            })
                                        }),
                                )
                            })
                            .when_some(self.label.clone(), |this, label| this.child(label)),
                    )
                    .child(self.render_colors(window, cx)),
            )
    }
}
