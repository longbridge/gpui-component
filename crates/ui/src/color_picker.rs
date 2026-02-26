use gpui::{
    App, AppContext, Context, Corner, Div, ElementId, Entity, EventEmitter, FocusHandle, Focusable,
    Hsla, InteractiveElement as _, IntoElement, KeyBinding, ParentElement, Render, RenderOnce,
    SharedString, Stateful, StatefulInteractiveElement as _, StyleRefinement, Styled, Subscription,
    TextAlign, Window, div, hsla, linear_color_stop, linear_gradient, prelude::FluentBuilder as _,
};
use rust_i18n::t;

use crate::{
    ActiveTheme as _, Colorize as _, Icon, Selectable, Sizable, Size, StyleSized,
    actions::Confirm,
    divider::Divider,
    h_flex,
    input::{Input, InputEvent, InputState},
    popover::Popover,
    slider::{Slider, SliderEvent, SliderState},
    tab::{Tab, TabBar},
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

#[derive(Clone)]
struct HslaSliders {
    hue: Entity<SliderState>,
    saturation: Entity<SliderState>,
    lightness: Entity<SliderState>,
    alpha: Entity<SliderState>,
}

impl HslaSliders {
    fn new(cx: &mut App) -> Self {
        Self {
            hue: cx.new(|_| {
                SliderState::new()
                    .min(0.)
                    .max(1.)
                    .step(0.01)
                    .default_value(0.)
            }),
            saturation: cx.new(|_| {
                SliderState::new()
                    .min(0.)
                    .max(1.)
                    .step(0.01)
                    .default_value(0.)
            }),
            lightness: cx.new(|_| {
                SliderState::new()
                    .min(0.)
                    .max(1.)
                    .step(0.01)
                    .default_value(0.)
            }),
            alpha: cx.new(|_| {
                SliderState::new()
                    .min(0.)
                    .max(1.)
                    .step(0.01)
                    .default_value(0.)
            }),
        }
    }

    fn read(&self, cx: &App) -> Hsla {
        hsla(
            self.hue.read(cx).value().start(),
            self.saturation.read(cx).value().start(),
            self.lightness.read(cx).value().start(),
            self.alpha.read(cx).value().start(),
        )
    }

    fn update(&self, new_color: Hsla, window: &mut Window, cx: &mut App) {
        self.hue.update(cx, |slider, cx| {
            slider.set_value(new_color.h, window, cx);
        });
        self.saturation.update(cx, |slider, cx| {
            slider.set_value(new_color.s, window, cx);
        });
        self.lightness.update(cx, |slider, cx| {
            slider.set_value(new_color.l, window, cx);
        });
        self.alpha.update(cx, |slider, cx| {
            slider.set_value(new_color.a, window, cx);
        });
    }
}

/// State of the [`ColorPicker`].
pub struct ColorPickerState {
    focus_handle: FocusHandle,
    value: Option<Hsla>,
    hovered_color: Option<Hsla>,
    state: Entity<InputState>,
    hsla_sliders: HslaSliders,
    needs_slider_sync: bool,
    suppress_input_change: bool,
    active_tab: usize,
    open: bool,
    _subscriptions: Vec<Subscription>,
}

impl ColorPickerState {
    /// Create a new [`ColorPickerState`].
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let state = cx.new(|cx| {
            InputState::new(window, cx).pattern(regex::Regex::new(r"^#[0-9a-fA-F]{0,8}$").unwrap())
        });
        let hsla_sliders = HslaSliders::new(cx);

        let mut _subscriptions = vec![
            cx.subscribe_in(
                &state,
                window,
                |this, state, ev: &InputEvent, window, cx| match ev {
                    InputEvent::Change => {
                        if this.suppress_input_change {
                            return;
                        }
                        let value = state.read(cx).value();
                        if let Ok(color) = Hsla::parse_hex(value.as_str()) {
                            this.hovered_color = Some(color);
                            this.sync_sliders(Some(color), window, cx);
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
            ),
            cx.subscribe_in(
                &hsla_sliders.hue,
                window,
                |this, _, _: &SliderEvent, window, cx| {
                    let color = this.hsla_sliders.read(cx);
                    this.update_value_from_slider(color, true, window, cx);
                },
            ),
            cx.subscribe_in(
                &hsla_sliders.saturation,
                window,
                |this, _, _: &SliderEvent, window, cx| {
                    let color = this.hsla_sliders.read(cx);
                    this.update_value_from_slider(color, true, window, cx);
                },
            ),
            cx.subscribe_in(
                &hsla_sliders.lightness,
                window,
                |this, _, _: &SliderEvent, window, cx| {
                    let color = this.hsla_sliders.read(cx);
                    this.update_value_from_slider(color, true, window, cx);
                },
            ),
            cx.subscribe_in(
                &hsla_sliders.alpha,
                window,
                |this, _, _: &SliderEvent, window, cx| {
                    let color = this.hsla_sliders.read(cx);
                    this.update_value_from_slider(color, true, window, cx);
                },
            ),
        ];

        Self {
            focus_handle: cx.focus_handle(),
            value: None,
            hovered_color: None,
            state,
            hsla_sliders,
            needs_slider_sync: false,
            suppress_input_change: false,
            active_tab: 0,
            open: false,
            _subscriptions,
        }
    }

    /// Set default color value.
    pub fn default_value(mut self, value: impl Into<Hsla>) -> Self {
        let value = value.into();
        self.value = Some(value);
        self.hovered_color = Some(value);
        self.needs_slider_sync = true;
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
        self.needs_slider_sync = false;
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

    fn update_value_from_slider(
        &mut self,
        value: Hsla,
        emit: bool,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.needs_slider_sync = false;
        self.value = Some(value);
        self.hovered_color = Some(value);
        if emit {
            cx.emit(ColorPickerEvent::Change(Some(value)));
        }
        cx.notify();
    }

    fn sync_sliders(&mut self, color: Option<Hsla>, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(color) = color {
            self.hsla_sliders.update(color, window, cx);
        }
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
        self.state.update(cx, |state, cx| {
            if state.needs_slider_sync {
                let value = state.value;
                state.update_value(value, false, window, cx);
            }
        });

        let active_tab = self.state.read(cx).active_tab;

        let (slider_color, hovered_color) = {
            let state = self.state.read(cx);
            let slider_color = state
                .hovered_color
                .or(state.value)
                .unwrap_or_else(|| hsla(0., 0., 0., 1.));
            (slider_color, state.hovered_color)
        };

        v_flex()
            .p_0p5()
            .gap_3()
            .child(
                TabBar::new("mode")
                    .segmented()
                    .selected_index(active_tab)
                    .on_click(
                        window.listener_for(&self.state, |state, ix: &usize, _, cx| {
                            state.active_tab = *ix;
                            cx.notify();
                        }),
                    )
                    .child(Tab::new().flex_1().label(t!("ColorPicker.Palette")))
                    .child(Tab::new().flex_1().label(t!("ColorPicker.HSLA"))),
            )
            .child(match active_tab {
                0 => self.render_palette_panel(window, cx).into_any_element(),
                _ => self
                    .render_slider_tab_panel(slider_color, cx)
                    .into_any_element(),
            })
            .when_some(hovered_color, |this, hovered_color| {
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

    fn render_palette_panel(&self, window: &mut Window, cx: &mut App) -> impl IntoElement {
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
    }

    fn render_slider_tab_panel(&self, slider_color: Hsla, cx: &mut App) -> impl IntoElement {
        let hsla_sliders = self.state.read(cx).hsla_sliders.clone();
        let steps = 96usize;
        let hue_colors = (0..steps)
            .map(|ix| {
                let h = ix as f32 / (steps.saturating_sub(1)) as f32;
                hsla(h, 1.0, 0.5, 1.0)
            })
            .collect::<Vec<_>>();
        let saturation_start = hsla(slider_color.h, 0.0, slider_color.l, 1.0);
        let saturation_end = hsla(slider_color.h, 1.0, slider_color.l, 1.0);
        let lightness_colors = (0..steps)
            .map(|ix| {
                let l = ix as f32 / (steps.saturating_sub(1)) as f32;
                hsla(slider_color.h, 1.0, l, 1.0)
            })
            .collect::<Vec<_>>();
        let alpha_start = hsla(slider_color.h, slider_color.s, slider_color.l, 0.0);
        let alpha_end = hsla(slider_color.h, slider_color.s, slider_color.l, 1.0);

        let label_color = cx.theme().foreground.opacity(0.7);

        v_flex()
            .gap_2()
            .child(
                h_flex()
                    .gap_2()
                    .items_center()
                    .child(
                        div()
                            .min_w_16()
                            .text_xs()
                            .text_color(label_color)
                            .child(t!("ColorPicker.Hue")),
                    )
                    .child(
                        div()
                            .relative()
                            .flex()
                            .items_center()
                            .flex_1()
                            .h_8()
                            .child(self.render_slider_track(hue_colors, cx))
                            .child(
                                Slider::new(&hsla_sliders.hue)
                                    .flex_1()
                                    .bg(cx.theme().transparent),
                            ),
                    )
                    .child(
                        div()
                            .w_10()
                            .text_xs()
                            .text_color(label_color)
                            .text_align(TextAlign::Right)
                            .child(format!("{:.0}", slider_color.h * 360.)),
                    ),
            )
            .child(
                h_flex()
                    .gap_2()
                    .items_center()
                    .child(
                        div()
                            .min_w_16()
                            .text_xs()
                            .text_color(label_color)
                            .child(t!("ColorPicker.Saturation")),
                    )
                    .child(
                        div()
                            .relative()
                            .flex()
                            .items_center()
                            .flex_1()
                            .h_8()
                            .child(self.render_slider_track_gradient(
                                saturation_start,
                                saturation_end,
                                cx,
                            ))
                            .child(
                                Slider::new(&hsla_sliders.saturation)
                                    .flex_1()
                                    .bg(cx.theme().transparent),
                            ),
                    )
                    .child(
                        div()
                            .w_10()
                            .text_xs()
                            .text_color(label_color)
                            .text_align(TextAlign::Right)
                            .child(format!("{:.0}", slider_color.s * 100.)),
                    ),
            )
            .child(
                h_flex()
                    .gap_2()
                    .items_center()
                    .child(
                        div()
                            .min_w_16()
                            .text_xs()
                            .text_color(label_color)
                            .child(t!("ColorPicker.Lightness")),
                    )
                    .child(
                        div()
                            .relative()
                            .flex()
                            .items_center()
                            .flex_1()
                            .h_8()
                            .child(self.render_slider_track(lightness_colors, cx))
                            .child(
                                Slider::new(&hsla_sliders.lightness)
                                    .flex_1()
                                    .bg(cx.theme().transparent),
                            ),
                    )
                    .child(
                        div()
                            .w_10()
                            .text_xs()
                            .text_color(label_color)
                            .text_align(TextAlign::Right)
                            .child(format!("{:.0}", slider_color.l * 100.)),
                    ),
            )
            .child(
                h_flex()
                    .gap_2()
                    .items_center()
                    .child(
                        div()
                            .min_w_16()
                            .text_xs()
                            .text_color(label_color)
                            .child(t!("ColorPicker.Alpha")),
                    )
                    .child(
                        div()
                            .relative()
                            .flex()
                            .items_center()
                            .flex_1()
                            .h_8()
                            .child(self.render_slider_track_gradient(alpha_start, alpha_end, cx))
                            .child(
                                Slider::new(&hsla_sliders.alpha)
                                    .flex_1()
                                    .bg(cx.theme().transparent),
                            ),
                    )
                    .child(
                        div()
                            .w_10()
                            .text_xs()
                            .text_color(label_color)
                            .text_align(TextAlign::Right)
                            .child(format!("{:.0}", slider_color.a * 100.)),
                    ),
            )
    }

    fn render_slider_track(&self, colors: Vec<Hsla>, _: &App) -> impl IntoElement {
        h_flex()
            .absolute()
            .left_0()
            .right_0()
            .h_2_5()
            .overflow_hidden()
            .children(
                colors
                    .into_iter()
                    .map(|color| div().flex_1().h_full().bg(color)),
            )
    }

    fn render_slider_track_gradient(&self, start: Hsla, end: Hsla, _: &App) -> impl IntoElement {
        div()
            .absolute()
            .left_0()
            .right_0()
            .h_2_5()
            .overflow_hidden()
            .bg(linear_gradient(
                90.,
                linear_color_stop(start, 0.),
                linear_color_stop(end, 1.),
            ))
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
                    .trigger(ColorPickerButton {
                        id: "trigger".into(),
                        size: self.size,
                        label: self.label.clone(),
                        value: state.value,
                        tooltip: if display_title.is_empty() {
                            None
                        } else {
                            Some(display_title.clone())
                        },
                        icon: self.icon.clone(),
                        selected: false,
                    })
                    .child(self.render_colors(window, cx)),
            )
    }
}

#[derive(IntoElement)]
struct ColorPickerButton {
    id: ElementId,
    selected: bool,
    icon: Option<Icon>,
    value: Option<Hsla>,
    size: Size,
    label: Option<SharedString>,
    tooltip: Option<SharedString>,
}

impl Selectable for ColorPickerButton {
    fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    fn is_selected(&self) -> bool {
        self.selected
    }
}

impl Sizable for ColorPickerButton {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }
}

impl RenderOnce for ColorPickerButton {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let has_icon = self.icon.is_some();
        h_flex()
            .id(self.id)
            .gap_2()
            .children(self.icon)
            .when(!has_icon, |this| {
                this.child(
                    div()
                        .id("square")
                        .bg(cx.theme().background)
                        .border_1()
                        .border_color(cx.theme().input)
                        .when(cx.theme().shadow, |this| this.shadow_xs())
                        .rounded(cx.theme().radius)
                        .overflow_hidden()
                        .size_with(self.size)
                        .when_some(self.value, |this, value| {
                            this.bg(value)
                                .border_color(value.darken(0.3))
                                .when(self.selected, |this| this.border_2())
                        })
                        .when_some(self.tooltip, |this, tooltip| {
                            this.tooltip(move |_, cx| {
                                cx.new(|_| Tooltip::new(tooltip.clone())).into()
                            })
                        }),
                )
            })
            .when_some(self.label, |this, label| this.child(label))
    }
}
