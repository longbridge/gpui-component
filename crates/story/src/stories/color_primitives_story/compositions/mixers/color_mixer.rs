use std::marker::PhantomData;

use crate::stories::color_primitives_story::color_primitives::color_slider::color_spec::ColorSpecification;
use crate::stories::color_primitives_story::color_primitives::color_slider::delegates::{
    AlphaDelegate, ChannelDelegate,
};
use crate::stories::color_primitives_story::color_primitives::color_slider::{
    ColorInterpolation, ColorSliderEvent, ColorSliderState,
};
use gpui::{
    App, AppContext, Context, Empty, Entity, EventEmitter, FocusHandle, Focusable, Hsla,
    IntoElement, Render, SharedString, Subscription, Window, px,
};
use gpui_component::{Sizable, Size};

/// Events emitted by the [`ColorMixerState`].
#[derive(Clone)]
pub enum ColorMixerEvent {
    Change(Option<Hsla>),
}

/// Headless state of the Color picker controls.
pub struct ColorMixerState<S: ColorSpecification> {
    focus_handle: FocusHandle,
    spec: S,
    value: Option<Hsla>,
    sliders: Vec<(SharedString, Entity<ColorSliderState>)>,
    alpha_slider: Option<Entity<ColorSliderState>>,
    _subscriptions: Vec<Subscription>,
    _marker: PhantomData<S>,
}

impl<S: ColorSpecification> ColorMixerState<S> {
    /// Create a new [`ColorMixerState`].
    pub fn new(_window: &mut Window, cx: &mut Context<Self>) -> Self {
        let spec = S::from_hsla(gpui::red());

        let mut sliders = Vec::new();
        let mut _subscriptions = vec![];

        for channel in spec.channels() {
            if channel.name == "alpha" {
                continue;
            }

            let channel_name: SharedString = channel.name.into();
            let channel_value = spec.get_value(&channel_name);

            let interpolation = if spec.name() == "Lab" {
                ColorInterpolation::Lab
            } else if spec.name() == "HSL" || spec.name() == "HSV" || spec.name() == "Hue+Alpha" {
                ColorInterpolation::Hsl
            } else {
                ColorInterpolation::Rgb
            };

            // Hue has a special delegate for rendering the spectrum
            let slider = if channel.name == "hue" {
                let name = format!("{}_hue", spec.name());
                cx.new(|cx| {
                    ColorSliderState::hue(name, channel_value, cx)
                        .interpolation(interpolation)
                        .horizontal()
                        .with_size(Size::Small)
                        .thumb_medium()
                        .edge_to_edge()
                })
            } else {
                let name = format!("{}_{}", spec.name(), channel_name);
                cx.new(|cx| {
                    ColorSliderState::channel(
                        name,
                        channel_value,
                        ChannelDelegate::new(spec, channel_name.clone())
                            .expect("Failed to create ChannelDelegate in generic mixer"),
                        cx,
                    )
                    .min(channel.min)
                    .max(channel.max)
                    .interpolation(interpolation)
                    .horizontal()
                    .with_size(Size::Small)
                    .thumb_medium()
                    .edge_to_edge()
                })
            };

            slider.update(cx, |s, cx| {
                s.set_corner_radius(px(0.0).into(), cx);
            });

            let channel_name_clone = channel_name.clone();
            _subscriptions.push(cx.subscribe(&slider, move |this, _, event, cx| {
                let value = match event {
                    ColorSliderEvent::Change(value) | ColorSliderEvent::Release(value) => *value,
                };
                let clamped_value = this.spec.clamp_channel_to_gamut(&channel_name_clone, value);
                this.spec.set_value(&channel_name_clone, clamped_value);

                // If it was clamped away from the raw slider value, we must force the slider to the clamped value.
                if (clamped_value - value).abs() > f32::EPSILON {
                    if let Some((_, s)) = this
                        .sliders
                        .iter()
                        .find(|(name, _)| name == &channel_name_clone)
                    {
                        s.update(cx, |slider, cx| slider.set_value(clamped_value, cx));
                    }
                }

                // For Lab, changing one slider can affect the valid dynamic ranges of other sliders
                this.refresh_slider_metadata(cx);
                this.update_from_controls(true, cx);
            }));

            sliders.push((channel_name, slider));
        }

        let has_alpha = spec.channels().iter().any(|c| c.name == "alpha");
        let mut alpha_slider = None;
        if has_alpha {
            let spec_copy = spec;
            let name = format!("{}_alpha", spec.name());
            let slider = cx.new(|cx| {
                ColorSliderState::alpha(
                    name,
                    spec.get_value("alpha"),
                    AlphaDelegate { spec: spec_copy },
                    cx,
                )
                .interpolation(if spec.name() == "Lab" {
                    ColorInterpolation::Lab
                } else {
                    ColorInterpolation::Hsl
                })
                .horizontal()
                .with_size(Size::Small)
                .thumb_medium()
                .edge_to_edge()
            });
            slider.update(cx, |s, cx| {
                s.set_corner_radius(px(0.0).into(), cx);
            });
            _subscriptions.push(cx.subscribe(&slider, |this, _, event, cx| {
                let value = match event {
                    ColorSliderEvent::Change(value) | ColorSliderEvent::Release(value) => *value,
                };
                this.spec
                    .set_value("alpha", (value * 1000.0).round() / 1000.0);
                this.refresh_slider_metadata(cx);
                this.update_from_controls(true, cx);
            }));
            alpha_slider = Some(slider);
        }

        Self {
            focus_handle: cx.focus_handle(),
            spec,
            value: Some(spec.to_hsla()),
            sliders,
            alpha_slider,
            _subscriptions,
            _marker: PhantomData,
        }
    }

    /// Set default color value.
    pub fn default_value(mut self, value: impl Into<Hsla>, cx: &mut Context<Self>) -> Self {
        self.apply_hsla(value.into(), false, cx);
        self
    }

    // Allows Lab auto clamping. Reuses the generic hooks.
    pub fn auto_clamp(mut self, auto_clamp: bool) -> Self {
        self.spec.set_auto_clamp(auto_clamp);
        self.spec.clamp_spec_to_gamut();
        self
    }

    // Allows Lab dynamic ranges.
    pub fn dynamic_range(mut self, dynamic_range: bool) -> Self {
        self.spec.set_dynamic_range(dynamic_range);
        self.spec.clamp_spec_to_gamut();
        self
    }

    fn refresh_slider_metadata(&mut self, cx: &mut Context<Self>) {
        for (channel_name, slider) in &self.sliders {
            let (min, max) = self.spec.channel_bounds(channel_name);
            slider.update(cx, |s, cx| {
                s.set_range(min, max, cx);
                if channel_name.as_ref() != "hue" {
                    s.set_delegate(
                        Box::new(ChannelDelegate::new(self.spec, channel_name.clone()).unwrap()),
                        cx,
                    );
                }
            });
        }
        if let Some(alpha_slider) = &self.alpha_slider {
            alpha_slider.update(cx, |s, cx| {
                s.set_delegate(Box::new(AlphaDelegate { spec: self.spec }), cx);
            });
        }
    }

    fn update_from_controls(&mut self, emit: bool, cx: &mut Context<Self>) {
        let hsla = self.spec.to_hsla();
        self.value = Some(hsla);
        if emit {
            cx.emit(ColorMixerEvent::Change(self.value));
        }
        cx.notify();
    }

    pub fn value(&self) -> Option<Hsla> {
        self.value
    }

    pub fn spec(&self) -> &S {
        &self.spec
    }

    pub fn sliders(&self) -> &[(SharedString, Entity<ColorSliderState>)] {
        &self.sliders
    }

    pub fn alpha_slider(&self) -> Option<&Entity<ColorSliderState>> {
        self.alpha_slider.as_ref()
    }

    pub fn set_hsla(&mut self, value: Hsla, cx: &mut Context<Self>) {
        self.apply_hsla(value, true, cx);
    }

    fn apply_hsla(&mut self, value: Hsla, emit: bool, cx: &mut Context<Self>) {
        let source = S::from_hsla(value);
        let channel_names: Vec<SharedString> = self
            .spec
            .channels()
            .iter()
            .map(|channel| channel.name.into())
            .collect();
        for channel_name in channel_names {
            self.spec.set_value(&channel_name, source.get_value(&channel_name));
        }
        self.spec.clamp_spec_to_gamut();

        for (channel_name, slider) in &self.sliders {
            let channel_value = self.spec.get_value(channel_name);
            slider.update(cx, |s, cx| s.set_value(channel_value, cx));
        }
        if let Some(alpha_slider) = &self.alpha_slider {
            alpha_slider.update(cx, |slider, cx| {
                slider.set_value(self.spec.get_value("alpha"), cx);
            });
        }

        self.refresh_slider_metadata(cx);
        self.update_from_controls(emit, cx);
    }
}

impl<S: ColorSpecification> EventEmitter<ColorMixerEvent> for ColorMixerState<S> {}

impl<S: ColorSpecification> Render for ColorMixerState<S> {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        Empty
    }
}

impl<S: ColorSpecification> Focusable for ColorMixerState<S> {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}
