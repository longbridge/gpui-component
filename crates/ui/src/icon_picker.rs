use gpui::{
    Anchor, App, AppContext, Context, Div, ElementId, Entity, EventEmitter, FocusHandle, Focusable,
    Hsla, InteractiveElement as _, IntoElement, KeyBinding, ParentElement, Render, RenderOnce,
    SharedString, Stateful, StatefulInteractiveElement as _, StyleRefinement, Styled,
    UniformListScrollHandle, Window, div, prelude::FluentBuilder as _, uniform_list,
};
use rust_i18n::t;
use strum::IntoEnumIterator;

use crate::{
    ActiveTheme as _, Colorize as _, Icon, IconName, IconNamed, Selectable, Sizable, Size,
    StyleSized,
    actions::Confirm,
    h_flex,
    input::{Input, InputState},
    popover::Popover,
    scroll::ScrollableElement,
    separator::Separator,
    tooltip::{ManagedTooltipExt as _, Tooltip},
    v_flex,
};

const CONTEXT: &'static str = "IconPicker";
pub(crate) fn init(cx: &mut App) {
    cx.bind_keys([KeyBinding::new(
        "enter",
        Confirm { secondary: false },
        Some(CONTEXT),
    )])
}

/// Events emitted by the [`IconPicker`].
#[derive(Clone)]
pub enum IconPickerEvent<T>
where
    T: 'static + Copy + IconNamed + IntoEnumIterator,
{
    Change(Option<T>),
}

fn icons<T>(allowed_icons: Option<Vec<T>>, search: &str) -> Vec<Vec<T>>
where
    T: 'static + Copy + IconNamed + IntoEnumIterator,
{
    let icons: Vec<T> = allowed_icons.unwrap_or_else(|| T::iter().collect());
    icons
        .into_iter()
        .filter(|icon| icon.name().to_lowercase().contains(search))
        .collect::<Vec<T>>()
        .chunks(6)
        .fold(vec![], |mut vec, chunk| {
            vec.push(Vec::from(chunk));
            vec
        })
}

/// State of the [`IconPicker`].
pub struct IconPickerState<T>
where
    T: 'static + Copy + IconNamed + IntoEnumIterator,
{
    allowed_icons: Option<Vec<T>>,
    focus_handle: FocusHandle,
    value: Option<T>,
    chosen_icon: Option<T>,
    scroll_handle: UniformListScrollHandle,
    search: Entity<InputState>,
    open: bool,
}

impl<T> IconPickerState<T>
where
    T: 'static + Copy + IconNamed + IntoEnumIterator,
{
    /// Create a new [`IconPickerState`].
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let search = cx
            .new(|cx| InputState::new(window, cx).placeholder(t!("IconPicker.search_placeholder")));

        Self {
            allowed_icons: None,
            focus_handle: cx.focus_handle(),
            value: None,
            chosen_icon: None,
            scroll_handle: UniformListScrollHandle::new(),
            search,
            open: false,
        }
    }

    /// Set the icons the user will pick from.
    pub fn allowed_icons(mut self, icons: impl Into<Vec<T>>) -> Self {
        let icons = icons.into();
        self.allowed_icons = Some(icons);
        self
    }

    /// Set default icon value.
    pub fn default_value(mut self, value: impl Into<T>) -> Self {
        let value = value.into();
        self.value = Some(value);
        self.chosen_icon = Some(value);
        self
    }

    /// Set current icon value.
    pub fn set_value(&mut self, value: impl Into<T>, window: &mut Window, cx: &mut Context<Self>) {
        self.update_value(Some(value.into()), false, window, cx)
    }

    /// Get current icon value.
    pub fn value(&self) -> Option<T> {
        self.value
    }

    fn on_confirm(&mut self, _: &Confirm, _: &mut Window, cx: &mut Context<Self>) {
        self.open = !self.open;
        cx.notify();
    }

    fn update_value(
        &mut self,
        value: Option<T>,
        emit: bool,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.value = value;
        self.chosen_icon = value;

        if emit {
            cx.emit(IconPickerEvent::Change(value));
        }
        cx.notify();
    }
}

impl<T> EventEmitter<IconPickerEvent<T>> for IconPickerState<T> where
    T: 'static + Copy + IconNamed + IntoEnumIterator
{
}

impl<T> Render for IconPickerState<T>
where
    T: 'static + Copy + IconNamed + IntoEnumIterator,
{
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        self.search.clone()
    }
}

impl<T> Focusable for IconPickerState<T>
where
    T: 'static + Copy + IconNamed + IntoEnumIterator,
{
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

/// An icon picker element.
#[derive(IntoElement)]
pub struct IconPicker<T>
where
    T: 'static + Copy + IconNamed + IntoEnumIterator,
{
    id: ElementId,
    style: StyleRefinement,
    state: Entity<IconPickerState<T>>,
    label: Option<SharedString>,
    icon: Option<Icon>,
    icons_color: Option<Hsla>,
    size: Size,
    anchor: Anchor,
}

impl<T> IconPicker<T>
where
    T: 'static + Copy + IconNamed + IntoEnumIterator,
{
    /// Create a new icon picker element with the given [`IconPickerState`].
    pub fn new(state: &Entity<IconPickerState<T>>) -> Self {
        Self {
            id: ("icon-picker", state.entity_id()).into(),
            style: StyleRefinement::default(),
            state: state.clone(),
            size: Size::Medium,
            label: None,
            icon: None,
            icons_color: None,
            anchor: Anchor::TopLeft,
        }
    }

    /// Set the icon to the icon picker button.
    ///
    /// If this is set the icon picker button will display this icon.
    /// Else it will display the icon of the current value.
    pub fn icon(mut self, icon: impl Into<Icon>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    /// Set the color of the icons in this picker.
    ///
    /// If this is not set, the theme's default will be used
    pub fn icons_color(mut self, color: impl Into<Hsla>) -> Self {
        self.icons_color = Some(color.into());
        self
    }

    /// Set the label to be displayed above the icon picker.
    ///
    /// Default is `None`.
    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Set the anchor corner of the icon picker.
    ///
    /// Default is `Anchor::TopLeft`.
    pub fn anchor(mut self, anchor: Anchor) -> Self {
        self.anchor = anchor;
        self
    }

    fn render_icon(
        state: Entity<IconPickerState<T>>,
        icon: T,
        icons_color: Option<Hsla>,
        window: &mut Window,
        cx: &mut App,
    ) -> Stateful<Div> {
        let bg = cx.theme().button;
        let border = cx.theme().input;
        div()
            .id(icon.name())
            .flex()
            .size_9()
            .bg(bg)
            .border_1()
            .border_color(border)
            .rounded(cx.theme().radius)
            .items_center()
            .justify_center()
            .child(
                Icon::new(icon)
                    .when_some(icons_color, |icon, color| icon.text_color(color))
                    .size_5(),
            )
            .hover(|this| this.border_color(border.darken(0.3)).bg(bg.lighten(0.1)))
            .active(|this| this.border_color(border.darken(0.5)).bg(bg.darken(0.2)))
            .on_hover(window.listener_for(&state, move |state, enter, _, cx| {
                if *enter {
                    state.chosen_icon = Some(icon);
                    cx.notify();
                }
            }))
            .on_click(window.listener_for(&state, move |state, _, window, cx| {
                state.open = false;
                state.update_value(Some(icon), true, window, cx);
                cx.notify();
            }))
    }

    fn render_icons(&self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let chosen_icon = self.state.read(cx).chosen_icon;

        let bg = cx.theme().button;
        let border = cx.theme().input;

        v_flex()
            .p_0p5()
            .gap_3()
            .h_72()
            .child(
                h_flex()
                    .gap_2()
                    .when_some(chosen_icon, |this, chosen_icon| {
                        this.child(
                            div()
                                .flex()
                                .min_size_8()
                                .bg(bg)
                                .border_1()
                                .border_color(border)
                                .rounded(cx.theme().radius)
                                .items_center()
                                .justify_center()
                                .child(
                                    Icon::new(chosen_icon)
                                        .when_some(self.icons_color, |icon, color| {
                                            icon.text_color(color)
                                        })
                                        .size_5(),
                                ),
                        )
                        .child(
                            div()
                                .id("clear-button")
                                .flex()
                                .min_size_8()
                                .bg(bg)
                                .border_1()
                                .border_color(border)
                                .rounded(cx.theme().radius)
                                .items_center()
                                .justify_center()
                                .hover(|this| {
                                    this.border_color(cx.theme().danger)
                                        .bg(cx.theme().button_danger_hover)
                                })
                                .child(
                                    Icon::new(IconName::Close)
                                        .text_color(cx.theme().danger)
                                        .size_4(),
                                )
                                .on_click(window.listener_for(
                                    &self.state,
                                    |state, _, window, cx| {
                                        state.open = false;
                                        state.update_value(None, true, window, cx);
                                    },
                                )),
                        )
                    })
                    .child(
                        Input::new(&self.state.read(cx).search)
                            .prefix(Icon::new(IconName::Search))
                            .px_2p5(),
                    ),
            )
            .child(Separator::horizontal())
            .child(self.render_icons_list(window, cx))
    }

    fn render_icons_list(&self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let state = self.state.clone();
        let icons_color = self.icons_color;
        let allowed_icons = state.read(cx).allowed_icons.clone();
        let search = state.read(cx).search.read(cx).value().trim().to_lowercase();
        let icons = icons(allowed_icons, &search);
        let scroll = state.read(cx).scroll_handle.clone();

        div().relative().flex_1().when_else(
            icons.is_empty(),
            |div| {
                div.text_center()
                    .justify_center()
                    .child(t!("IconPicker.empty"))
            },
            |div| {
                div.child(
                    uniform_list("list", icons.len(), {
                        let icons = icons.clone();
                        move |range, window, cx| {
                            let mut elements = Vec::with_capacity(range.len());
                            for i in range {
                                let icon_row = icons[i].clone();
                                elements.push(h_flex().gap_1().h_10().children(
                                    icon_row.iter().map(|icon| {
                                        Self::render_icon(
                                            state.clone(),
                                            *icon,
                                            icons_color,
                                            window,
                                            cx,
                                        )
                                    }),
                                ));
                            }
                            elements
                        }
                    })
                    .size_full()
                    .track_scroll(&scroll),
                )
                .vertical_scrollbar(&scroll)
            },
        )
    }
}

impl<T> Sizable for IconPicker<T>
where
    T: 'static + Copy + IconNamed + IntoEnumIterator,
{
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }
}

impl<T> Focusable for IconPicker<T>
where
    T: 'static + Copy + IconNamed + IntoEnumIterator,
{
    fn focus_handle(&self, cx: &App) -> FocusHandle {
        self.state.read(cx).focus_handle.clone()
    }
}

impl<T> Styled for IconPicker<T>
where
    T: 'static + Copy + IconNamed + IntoEnumIterator,
{
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl<T> RenderOnce for IconPicker<T>
where
    T: 'static + Copy + IconNamed + IntoEnumIterator,
{
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let state = self.state.read(cx);
        let display_title: SharedString = if let Some(value) = state.value {
            value.name()
        } else {
            SharedString::default()
        }
        .into();

        let focus_handle = state.focus_handle.clone().tab_stop(true);

        div()
            .id(self.id.clone())
            .key_context(CONTEXT)
            .track_focus(&focus_handle)
            .on_action(window.listener_for(&self.state, IconPickerState::on_confirm))
            .child(
                Popover::new("popover")
                    .open(state.open)
                    .w_72()
                    .on_open_change(window.listener_for(
                        &self.state,
                        |this, open: &bool, window, cx| {
                            this.open = *open;
                            this.search.update(cx, |search, cx| {
                                search.set_value("", window, cx);
                            });
                            if *open {
                                this.search.focus_handle(cx).focus(window, cx);
                            }
                            cx.notify();
                        },
                    ))
                    .trigger(IconPickerButton {
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
                        icon_color: self.icons_color,
                        selected: false,
                    })
                    .child(self.render_icons(window, cx)),
            )
    }
}

#[derive(IntoElement)]
struct IconPickerButton<T>
where
    T: 'static + Copy + IconNamed + IntoEnumIterator,
{
    id: ElementId,
    selected: bool,
    icon: Option<Icon>,
    icon_color: Option<Hsla>,
    value: Option<T>,
    size: Size,
    label: Option<SharedString>,
    tooltip: Option<SharedString>,
}

impl<T> Selectable for IconPickerButton<T>
where
    T: 'static + Copy + IconNamed + IntoEnumIterator,
{
    fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    fn is_selected(&self) -> bool {
        self.selected
    }
}

impl<T> Sizable for IconPickerButton<T>
where
    T: 'static + Copy + IconNamed + IntoEnumIterator,
{
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }
}

impl<T> RenderOnce for IconPickerButton<T>
where
    T: 'static + Copy + IconNamed + IntoEnumIterator,
{
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
                        .flex()
                        .bg(cx.theme().tokens.background)
                        .border_1()
                        .border_color(cx.theme().input)
                        .rounded(cx.theme().radius)
                        .size_with(self.size)
                        .items_center()
                        .justify_center()
                        .when_some(self.value, |this, value| {
                            this.items_center()
                                .child(
                                    Icon::new(value)
                                        .when_some(self.icon_color, |icon, color| {
                                            icon.text_color(color)
                                        })
                                        .size_5(),
                                )
                                .when(self.selected, |this| this.border_2())
                        })
                        .when_some(self.tooltip, |this, tooltip| {
                            this.managed_tooltip(move |window, cx| {
                                Tooltip::new(tooltip.clone()).build(window, cx)
                            })
                        }),
                )
            })
            .when_some(self.label, |this, label| this.child(label))
    }
}
