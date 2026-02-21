use crate::stories::color_primitives_story::compositions::color_hex::{
    format_rgb_hex, parse_rgb_hex,
};
use gpui::{
    AnyElement, App, AppContext, Context, Entity, EventEmitter, FocusHandle, Focusable, Hsla,
    IntoElement, ParentElement, Render, SharedString, Styled, Subscription, Window, div,
    prelude::FluentBuilder as _, px,
};
use gpui_component::{
    ActiveTheme as _, Colorize as _, Icon, IconName, Sizable,
    clipboard::Clipboard,
    h_flex,
    input::{Input, InputEvent, InputState},
    select::{Select, SelectEvent, SelectState},
    v_flex,
};

use crate::stories::color_primitives_story::{
    color_readout::PublishedColorSpec,
    color_spec::{ColorSpecification, Hsl, Hsv, HueAlpha, Lab, RgbaSpec},
    compositions::mixers::color_mixer::{ColorMixerEvent, ColorMixerState},
};

const MULTI_MIXER_STYLE_TEMPLATE: MultiMixerStyleTemplate = MultiMixerStyleTemplate {
    text_size_px: 10.0,
    select_text_size_px: 13.0,
    text_font_family: MultiMixerFontFamily::Mono,
    top_section_vertical_padding_px: 2.0,
    slider_vertical_gap_px: 6.0,
    panel_width_px: 640.0,
    select_width_px: 200.0,
    channel_label_width_px: 16.0,
    value_box_width_px: 80.0,
    swatch_width_px: 96.0,
    swatch_height_px: 32.0,
    hex_box_width_px: 96.0,
};

const MIXER_OPTIONS: [&str; 7] = [
    "Hue + Alpha",
    "RGB",
    "HSLA",
    "HSVA",
    "Lab",
    "Lab (Auto-clamped)",
    "Lab (Dynamic Range)",
];

#[derive(Clone, Copy)]
struct MultiMixerStyleTemplate {
    text_size_px: f32,
    select_text_size_px: f32,
    text_font_family: MultiMixerFontFamily,
    top_section_vertical_padding_px: f32,
    slider_vertical_gap_px: f32,
    panel_width_px: f32,
    select_width_px: f32,
    channel_label_width_px: f32,
    value_box_width_px: f32,
    swatch_width_px: f32,
    swatch_height_px: f32,
    hex_box_width_px: f32,
}

#[derive(Clone, Copy)]
enum MultiMixerFontFamily {
    Mono,
}

impl MultiMixerFontFamily {
    fn resolve(self, cx: &App) -> gpui::SharedString {
        match self {
            Self::Mono => cx.theme().mono_font_family.clone(),
        }
    }
}

#[derive(Clone, Debug)]
pub enum MultiMixerEvent {
    Change {
        color: Option<Hsla>,
        published_spec: Option<PublishedColorSpec>,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MixerKind {
    HueAlpha,
    Rgb,
    Hsla,
    Hsva,
    Lab,
    LabAutoClamped,
    LabDynamicRange,
}

impl MixerKind {
    fn from_label(label: Option<&&'static str>) -> Self {
        match label {
            Some(&label) if label == MIXER_OPTIONS[1] => Self::Rgb,
            Some(&label) if label == MIXER_OPTIONS[2] => Self::Hsla,
            Some(&label) if label == MIXER_OPTIONS[3] => Self::Hsva,
            Some(&label) if label == MIXER_OPTIONS[4] => Self::Lab,
            Some(&label) if label == MIXER_OPTIONS[5] => Self::LabAutoClamped,
            Some(&label) if label == MIXER_OPTIONS[6] => Self::LabDynamicRange,
            _ => Self::HueAlpha,
        }
    }
}

enum ActiveMixerStateEntity {
    HueAlpha(Entity<ColorMixerState<HueAlpha>>),
    Rgb(Entity<ColorMixerState<RgbaSpec>>),
    Hsla(Entity<ColorMixerState<Hsl>>),
    Hsva(Entity<ColorMixerState<Hsv>>),
    Lab(Entity<ColorMixerState<Lab>>),
}

struct ActiveMixerView {
    dynamic_mixer_selection: Entity<SelectState<Vec<&'static str>>>,
    selected_color_input: Entity<InputState>,
    selected_color_input_error: Option<SharedString>,
    active_mixer: ActiveMixerStateEntity,
}

impl ActiveMixerView {
    fn from_state(state: &MultiMixerState, cx: &App) -> Self {
        let active_mixer = match state.selected_mixer_kind(cx) {
            MixerKind::HueAlpha => {
                ActiveMixerStateEntity::HueAlpha(state.dynamic_hue_alpha_mixer.clone())
            }
            MixerKind::Rgb => ActiveMixerStateEntity::Rgb(state.dynamic_rgb_mixer.clone()),
            MixerKind::Hsla => ActiveMixerStateEntity::Hsla(state.dynamic_hsla_mixer.clone()),
            MixerKind::Hsva => ActiveMixerStateEntity::Hsva(state.dynamic_hsva_mixer.clone()),
            MixerKind::Lab => ActiveMixerStateEntity::Lab(state.dynamic_lab_mixer.clone()),
            MixerKind::LabAutoClamped => {
                ActiveMixerStateEntity::Lab(state.dynamic_lab_auto_clamped_mixer.clone())
            }
            MixerKind::LabDynamicRange => {
                ActiveMixerStateEntity::Lab(state.dynamic_lab_dynamic_range_mixer.clone())
            }
        };

        Self {
            dynamic_mixer_selection: state.dynamic_mixer_selection.clone(),
            selected_color_input: state.selected_color_input.clone(),
            selected_color_input_error: state.selected_color_input_error.clone(),
            active_mixer,
        }
    }

    fn render(self, cx: &mut App) -> AnyElement {
        let select_menu = Select::new(&self.dynamic_mixer_selection)
            .with_size(px(MULTI_MIXER_STYLE_TEMPLATE.select_text_size_px))
            .w(px(MULTI_MIXER_STYLE_TEMPLATE.select_width_px))
            .into_any_element();

        match self.active_mixer {
            ActiveMixerStateEntity::HueAlpha(mixer) => render_dynamic_mixer_body(
                &mixer,
                self.selected_color_input.clone(),
                self.selected_color_input_error.clone(),
                select_menu,
                cx,
            )
            .into_any_element(),
            ActiveMixerStateEntity::Rgb(mixer) => render_dynamic_mixer_body(
                &mixer,
                self.selected_color_input.clone(),
                self.selected_color_input_error.clone(),
                select_menu,
                cx,
            )
            .into_any_element(),
            ActiveMixerStateEntity::Hsla(mixer) => render_dynamic_mixer_body(
                &mixer,
                self.selected_color_input.clone(),
                self.selected_color_input_error.clone(),
                select_menu,
                cx,
            )
            .into_any_element(),
            ActiveMixerStateEntity::Hsva(mixer) => render_dynamic_mixer_body(
                &mixer,
                self.selected_color_input.clone(),
                self.selected_color_input_error.clone(),
                select_menu,
                cx,
            )
            .into_any_element(),
            ActiveMixerStateEntity::Lab(mixer) => render_dynamic_mixer_body(
                &mixer,
                self.selected_color_input,
                self.selected_color_input_error,
                select_menu,
                cx,
            )
            .into_any_element(),
        }
    }
}

pub struct MultiMixerState {
    focus_handle: FocusHandle,
    dynamic_mixer_selection: Entity<SelectState<Vec<&'static str>>>,
    selected_color_input: Entity<InputState>,
    selected_color_input_programmatic_update: bool,
    selected_color_input_error: Option<SharedString>,
    dynamic_hue_alpha_mixer: Entity<ColorMixerState<HueAlpha>>,
    dynamic_rgb_mixer: Entity<ColorMixerState<RgbaSpec>>,
    dynamic_hsla_mixer: Entity<ColorMixerState<Hsl>>,
    dynamic_hsva_mixer: Entity<ColorMixerState<Hsv>>,
    dynamic_lab_mixer: Entity<ColorMixerState<Lab>>,
    dynamic_lab_auto_clamped_mixer: Entity<ColorMixerState<Lab>>,
    dynamic_lab_dynamic_range_mixer: Entity<ColorMixerState<Lab>>,
    _subscriptions: Vec<Subscription>,
}

impl MultiMixerState {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let initial_color: Hsla = gpui::Rgba {
            r: 0.0,
            g: 216.0 / 255.0,
            b: 1.0,
            a: 1.0,
        }
        .into();

        let dynamic_mixer_selection = cx.new(|cx| {
            SelectState::new(
                MIXER_OPTIONS.to_vec(),
                Some(gpui_component::IndexPath::default().row(2)),
                window,
                cx,
            )
        });
        let selected_color_input = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("#RRGGBB")
                .pattern(regex::Regex::new(r"^#[0-9a-fA-F]{0,6}$").unwrap())
                .default_value(format_rgb_hex(initial_color))
        });

        let dynamic_hue_alpha_mixer = cx.new(|cx| {
            ColorMixerState::<HueAlpha>::new(window, cx).default_value(initial_color, cx)
        });
        let dynamic_rgb_mixer = cx.new(|cx| {
            ColorMixerState::<RgbaSpec>::new(window, cx).default_value(initial_color, cx)
        });
        let dynamic_hsla_mixer =
            cx.new(|cx| ColorMixerState::<Hsl>::new(window, cx).default_value(initial_color, cx));
        let dynamic_hsva_mixer =
            cx.new(|cx| ColorMixerState::<Hsv>::new(window, cx).default_value(initial_color, cx));
        let dynamic_lab_mixer =
            cx.new(|cx| ColorMixerState::<Lab>::new(window, cx).default_value(initial_color, cx));
        let dynamic_lab_auto_clamped_mixer = cx.new(|cx| {
            ColorMixerState::<Lab>::new(window, cx)
                .default_value(initial_color, cx)
                .auto_clamp(true)
        });
        let dynamic_lab_dynamic_range_mixer = cx.new(|cx| {
            ColorMixerState::<Lab>::new(window, cx)
                .default_value(initial_color, cx)
                .dynamic_range(true)
        });

        let mut _subscriptions = vec![];

        _subscriptions.push(cx.subscribe_in(
            &dynamic_mixer_selection,
            window,
            |this, _, _: &SelectEvent<Vec<&'static str>>, window, cx| {
                this.sync_selected_hex_input(window, cx);
                this.emit_active_change(cx);
                cx.notify();
            },
        ));
        _subscriptions.push(cx.subscribe_in(
            &selected_color_input,
            window,
            |this, _, event: &InputEvent, window, cx| {
                this.on_selected_color_input_event(event, window, cx);
            },
        ));

        let mut state = Self {
            focus_handle: cx.focus_handle(),
            dynamic_mixer_selection,
            selected_color_input,
            selected_color_input_programmatic_update: false,
            selected_color_input_error: None,
            dynamic_hue_alpha_mixer,
            dynamic_rgb_mixer,
            dynamic_hsla_mixer,
            dynamic_hsva_mixer,
            dynamic_lab_mixer,
            dynamic_lab_auto_clamped_mixer,
            dynamic_lab_dynamic_range_mixer,
            _subscriptions,
        };

        state.subscribe_to_mixers(window, cx);
        state.sync_selected_hex_input(window, cx);
        state
    }

    fn emit_active_change(&self, cx: &mut Context<Self>) {
        let color = self.active_value(cx);
        let published_spec = self.active_published_spec(cx);
        cx.emit(MultiMixerEvent::Change {
            color,
            published_spec,
        });
    }

    fn selected_mixer_kind(&self, cx: &App) -> MixerKind {
        MixerKind::from_label(self.dynamic_mixer_selection.read(cx).selected_value())
    }

    pub fn active_value(&self, cx: &App) -> Option<Hsla> {
        match self.selected_mixer_kind(cx) {
            MixerKind::HueAlpha => self.dynamic_hue_alpha_mixer.read(cx).value(),
            MixerKind::Rgb => self.dynamic_rgb_mixer.read(cx).value(),
            MixerKind::Hsla => self.dynamic_hsla_mixer.read(cx).value(),
            MixerKind::Hsva => self.dynamic_hsva_mixer.read(cx).value(),
            MixerKind::Lab => self.dynamic_lab_mixer.read(cx).value(),
            MixerKind::LabAutoClamped => self.dynamic_lab_auto_clamped_mixer.read(cx).value(),
            MixerKind::LabDynamicRange => self.dynamic_lab_dynamic_range_mixer.read(cx).value(),
        }
    }

    pub fn active_published_spec(&self, cx: &App) -> Option<PublishedColorSpec> {
        match self.selected_mixer_kind(cx) {
            MixerKind::HueAlpha | MixerKind::Rgb | MixerKind::Hsla | MixerKind::Hsva => {
                Some(PublishedColorSpec::Rgb)
            }
            MixerKind::Lab => Some(PublishedColorSpec::Lab(
                *self.dynamic_lab_mixer.read(cx).spec(),
            )),
            MixerKind::LabAutoClamped => Some(PublishedColorSpec::Lab(
                *self.dynamic_lab_auto_clamped_mixer.read(cx).spec(),
            )),
            MixerKind::LabDynamicRange => Some(PublishedColorSpec::Lab(
                *self.dynamic_lab_dynamic_range_mixer.read(cx).spec(),
            )),
        }
    }

    fn subscribe_to_mixers(&self, window: &mut Window, cx: &mut Context<Self>) {
        self.subscribe_rgb_mixer(&self.dynamic_hue_alpha_mixer, window, cx);
        self.subscribe_rgb_mixer(&self.dynamic_rgb_mixer, window, cx);
        self.subscribe_rgb_mixer(&self.dynamic_hsla_mixer, window, cx);
        self.subscribe_rgb_mixer(&self.dynamic_hsva_mixer, window, cx);
        self.subscribe_lab_mixer(&self.dynamic_lab_mixer, window, cx, |this, cx| {
            PublishedColorSpec::Lab(*this.dynamic_lab_mixer.read(cx).spec())
        });
        self.subscribe_lab_mixer(
            &self.dynamic_lab_auto_clamped_mixer,
            window,
            cx,
            |this, cx| {
                PublishedColorSpec::Lab(*this.dynamic_lab_auto_clamped_mixer.read(cx).spec())
            },
        );
        self.subscribe_lab_mixer(
            &self.dynamic_lab_dynamic_range_mixer,
            window,
            cx,
            |this, cx| {
                PublishedColorSpec::Lab(*this.dynamic_lab_dynamic_range_mixer.read(cx).spec())
            },
        );
    }

    fn subscribe_rgb_mixer<S: ColorSpecification>(
        &self,
        mixer: &Entity<ColorMixerState<S>>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        cx.subscribe_in(mixer, window, |this, _, ev, window, cx| {
            let ColorMixerEvent::Change(color) = ev;
            this.sync_selected_hex_input(window, cx);
            cx.emit(MultiMixerEvent::Change {
                color: *color,
                published_spec: Some(PublishedColorSpec::Rgb),
            });
        })
        .detach();
    }

    fn subscribe_lab_mixer(
        &self,
        mixer: &Entity<ColorMixerState<Lab>>,
        window: &mut Window,
        cx: &mut Context<Self>,
        published_spec: impl Fn(&Self, &mut Context<Self>) -> PublishedColorSpec + 'static,
    ) {
        cx.subscribe_in(mixer, window, move |this, _, ev, window, cx| {
            let ColorMixerEvent::Change(color) = ev;
            this.sync_selected_hex_input(window, cx);
            let published_spec = published_spec(this, cx);
            cx.emit(MultiMixerEvent::Change {
                color: *color,
                published_spec: Some(published_spec),
            });
        })
        .detach();
    }

    fn on_selected_color_input_event(
        &mut self,
        event: &InputEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.selected_color_input_programmatic_update {
            return;
        }

        let input_text = self.selected_color_input.read(cx).value().to_string();
        let current_hex = self
            .active_value(cx)
            .map(format_rgb_hex)
            .unwrap_or_else(|| "".into());
        if !current_hex.is_empty() && input_text.trim().eq_ignore_ascii_case(current_hex.as_ref()) {
            self.selected_color_input_error = None;
            return;
        }

        let strict_validation = matches!(event, InputEvent::PressEnter { .. } | InputEvent::Blur);
        match parse_rgb_hex(&input_text) {
            Ok(parsed) => {
                self.selected_color_input_error = None;
                self.set_active_value(parsed, cx);
                self.sync_selected_hex_input(window, cx);
                cx.notify();
            }
            Err(message) => {
                if strict_validation {
                    self.selected_color_input_error = Some(message.into());
                } else {
                    self.selected_color_input_error = None;
                }
                cx.notify();
            }
        }
    }

    fn set_active_value(&mut self, value: Hsla, cx: &mut Context<Self>) {
        match self.selected_mixer_kind(cx) {
            MixerKind::HueAlpha => self
                .dynamic_hue_alpha_mixer
                .update(cx, |mixer, cx| mixer.set_hsla(value, cx)),
            MixerKind::Rgb => self
                .dynamic_rgb_mixer
                .update(cx, |mixer, cx| mixer.set_hsla(value, cx)),
            MixerKind::Hsla => self
                .dynamic_hsla_mixer
                .update(cx, |mixer, cx| mixer.set_hsla(value, cx)),
            MixerKind::Hsva => self
                .dynamic_hsva_mixer
                .update(cx, |mixer, cx| mixer.set_hsla(value, cx)),
            MixerKind::Lab => self
                .dynamic_lab_mixer
                .update(cx, |mixer, cx| mixer.set_hsla(value, cx)),
            MixerKind::LabAutoClamped => self
                .dynamic_lab_auto_clamped_mixer
                .update(cx, |mixer, cx| mixer.set_hsla(value, cx)),
            MixerKind::LabDynamicRange => self
                .dynamic_lab_dynamic_range_mixer
                .update(cx, |mixer, cx| mixer.set_hsla(value, cx)),
        }
    }

    fn sync_selected_hex_input(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(active) = self.active_value(cx) else {
            return;
        };
        let value = format_rgb_hex(active);
        let current = self.selected_color_input.read(cx).value();
        if current == value {
            return;
        }

        self.selected_color_input_programmatic_update = true;
        self.selected_color_input.update(cx, |input, cx| {
            input.set_value(value, window, cx);
        });
        self.selected_color_input_programmatic_update = false;
    }
}

impl EventEmitter<MultiMixerEvent> for MultiMixerState {}

impl Focusable for MultiMixerState {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for MultiMixerState {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        MultiMixer::new(&cx.entity())
    }
}

#[derive(IntoElement)]
pub struct MultiMixer {
    state: Entity<MultiMixerState>,
}

impl MultiMixer {
    pub fn new(state: &Entity<MultiMixerState>) -> Self {
        Self {
            state: state.clone(),
        }
    }
}

impl gpui::RenderOnce for MultiMixer {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let state = self.state.read(cx);
        ActiveMixerView::from_state(&state, cx).render(cx)
    }
}

fn render_dynamic_mixer_body<S: ColorSpecification>(
    state_entity: &Entity<ColorMixerState<S>>,
    selected_color_input: Entity<InputState>,
    selected_color_input_error: Option<SharedString>,
    select_menu: AnyElement,
    cx: &mut App,
) -> impl IntoElement {
    let style = MULTI_MIXER_STYLE_TEMPLATE;
    let state = state_entity.read(cx);
    let spec = state.spec();
    let hsla = state.value().unwrap_or(gpui::red());
    let hex = hsla.to_hex().to_uppercase();
    let out_of_gamut = spec.is_out_of_gamut();
    let text_font_family = style.text_font_family.resolve(cx);
    let text_size = px(style.text_size_px);

    let mut sliders_flex = v_flex().gap(px(style.slider_vertical_gap_px));
    for (channel_name, slider) in state.sliders() {
        let formatted_value = spec.format_value(&channel_name);

        let display_char = match channel_name.as_ref() {
            "red" => "R",
            "green" => "G",
            "blue" => "B",
            "hue" => "H",
            "saturation" => "S",
            "lightness" if spec.name() == "Lab" => "L*",
            "lightness" => "L",
            "value" => "V",
            "a" => "a*",
            "b" => "b*",
            _ => channel_name.as_ref(),
        };

        sliders_flex = sliders_flex.child(
            h_flex()
                .gap_4()
                .items_center()
                .child(
                    div()
                        .w(px(style.channel_label_width_px))
                        .font_family(text_font_family.clone())
                        .text_size(text_size)
                        .child(display_char.to_string()),
                )
                .child(div().flex_1().child(slider.clone()))
                .child(
                    div()
                        .w(px(style.value_box_width_px))
                        .border_1()
                        .border_color(cx.theme().border)
                        .px_2()
                        .py_1()
                        .rounded_sm()
                        .font_family(text_font_family.clone())
                        .text_size(text_size)
                        .child(formatted_value),
                ),
        );
    }

    if let Some(alpha_slider) = state.alpha_slider() {
        let formatted_value = spec.format_value("alpha");
        sliders_flex = sliders_flex.child(
            h_flex()
                .gap_4()
                .items_center()
                .child(
                    div()
                        .w(px(style.channel_label_width_px))
                        .font_family(text_font_family.clone())
                        .text_size(text_size)
                        .child("A"),
                )
                .child(div().flex_1().child(alpha_slider.clone()))
                .child(
                    div()
                        .w(px(style.value_box_width_px))
                        .border_1()
                        .border_color(cx.theme().border)
                        .px_2()
                        .py_1()
                        .rounded_sm()
                        .font_family(text_font_family.clone())
                        .text_size(text_size)
                        .child(formatted_value),
                ),
        );
    }

    div()
        .w(px(style.panel_width_px))
        .bg(cx.theme().background)
        .border_1()
        .border_color(cx.theme().border)
        .rounded_none()
        .child(
            v_flex()
                .child(
                    div()
                        .px_4()
                        .pt(px(style.top_section_vertical_padding_px))
                        .pb(px(style.top_section_vertical_padding_px))
                        .child(
                            h_flex()
                                .justify_between()
                                .items_center()
                                .child(
                                    h_flex()
                                        .gap_2()
                                        .items_center()
                                        .child(
                                            Icon::new(IconName::TriangleAlert)
                                                .text_color(cx.theme().danger)
                                                .when(!out_of_gamut, |this: Icon| this.invisible()),
                                        )
                                        .child(select_menu),
                                )
                                .child(
                                    h_flex()
                                        .items_center()
                                        .gap_2()
                                        .child(
                                            div()
                                                .w(px(style.swatch_width_px))
                                                .h(px(style.swatch_height_px))
                                                .bg(hsla)
                                                .border_1()
                                                .border_color(cx.theme().border)
                                                .rounded_sm(),
                                        )
                                        .child(
                                            Input::new(&selected_color_input)
                                                .w(px(style.hex_box_width_px))
                                                .with_size(gpui_component::Size::Small)
                                                .font_family(text_font_family.clone())
                                                .text_size(text_size)
                                                .suffix(
                                                    Clipboard::new("multi-mixer-copy")
                                                        .value(hex.clone()),
                                                )
                                                .when(
                                                    selected_color_input_error.is_some(),
                                                    |input| input.border_color(cx.theme().danger),
                                                ),
                                        ),
                                ),
                        ),
                )
                .child(gpui_component::divider::Divider::horizontal())
                .child(v_flex().gap_4().px_4().pt_4().pb_4().child(sliders_flex)),
        )
}
