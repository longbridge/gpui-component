use gpui::{
    anchored, canvas, deferred, div, prelude::FluentBuilder as _, px, relative, App, AppContext,
    Bounds, Context, Corner, ElementId, Entity, EventEmitter, FocusHandle, Focusable, Hsla,
    InteractiveElement as _, IntoElement, KeyBinding, MouseButton, ParentElement, Pixels, Point,
    Render, SharedString, StatefulInteractiveElement as _, Styled, Subscription, Window,
};

use crate::{
    actions::Cancel,
    button::{Button, ButtonVariants},
    divider::Divider,
    h_flex,
    input::{InputEvent, TextInput},
    tooltip::Tooltip,
    v_flex, ActiveTheme as _, Colorize as _, Icon, Selectable as _, Sizable, Size, StyleSized,
};

const CONTEXT: &'static str = "ColorPicker";

pub fn init(cx: &mut App) {
    cx.bind_keys([KeyBinding::new("escape", Cancel, Some(CONTEXT))])
}

#[derive(Clone)]
pub enum ColorPickerEvent {
    Change(Option<Hsla>),
}

fn color_palettes() -> Vec<Vec<Hsla>> {
    use crate::colors::DEFAULT_COLOR;
    use itertools::Itertools as _;

    macro_rules! c {
        ($color:tt) => {
            DEFAULT_COLOR
                .$color
                .keys()
                .sorted()
                .map(|k| DEFAULT_COLOR.$color.get(k).map(|c| c.hsla).unwrap())
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

pub struct ColorPicker {
    id: ElementId,
    focus_handle: FocusHandle,
    value: Option<Hsla>,
    featured_colors: Vec<Hsla>,
    hovered_color: Option<Hsla>,
    label: Option<SharedString>,
    icon: Option<Icon>,
    size: Size,
    anchor: Corner,
    color_input: Entity<TextInput>,

    open: bool,
    bounds: Bounds<Pixels>,
    _subscriptions: Vec<Subscription>,
}

impl ColorPicker {
    pub fn new(id: impl Into<ElementId>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let color_input = cx.new(|cx| TextInput::new(window, cx).small());

        let _subscriptions = vec![cx.subscribe_in(
            &color_input,
            window,
            |this, _, ev: &InputEvent, window, cx| match ev {
                InputEvent::Change(value) => {
                    if let Ok(color) = Hsla::parse_hex(value) {
                        this.value = Some(color);
                        this.hovered_color = Some(color);
                    }
                }
                InputEvent::PressEnter { .. } => {
                    let val = this.color_input.read(cx).text();
                    if let Ok(color) = Hsla::parse_hex(&val) {
                        this.open = false;
                        this.update_value(Some(color), true, window, cx);
                    }
                }
                _ => {}
            },
        )];

        Self {
            id: id.into(),
            focus_handle: cx.focus_handle(),
            featured_colors: vec![
                crate::black(),
                crate::gray_600(),
                crate::gray_400(),
                crate::white(),
                crate::red_600(),
                crate::orange_600(),
                crate::yellow_600(),
                crate::green_600(),
                crate::blue_600(),
                crate::indigo_600(),
                crate::purple_600(),
            ],
            value: None,
            hovered_color: None,
            size: Size::Medium,
            label: None,
            icon: None,
            anchor: Corner::TopLeft,
            color_input,
            open: false,
            bounds: Bounds::default(),
            _subscriptions,
        }
    }

    /// Set the featured colors to be displayed in the color picker.
    ///
    /// This is used to display a set of colors that the user can quickly select from,
    /// for example provided user's last used colors.
    pub fn featured_colors(mut self, colors: Vec<Hsla>) -> Self {
        self.featured_colors = colors;
        self
    }

    /// Set default color value.
    pub fn default_value(mut self, value: Hsla) -> Self {
        self.value = Some(value);
        self
    }

    /// Set current color value.
    pub fn set_value(&mut self, value: Hsla, window: &mut Window, cx: &mut Context<Self>) {
        self.update_value(Some(value), false, window, cx)
    }

    /// Get current color value.
    pub fn value(&self) -> Option<Hsla> {
        self.value
    }

    /// Set the size of the color picker, default is `Size::Medium`.
    pub fn size(mut self, size: Size) -> Self {
        self.size = size;
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

    fn on_escape(&mut self, _: &Cancel, _: &mut Window, cx: &mut Context<Self>) {
        if !self.open {
            cx.propagate();
        }

        self.open = false;
        cx.notify();
    }

    fn toggle_picker(&mut self, _: &gpui::ClickEvent, _: &mut Window, cx: &mut Context<Self>) {
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
        self.color_input.update(cx, |view, cx| {
            if let Some(value) = value {
                view.set_text(value.to_hex(), window, cx);
            } else {
                view.set_text("", window, cx);
            }
        });
        if emit {
            cx.emit(ColorPickerEvent::Change(value));
        }
        cx.notify();
    }

    fn render_item(
        &self,
        color: Hsla,
        clickable: bool,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
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
                        .shadow_sm()
                })
                .active(|this| this.border_color(color.darken(0.5)).bg(color.darken(0.2)))
                .on_mouse_move(cx.listener(move |view, _, _, cx| {
                    view.hovered_color = Some(color);
                    cx.notify();
                }))
                .on_click(cx.listener(move |view, _, window, cx| {
                    view.update_value(Some(color), true, window, cx);
                    view.open = false;
                    cx.notify();
                }))
            })
    }

    fn render_colors(&self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .gap_3()
            .child(
                h_flex().gap_1().children(
                    self.featured_colors
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
            .when_some(self.hovered_color, |this, hovered_color| {
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
                        .child(self.color_input.clone()),
                )
            })
    }

    fn resolved_corner(&self, bounds: Bounds<Pixels>) -> Point<Pixels> {
        bounds.corner(match self.anchor {
            Corner::TopLeft => Corner::BottomLeft,
            Corner::TopRight => Corner::BottomRight,
            Corner::BottomLeft => Corner::TopLeft,
            Corner::BottomRight => Corner::TopRight,
        })
    }
}

impl Sizable for ColorPicker {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }
}
impl EventEmitter<ColorPickerEvent> for ColorPicker {}
impl Focusable for ColorPicker {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for ColorPicker {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let display_title: SharedString = if let Some(value) = self.value {
            value.to_hex()
        } else {
            "".to_string()
        }
        .into();

        let view = cx.entity().clone();

        div()
            .id(self.id.clone())
            .key_context(CONTEXT)
            .track_focus(&self.focus_handle)
            .on_action(cx.listener(Self::on_escape))
            .child(
                h_flex()
                    .id("color-picker-input")
                    .gap_2()
                    .items_center()
                    .input_text_size(self.size)
                    .line_height(relative(1.))
                    .when_some(self.icon.clone(), |this, icon| {
                        this.child(
                            Button::new("btn")
                                .ghost()
                                .selected(self.open)
                                .with_size(self.size)
                                .icon(icon.clone()),
                        )
                    })
                    .when_none(&self.icon, |this| {
                        this.child(
                            div()
                                .id("color-picker-square")
                                .bg(cx.theme().background)
                                .border_1()
                                .border_color(cx.theme().input)
                                .rounded(cx.theme().radius)
                                .shadow_sm()
                                .overflow_hidden()
                                .size_with(self.size)
                                .when_some(self.value, |this, value| {
                                    this.bg(value)
                                        .border_color(value.darken(0.3))
                                        .when(self.open, |this| this.border_2())
                                })
                                .when(!display_title.is_empty(), |this| {
                                    this.tooltip(move |_, cx| {
                                        cx.new(|_| Tooltip::new(display_title.clone())).into()
                                    })
                                }),
                        )
                    })
                    .when_some(self.label.clone(), |this, label| this.child(label))
                    .on_click(cx.listener(Self::toggle_picker))
                    .child(
                        canvas(
                            move |bounds, _, cx| view.update(cx, |r, _| r.bounds = bounds),
                            |_, _, _, _| {},
                        )
                        .absolute()
                        .size_full(),
                    ),
            )
            .when(self.open, |this| {
                this.child(
                    deferred(
                        anchored()
                            .anchor(self.anchor)
                            .snap_to_window_with_margin(px(8.))
                            .position(self.resolved_corner(self.bounds))
                            .child(
                                div()
                                    .occlude()
                                    .map(|this| match self.anchor {
                                        Corner::TopLeft | Corner::TopRight => this.mt_1p5(),
                                        Corner::BottomLeft | Corner::BottomRight => this.mb_1p5(),
                                    })
                                    .w_72()
                                    .overflow_hidden()
                                    .rounded(cx.theme().radius)
                                    .p_3()
                                    .border_1()
                                    .border_color(cx.theme().border)
                                    .shadow_lg()
                                    .rounded(cx.theme().radius)
                                    .bg(cx.theme().background)
                                    .child(self.render_colors(window, cx))
                                    .on_mouse_up_out(
                                        MouseButton::Left,
                                        cx.listener(|view, _, window, cx| {
                                            view.on_escape(&Cancel, window, cx)
                                        }),
                                    ),
                            ),
                    )
                    .with_priority(1),
                )
            })
    }
}
