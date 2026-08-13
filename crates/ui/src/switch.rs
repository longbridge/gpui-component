use crate::{
    ActiveTheme, Disableable, Side, Sizable, Size, StyledExt, text::Text, tooltip::ComponentTooltip,
};
use gpui::{
    Animation, AnimationExt as _, App, Background, ElementId, Hsla, InteractiveElement,
    IntoElement, ParentElement as _, RenderOnce, SharedString, StyleRefinement, Styled, Window,
    div, prelude::FluentBuilder as _, px,
};
use gpui_base::{Switch as BaseSwitch, SwitchThumb, SwitchTrack};
use std::{rc::Rc, time::Duration};

/// A Switch element that can be toggled on or off.
#[derive(IntoElement)]
pub struct Switch {
    id: ElementId,
    style: StyleRefinement,
    checked: bool,
    disabled: bool,
    label: Option<Text>,
    label_side: Side,
    on_click: Option<Rc<dyn Fn(&bool, &mut Window, &mut App)>>,
    size: Size,
    color: Option<Hsla>,
    tooltip: ComponentTooltip,
}

impl Switch {
    /// Create a new Switch element.
    pub fn new(id: impl Into<ElementId>) -> Self {
        let id: ElementId = id.into();
        Self {
            id: id.clone(),
            style: StyleRefinement::default(),
            checked: false,
            disabled: false,
            label: None,
            on_click: None,
            label_side: Side::Right,
            size: Size::Medium,
            color: None,
            tooltip: ComponentTooltip::default(),
        }
    }

    /// Set the checked state of the switch.
    pub fn checked(mut self, checked: bool) -> Self {
        self.checked = checked;
        self
    }

    /// Set the label of the switch.
    pub fn label(mut self, label: impl Into<Text>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Add a click handler for the switch.
    pub fn on_click<F>(mut self, handler: F) -> Self
    where
        F: Fn(&bool, &mut Window, &mut App) + 'static,
    {
        self.on_click = Some(Rc::new(handler));
        self
    }

    /// Set the background color of the switch when checked.
    /// Defaults to `cx.theme().primary`.
    pub fn color(mut self, color: impl Into<Hsla>) -> Self {
        self.color = Some(color.into());
        self
    }

    /// Set tooltip text for the switch.
    pub fn tooltip(mut self, tooltip: impl Into<SharedString>) -> Self {
        self.tooltip.text = Some((tooltip.into(), None));
        self
    }
}

impl Styled for Switch {
    fn style(&mut self) -> &mut gpui::StyleRefinement {
        &mut self.style
    }
}

impl Sizable for Switch {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }
}

impl Disableable for Switch {
    fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl RenderOnce for Switch {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let checked = self.checked;
        let on_click = self.on_click.clone();
        let toggle_state = window.use_keyed_state(self.id.clone(), cx, |_, _| checked);

        let checked_bg = self
            .color
            .map(Background::from)
            .unwrap_or(cx.theme().tokens.primary.into());
        let unchecked_bg: Background = cx.theme().tokens.switch.into();
        let disabled_checked_bg = checked_bg.clone().opacity(0.5);
        let toggle_bg: Background = cx.theme().tokens.switch_thumb.into();

        let (bg_width, bg_height) = match self.size {
            Size::XSmall | Size::Small => (px(28.), px(16.)),
            _ => (px(36.), px(20.)),
        };
        let bar_width = match self.size {
            Size::XSmall | Size::Small => px(12.),
            _ => px(16.),
        };
        let inset = px(2.);
        let radius = if cx.theme().radius >= px(4.) {
            bg_height
        } else {
            cx.theme().radius
        };

        div().refine_style(&self.style).child(
            BaseSwitch::new(self.id.clone())
                .checked(checked)
                .disabled(self.disabled)
                .when_some(
                    self.label.as_ref().map(|l| l.get_text(cx)),
                    |this, label| this.accessibility_label(label),
                )
                .when_some(on_click, |this, on_click| {
                    let toggle_state = toggle_state.clone();
                    this.on_change(move |next, _, window, cx| {
                        _ = toggle_state.update(cx, |this, _| *this = checked);
                        on_click(&next, window, cx);
                    })
                })
                .h_flex()
                .gap_2()
                .items_start()
                .when(self.label_side.is_left(), |this| this.flex_row_reverse())
                .child(
                    // Switch Bar
                    SwitchTrack::new((self.id.clone(), "track"))
                        .checked(checked)
                        .disabled(self.disabled)
                        .when(cfg!(test), |this| {
                            this.debug_selector(|| "switch-bar".into())
                        })
                        .w(bg_width)
                        .h(bg_height)
                        .rounded(radius)
                        .flex()
                        .items_center()
                        .border(inset)
                        .border_color(cx.theme().transparent)
                        .when(!checked, |this| this.bg(unchecked_bg))
                        .styles(|styles| {
                            styles
                                .checked(|style| style.bg(checked_bg))
                                .disabled(|style| {
                                    style.when(checked, |style| style.bg(disabled_checked_bg))
                                })
                        })
                        .map(|this| self.tooltip.apply(this))
                        .child(
                            // Switch Toggle
                            SwitchThumb::new(checked)
                                .disabled(self.disabled)
                                .rounded(radius)
                                .size(bar_width)
                                .when(!checked, |this| this.left(px(0.)))
                                .when(!self.disabled, |this| this.bg(toggle_bg))
                                .styles(|styles| {
                                    styles
                                        .checked(|style| {
                                            style.left(bg_width - bar_width - inset * 2)
                                        })
                                        .disabled(|style| style.bg(toggle_bg.clone().opacity(0.35)))
                                })
                                .map(|this| {
                                    let prev_checked = toggle_state.read(cx);
                                    if !self.disabled && *prev_checked != checked {
                                        let duration = Duration::from_secs_f64(0.15);
                                        cx.spawn({
                                            let toggle_state = toggle_state.clone();
                                            async move |cx| {
                                                cx.background_executor().timer(duration).await;
                                                _ = toggle_state
                                                    .update(cx, |this, _| *this = checked);
                                            }
                                        })
                                        .detach();

                                        this.with_animation(
                                            ElementId::NamedInteger("move".into(), checked as u64),
                                            Animation::new(duration),
                                            move |this, delta| {
                                                let max_x = bg_width - bar_width - inset * 2;
                                                let x = if checked {
                                                    max_x * delta
                                                } else {
                                                    max_x - max_x * delta
                                                };
                                                this.left(x)
                                            },
                                        )
                                        .into_any_element()
                                    } else {
                                        let max_x = bg_width - bar_width - inset * 2;
                                        let x = if checked { max_x } else { px(0.) };
                                        this.left(x).into_any_element()
                                    }
                                }),
                        ),
                )
                .when_some(self.label, |this, label| {
                    this.child(
                        div()
                            .when(cfg!(test), |this| {
                                this.debug_selector(|| "switch-label".into())
                            })
                            .line_height(bg_height)
                            .child(label)
                            .map(|this| match self.size {
                                Size::XSmall | Size::Small => this.text_sm(),
                                _ => this.text_base(),
                            }),
                    )
                }),
        )
    }
}

#[cfg(test)]
mod tests {
    use std::{cell::Cell, rc::Rc};

    use gpui::{
        Context, KeyDownEvent, KeyUpEvent, Keystroke, Modifiers, Render,
        StatefulInteractiveElement as _, TestAppContext, VisualTestContext, point,
    };

    use super::*;

    struct SwitchHarness {
        disabled: bool,
        toggles: Rc<Cell<usize>>,
        parent_clicks: Rc<Cell<usize>>,
    }

    impl Render for SwitchHarness {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            let toggles = self.toggles.clone();
            let parent_clicks = self.parent_clicks.clone();
            div()
                .id("switch-parent")
                .tab_group()
                .size(px(100.))
                .on_click(move |_, _, _| parent_clicks.set(parent_clicks.get() + 1))
                .child(Switch::new("switch").disabled(self.disabled).on_click(
                    move |checked, _, _| {
                        assert!(*checked);
                        toggles.set(toggles.get() + 1);
                    },
                ))
        }
    }

    fn harness(
        cx: &mut TestAppContext,
        disabled: bool,
    ) -> (&mut VisualTestContext, Rc<Cell<usize>>, Rc<Cell<usize>>) {
        cx.update(crate::init);
        let toggles = Rc::new(Cell::new(0));
        let parent_clicks = Rc::new(Cell::new(0));
        let (_, cx) = cx.add_window_view({
            let toggles = toggles.clone();
            let parent_clicks = parent_clicks.clone();
            move |_, _| SwitchHarness {
                disabled,
                toggles,
                parent_clicks,
            }
        });
        cx.update(|window, cx| window.draw(cx).clear(cx));
        (cx, toggles, parent_clicks)
    }

    fn activate_key(cx: &mut VisualTestContext, key: &str) {
        let keystroke = Keystroke::parse(key).unwrap();
        cx.simulate_event(KeyDownEvent {
            keystroke: keystroke.clone(),
            is_held: false,
            prefer_character_input: false,
        });
        cx.simulate_event(KeyUpEvent { keystroke });
    }

    #[gpui::test]
    fn canonical_pointer_activation_fires_once_and_focuses(cx: &mut TestAppContext) {
        let (cx, toggles, _) = harness(cx, false);
        cx.simulate_click(point(px(10.), px(10.)), Modifiers::default());

        assert_eq!(toggles.get(), 1);
        cx.update(|window, cx| assert!(window.focused(cx).is_some()));
    }

    #[gpui::test]
    fn canonical_switch_supports_tab_enter_and_space(cx: &mut TestAppContext) {
        let (cx, toggles, _) = harness(cx, false);
        cx.update(|window, cx| window.focus_next(cx));
        cx.update(|window, cx| assert!(window.focused(cx).is_some()));

        activate_key(cx, "enter");
        activate_key(cx, "space");

        assert_eq!(toggles.get(), 2);
    }

    #[gpui::test]
    fn canonical_disabled_switch_is_inert_and_blocks_parent(cx: &mut TestAppContext) {
        let (cx, toggles, parent_clicks) = harness(cx, true);
        cx.simulate_click(point(px(10.), px(10.)), Modifiers::default());

        assert_eq!(toggles.get(), 0);
        assert_eq!(parent_clicks.get(), 0);
        cx.update(|window, cx| assert!(window.focused(cx).is_none()));
    }

    #[gpui::test]
    fn label_prepaints_with_the_base_switch_content(cx: &mut TestAppContext) {
        struct LabelHarness;

        impl Render for LabelHarness {
            fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
                div()
                    .debug_selector(|| "labeled-switch".into())
                    .child(Switch::new("switch").label("Airplane mode"))
            }
        }

        cx.update(crate::init);
        let (_, cx) = cx.add_window_view(|_, _| LabelHarness);
        cx.update(|window, cx| window.draw(cx).clear(cx));

        let bounds = cx
            .debug_bounds("labeled-switch")
            .expect("the complete labeled Switch must participate in prepaint");
        assert!(bounds.size.width > px(36.));
        let bar = cx
            .debug_bounds("switch-bar")
            .expect("the Switch bar must participate in prepaint");
        let label = cx
            .debug_bounds("switch-label")
            .expect("the Switch label must participate in prepaint");
        assert_eq!(bar.origin.y, label.origin.y);
    }
}
