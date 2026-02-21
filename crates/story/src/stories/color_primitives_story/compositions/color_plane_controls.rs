use super::color_control_channels::ColorControlChannels;
use super::color_text_label::ColorTextLabel;
use crate::stories::color_primitives_story::color_primitives::color_field::{
    ColorFieldEvent, ColorFieldState,
};
use crate::stories::color_primitives_story::color_primitives::color_slider::color_spec::{
    ColorSpecification, Hsv,
};
use crate::stories::color_primitives_story::color_primitives::color_slider::delegates::{
    AlphaDelegate, ChannelDelegate,
};
use crate::stories::color_primitives_story::color_primitives::color_slider::sizing as slider_constants;
use crate::stories::color_primitives_story::color_primitives::color_slider::{
    ColorSliderEvent, ColorSliderState,
};
use gpui::{
    App, AppContext, Context, Empty, Entity, EventEmitter, FocusHandle, Focusable, Hsla,
    IntoElement, ParentElement as _, Render, RenderOnce, Styled as _, Subscription, Window, div,
    prelude::FluentBuilder as _, px,
};
use gpui_component::{
    ActiveTheme as _, Colorize as _, IconName, Sizable, Size,
    button::{Button, ButtonVariants},
    h_flex, v_flex,
};

const CONTROL_ROW_WIDTH_PX: f32 = 256.0;
const SELECTED_SWATCH_HEIGHT_PX: f32 = 40.0;

#[derive(Clone)]
#[allow(dead_code)]
pub enum ColorPlaneControlsEvent {
    Change(Option<Hsla>),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ColorControlMode {
    Immediate,
    Deferred,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(dead_code)]
pub enum PlaneFieldKind {
    SaturationValueAtHue,
    HueSaturationAtValue,
    HueValueAtSaturation,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(dead_code)]
pub enum PlaneAxisKind {
    Hue,
    Saturation,
    Value,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ColorPlaneControlsConfig {
    pub plane_kind: PlaneFieldKind,
    pub axis_kind: PlaneAxisKind,
    pub channels: ColorControlChannels,
}

impl ColorPlaneControlsConfig {
    pub fn saturation_value_at_hue() -> Self {
        Self {
            plane_kind: PlaneFieldKind::SaturationValueAtHue,
            axis_kind: PlaneAxisKind::Hue,
            channels: ColorControlChannels::hsv(),
        }
    }

    pub fn photoshop() -> Self {
        Self::saturation_value_at_hue()
    }
}

impl Default for ColorPlaneControlsConfig {
    fn default() -> Self {
        Self::photoshop()
    }
}

/// State for HSV plane/slider controls.
pub struct ColorPlaneControlsState {
    focus_handle: FocusHandle,
    hsv: Hsv,
    value: Option<Hsla>,
    pending_value: Option<Hsla>,
    mode: ColorControlMode,
    config: ColorPlaneControlsConfig,
    plane_field: Entity<ColorFieldState>,
    axis_slider: Entity<ColorSliderState>,
    alpha_slider: Entity<ColorSliderState>,
    _subscriptions: Vec<Subscription>,
}

impl ColorPlaneControlsState {
    // Construction

    /// Create default saturation/value-at-hue controls.
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        Self::new_with_config(window, cx, ColorPlaneControlsConfig::default())
    }

    pub fn new_with_config(
        _window: &mut Window,
        cx: &mut Context<Self>,
        config: ColorPlaneControlsConfig,
    ) -> Self {
        let hsv = Hsv {
            h: 0.0,
            s: 0.5,
            v: 0.5,
            a: 1.0,
        };

        let plane_field = cx.new(|_cx| {
            let plane = match config.plane_kind {
                PlaneFieldKind::SaturationValueAtHue => ColorFieldState::saturation_value(
                    "color_plane_controls_plane",
                    hsv,
                    slider_constants::THUMB_SIZE_MEDIUM,
                ),
                PlaneFieldKind::HueSaturationAtValue => ColorFieldState::hue_saturation(
                    "color_plane_controls_plane",
                    hsv,
                    slider_constants::THUMB_SIZE_MEDIUM,
                ),
                PlaneFieldKind::HueValueAtSaturation => ColorFieldState::hue_value(
                    "color_plane_controls_plane",
                    hsv,
                    slider_constants::THUMB_SIZE_MEDIUM,
                ),
            };

            plane.rounded(px(3.0))
        });

        let axis_slider = cx.new(|cx| {
            let slider = match config.axis_kind {
                PlaneAxisKind::Hue => {
                    ColorSliderState::hue("color_plane_controls_axis", hsv.h, cx).horizontal()
                }
                PlaneAxisKind::Saturation => ColorSliderState::channel(
                    "color_plane_controls_axis",
                    hsv.s,
                    ChannelDelegate::new(hsv, Hsv::SATURATION.into())
                        .expect("Failed to create Saturation ChannelDelegate"),
                    cx,
                )
                .horizontal(),
                PlaneAxisKind::Value => ColorSliderState::channel(
                    "color_plane_controls_axis",
                    hsv.v,
                    ChannelDelegate::new(hsv, Hsv::VALUE.into())
                        .expect("Failed to create Value ChannelDelegate"),
                    cx,
                )
                .horizontal(),
            };

            slider.with_size(Size::Small).thumb_medium().edge_to_edge()
        });

        let alpha_slider = cx.new(|cx| {
            ColorSliderState::alpha(
                "color_plane_controls_alpha",
                hsv.a,
                AlphaDelegate { spec: hsv },
                cx,
            )
            .horizontal()
            .with_size(Size::Small)
            .thumb_medium()
            .edge_to_edge()
        });

        axis_slider.update(cx, |s, cx| {
            s.set_corner_radius(px(4.0).into(), cx);
        });
        alpha_slider.update(cx, |s, cx| {
            s.set_corner_radius(px(4.0).into(), cx);
        });

        let mut _subscriptions = vec![];

        _subscriptions.push(cx.subscribe(&plane_field, |this, _, event, cx| {
            let (hue, saturation, value) = match event {
                ColorFieldEvent::Change(hsv) | ColorFieldEvent::Release(hsv) => {
                    (hsv.h, hsv.s, hsv.v)
                }
            };
            this.hsv.h = hue;
            this.hsv.s = saturation;
            this.hsv.v = value;
            this.sync_axis_slider_from_hsv(cx);
            this.update_alpha_delegate(cx);
            this.update_from_controls(true, cx);
        }));

        _subscriptions.push(cx.subscribe(&axis_slider, |this, _, event, cx| {
            let axis_value = match event {
                ColorSliderEvent::Change(axis_value) | ColorSliderEvent::Release(axis_value) => {
                    *axis_value
                }
            };
            this.apply_axis_value(axis_value);
            let hsv = this.hsv;
            this.plane_field.update(cx, |s, cx| {
                s.set_hsv_components(hsv.h, hsv.s, hsv.v, cx);
            });
            this.update_alpha_delegate(cx);
            this.update_from_controls(true, cx);
        }));

        _subscriptions.push(cx.subscribe(&alpha_slider, |this, _, event, cx| {
            let alpha = match event {
                ColorSliderEvent::Change(alpha) | ColorSliderEvent::Release(alpha) => *alpha,
            };
            this.hsv.a = (alpha * 1000.0).round() / 1000.0;
            this.update_from_controls(true, cx);
        }));

        let mut state = Self {
            focus_handle: cx.focus_handle(),
            hsv,
            value: Some(hsv.to_hsla()),
            pending_value: None,
            mode: ColorControlMode::Immediate,
            config,
            plane_field,
            axis_slider,
            alpha_slider,
            _subscriptions,
        };

        state.sync_axis_slider_from_hsv(cx);

        state
    }

    /// Set default color value.
    pub fn default_value(mut self, value: impl Into<Hsla>, cx: &mut Context<Self>) -> Self {
        self.update_value(Some(value.into()), false, cx);
        self
    }

    /// Set the behavior mode for the controls.
    pub fn mode(mut self, mode: ColorControlMode) -> Self {
        self.mode = mode;
        self
    }

    // Public actions

    pub fn commit_pending(&mut self, cx: &mut Context<Self>) {
        let value = self
            .pending_value
            .take()
            .unwrap_or_else(|| self.hsv.to_hsla());
        self.value = Some(value);
        cx.emit(ColorPlaneControlsEvent::Change(self.value));
        cx.notify();
    }

    pub fn cancel_pending(&mut self, cx: &mut Context<Self>) {
        self.sync_controls_from_value(cx);
        cx.notify();
    }

    // Internal sync + state transitions

    fn axis_value(&self) -> f32 {
        match self.config.axis_kind {
            PlaneAxisKind::Hue => self.hsv.h,
            PlaneAxisKind::Saturation => self.hsv.s,
            PlaneAxisKind::Value => self.hsv.v,
        }
    }

    fn apply_axis_value(&mut self, value: f32) {
        match self.config.axis_kind {
            PlaneAxisKind::Hue => self.hsv.h = value,
            PlaneAxisKind::Saturation => self.hsv.s = value,
            PlaneAxisKind::Value => self.hsv.v = value,
        }
    }

    fn sync_axis_slider_from_hsv(&mut self, cx: &mut Context<Self>) {
        let hsv = self.hsv;
        let value = self.axis_value();

        self.axis_slider.update(cx, |slider, cx| {
            slider.set_value(value, cx);

            match self.config.axis_kind {
                PlaneAxisKind::Hue => {}
                PlaneAxisKind::Saturation => {
                    slider.set_delegate(
                        Box::new(
                            ChannelDelegate::new(hsv, Hsv::SATURATION.into())
                                .expect("Failed to create Saturation ChannelDelegate"),
                        ),
                        cx,
                    );
                }
                PlaneAxisKind::Value => {
                    slider.set_delegate(
                        Box::new(
                            ChannelDelegate::new(hsv, Hsv::VALUE.into())
                                .expect("Failed to create Value ChannelDelegate"),
                        ),
                        cx,
                    );
                }
            }
        });
    }

    fn update_from_controls(&mut self, emit: bool, cx: &mut Context<Self>) {
        let hsla = self.hsv.to_hsla();
        match self.mode {
            ColorControlMode::Immediate => {
                self.value = Some(hsla);
                self.pending_value = Some(hsla);
                if emit {
                    cx.emit(ColorPlaneControlsEvent::Change(self.value));
                }
            }
            ColorControlMode::Deferred => {
                self.pending_value = Some(hsla);
            }
        }
        cx.notify();
    }

    fn update_value(&mut self, value: Option<Hsla>, emit: bool, cx: &mut Context<Self>) {
        if let Some(value) = value {
            let hsv = Hsv::from_hsla_ext(value);
            self.hsv = hsv;
            self.value = Some(value);
            self.pending_value = Some(value);
            self.sync_axis_slider_from_hsv(cx);
            self.plane_field.update(cx, |plane, cx| {
                plane.set_hsv_components(hsv.h, hsv.s, hsv.v, cx);
            });
            self.alpha_slider.update(cx, |slider, cx| {
                slider.set_value(hsv.a, cx);
                slider.set_delegate(Box::new(AlphaDelegate { spec: hsv }), cx);
            });
        } else {
            self.value = None;
            self.pending_value = None;
        }
        if emit {
            cx.emit(ColorPlaneControlsEvent::Change(self.value));
        }
        cx.notify();
    }

    fn sync_controls_from_value(&mut self, cx: &mut Context<Self>) {
        if let Some(value) = self.value {
            let hsv = Hsv::from_hsla_ext(value);
            self.hsv = hsv;
            self.pending_value = Some(value);
            self.sync_axis_slider_from_hsv(cx);
            self.plane_field.update(cx, |plane, cx| {
                plane.set_hsv_components(hsv.h, hsv.s, hsv.v, cx);
            });
            self.alpha_slider.update(cx, |slider, cx| {
                slider.set_value(hsv.a, cx);
                slider.set_delegate(Box::new(AlphaDelegate { spec: hsv }), cx);
            });
        } else {
            self.pending_value = None;
        }
    }

    fn update_alpha_delegate(&mut self, cx: &mut Context<Self>) {
        let hsv = self.hsv;
        self.alpha_slider.update(cx, |slider, cx| {
            slider.set_delegate(Box::new(AlphaDelegate { spec: hsv }), cx);
        });
    }

}

impl EventEmitter<ColorPlaneControlsEvent> for ColorPlaneControlsState {}

impl Render for ColorPlaneControlsState {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        Empty
    }
}

impl Focusable for ColorPlaneControlsState {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

/// Plane-field controls without popup chrome.
#[derive(IntoElement)]
pub struct ColorPlaneControls {
    state: Entity<ColorPlaneControlsState>,
    show_actions: Option<bool>,
    channels: Option<ColorControlChannels>,
}

impl ColorPlaneControls {
    pub fn new(state: &Entity<ColorPlaneControlsState>) -> Self {
        Self {
            state: state.clone(),
            show_actions: None,
            channels: None,
        }
    }

    pub fn show_actions(mut self, show: bool) -> Self {
        self.show_actions = Some(show);
        self
    }

    pub fn channels(mut self, channels: ColorControlChannels) -> Self {
        self.channels = Some(channels);
        self
    }
}

impl RenderOnce for ColorPlaneControls {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let (hsv, plane_field, axis_slider, alpha_slider, mode, config) = {
            let state = self.state.read(cx);
            (
                state.hsv,
                state.plane_field.clone(),
                state.axis_slider.clone(),
                state.alpha_slider.clone(),
                state.mode,
                state.config,
            )
        };
        let hsla = hsv.to_hsla();
        let show_actions = self
            .show_actions
            .unwrap_or(mode == ColorControlMode::Deferred);
        let channels = self.channels.unwrap_or(config.channels);
        let show_plane = match config.plane_kind {
            PlaneFieldKind::SaturationValueAtHue => channels.saturation && channels.value,
            PlaneFieldKind::HueSaturationAtValue => channels.hue && channels.saturation,
            PlaneFieldKind::HueValueAtSaturation => channels.hue && channels.value,
        };
        let show_axis = match config.axis_kind {
            PlaneAxisKind::Hue => channels.hue,
            PlaneAxisKind::Saturation => channels.saturation,
            PlaneAxisKind::Value => channels.value,
        };
        let show_alpha = channels.alpha;

        v_flex()
            .gap_3()
            .items_start()
            .px_2()
            .when(show_plane, |this| {
                this.child(
                    h_flex()
                        .w_full()
                        .justify_center()
                        .child(div().pt_2().child(div().size(px(CONTROL_ROW_WIDTH_PX)).child(plane_field))),
                )
            })
            .when(show_axis, |this| {
                this.child(
                    h_flex()
                        .w_full()
                        .justify_center()
                        .child(div().w(px(CONTROL_ROW_WIDTH_PX)).child(axis_slider)),
                )
            })
            .when(show_alpha, |this| {
                this.child(
                    h_flex()
                        .w_full()
                        .justify_center()
                        .child(div().w(px(CONTROL_ROW_WIDTH_PX)).child(alpha_slider)),
                )
            })
            .child(
                h_flex()
                    .w_full()
                    .gap_3()
                    .items_center()
                    .justify_between()
                    .child(
                        v_flex()
                            .w(px(CONTROL_ROW_WIDTH_PX))
                            .items_center()
                            .gap_1()
                            .child(
                                div()
                                    .w(px(CONTROL_ROW_WIDTH_PX))
                                    .h(px(SELECTED_SWATCH_HEIGHT_PX))
                                    .bg(hsla)
                                    .border_1()
                                    .border_color(cx.theme().border)
                                    .rounded_md(),
                            )
                            .child(
                                ColorTextLabel::new(
                                    "color-plane-controls-copy",
                                    hsla.to_hex().to_uppercase().into(),
                                )
                                .width_px(CONTROL_ROW_WIDTH_PX),
                            ),
                    )
                    .when(show_actions, |this| {
                        this.child(
                            h_flex()
                                .gap_2()
                                .items_center()
                                .child(
                                    Button::new("color-plane-controls-cancel")
                                        .outline()
                                        .icon(IconName::Close)
                                        .on_click(window.listener_for(
                                            &self.state,
                                            |this, _, _, cx| {
                                                this.cancel_pending(cx);
                                            },
                                        )),
                                )
                                .child(
                                    Button::new("color-plane-controls-set")
                                        .primary()
                                        .icon(IconName::Check)
                                        .on_click(window.listener_for(
                                            &self.state,
                                            |this, _, _, cx| {
                                                this.commit_pending(cx);
                                            },
                                        )),
                                ),
                        )
                    }),
            )
    }
}
