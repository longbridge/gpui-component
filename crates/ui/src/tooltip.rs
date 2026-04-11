use std::{cell::Cell, rc::Rc, time::Duration};

use gpui::{
    Action, AnyElement, AnyView, App, AppContext, Bounds, Context, ElementId, Half, IntoElement,
    ParentElement, Pixels, Render, SharedString, StatefulInteractiveElement, StyleRefinement,
    Styled, Task, Window, deferred, div, point, prelude::FluentBuilder, px,
};

use crate::{
    ActiveTheme, Anchor, StyledExt,
    anchored::anchored,
    animation::{Transition, ease_in_out_cubic, ease_out_cubic},
    h_flex,
    kbd::Kbd,
    root::Root,
    text::Text,
};

pub(crate) fn init(_cx: &mut App) {
    // No app-level init needed — TooltipOverlay is per-window via Root.
}

// ── Tooltip view (unchanged API) ────────────────────────────────────────────

enum TooltipContext {
    Text(Text),
    Element(Box<dyn Fn(&mut Window, &mut App) -> AnyElement>),
}

/// A Tooltip element that can display text or custom content,
/// with optional key binding information.
pub struct Tooltip {
    style: StyleRefinement,
    content: TooltipContext,
    key_binding: Option<Kbd>,
    action: Option<(Box<dyn Action>, Option<SharedString>)>,
}

impl Tooltip {
    /// Create a Tooltip with a text content.
    pub fn new(text: impl Into<Text>) -> Self {
        Self {
            style: StyleRefinement::default(),
            content: TooltipContext::Text(text.into()),
            key_binding: None,
            action: None,
        }
    }

    /// Create a Tooltip with a custom element.
    pub fn element<E, F>(builder: F) -> Self
    where
        E: IntoElement,
        F: Fn(&mut Window, &mut App) -> E + 'static,
    {
        Self {
            style: StyleRefinement::default(),
            key_binding: None,
            action: None,
            content: TooltipContext::Element(Box::new(move |window, cx| {
                builder(window, cx).into_any_element()
            })),
        }
    }

    /// Set Action to display key binding information for the tooltip if it exists.
    pub fn action(mut self, action: &dyn Action, context: Option<&str>) -> Self {
        self.action = Some((action.boxed_clone(), context.map(SharedString::new)));
        self
    }

    /// Set KeyBinding information for the tooltip.
    pub fn key_binding(mut self, key_binding: Option<Kbd>) -> Self {
        self.key_binding = key_binding;
        self
    }

    /// Build the tooltip and return it as an `AnyView`.
    pub fn build(self, _: &mut Window, cx: &mut App) -> AnyView {
        cx.new(|_| self).into()
    }
}

impl FluentBuilder for Tooltip {}
impl Styled for Tooltip {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}
impl Render for Tooltip {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let key_binding = if let Some(key_binding) = &self.key_binding {
            Some(key_binding.clone())
        } else {
            if let Some((action, context)) = &self.action {
                Kbd::binding_for_action(
                    action.as_ref(),
                    context.as_ref().map(|s| s.as_ref()),
                    window,
                )
            } else {
                None
            }
        };

        div().child(
            // Wrap in a child, to ensure the left margin is applied to the tooltip
            h_flex()
                .font_family(cx.theme().font_family.clone())
                .m_3()
                .bg(cx.theme().popover)
                .text_color(cx.theme().popover_foreground)
                .bg(cx.theme().popover)
                .border_1()
                .border_color(cx.theme().border)
                .shadow_md()
                .rounded(px(6.))
                .justify_between()
                .py_0p5()
                .px_2()
                .text_sm()
                .gap_3()
                .refine_style(&self.style)
                .map(|this| {
                    this.child(div().map(|this| match self.content {
                        TooltipContext::Text(ref text) => this.child(text.clone()),
                        TooltipContext::Element(ref builder) => this.child(builder(window, cx)),
                    }))
                })
                .when_some(key_binding, |this, kbd| {
                    this.child(
                        div()
                            .text_xs()
                            .flex_shrink_0()
                            .text_color(cx.theme().muted_foreground)
                            .child(kbd.appearance(false)),
                    )
                }),
        )
    }
}

// ── Managed tooltip system ──────────────────────────────────────────────────

/// Grace period: if a tooltip was hidden within this time, skip delay for next show.
const GRACE_PERIOD: Duration = Duration::from_millis(300);
/// Delay before showing a tooltip when no tooltip is currently active.
const SHOW_DELAY: Duration = Duration::from_millis(500);
/// Duration of the slide-down enter animation.
const ENTER_DURATION: Duration = Duration::from_millis(150);
/// Duration of the position-slide animation when switching tooltips.
const SLIDE_DURATION: Duration = Duration::from_millis(200);

/// Content for a managed tooltip.
#[derive(Clone)]
pub(crate) struct TooltipContent {
    pub build: Rc<dyn Fn(&mut Window, &mut App) -> AnyView>,
    pub trigger_bounds: Bounds<Pixels>,
}

/// Manages tooltip lifecycle: delay, grace period, animations, and rendering.
///
/// A single instance lives in [`Root`] per window. Components register hover
/// via [`ManagedTooltipExt::managed_tooltip`] which calls into this overlay.
pub struct TooltipOverlay {
    content: Option<TooltipContent>,
    prev_trigger_bounds: Option<Bounds<Pixels>>,
    epoch: usize,
    had_recent_tooltip: bool,
    animation_epoch: usize,
    is_switching: bool,

    _show_task: Option<Task<()>>,
    _hide_task: Option<Task<()>>,
}

impl TooltipOverlay {
    pub fn new() -> Self {
        Self {
            content: None,
            prev_trigger_bounds: None,
            epoch: 0,
            had_recent_tooltip: false,
            animation_epoch: 0,
            is_switching: false,
            _show_task: None,
            _hide_task: None,
        }
    }

    fn next_epoch(&mut self) -> usize {
        self.epoch += 1;
        self.epoch
    }

    /// Request showing a tooltip. If another tooltip is active or was recently
    /// hidden, shows immediately with a slide animation. Otherwise starts a delay.
    pub(crate) fn request_show(
        &mut self,
        content: TooltipContent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        // Cancel any pending hide
        self._hide_task = None;

        let was_visible = self.content.is_some();
        let in_grace = self.had_recent_tooltip;

        if was_visible || in_grace {
            // Switch: show immediately with slide animation
            self.prev_trigger_bounds = self.content.as_ref().map(|c| c.trigger_bounds);
            self.content = Some(content);
            self._show_task = None;
            self.is_switching = was_visible;
            self.animation_epoch += 1;
            cx.notify();
        } else {
            // New: delay then show with slideDown
            let epoch = self.next_epoch();
            let content = content.clone();
            self._show_task = Some(cx.spawn_in(window, async move |this, cx| {
                cx.background_executor().timer(SHOW_DELAY).await;
                let _ = this.update_in(cx, |this, _, cx| {
                    if this.epoch != epoch {
                        return;
                    }

                    this.content = Some(content);
                    this.prev_trigger_bounds = None;
                    this.is_switching = false;
                    this.animation_epoch += 1;
                    cx.notify();
                });
            }));
        }
    }

    /// Request hiding the current tooltip. Starts a brief grace period so that
    /// moving to another tooltip-bearing element feels instant.
    pub(crate) fn request_hide(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        // Cancel any pending show
        self._show_task = None;

        if self.content.is_none() {
            return;
        }

        let epoch = self.next_epoch();
        self.had_recent_tooltip = true;

        self._hide_task = Some(cx.spawn_in(window, async move |this, cx| {
            cx.background_executor().timer(GRACE_PERIOD).await;
            let _ = this.update_in(cx, |this, _, cx| {
                if this.epoch != epoch {
                    return;
                }
                this.content = None;
                this.prev_trigger_bounds = None;
                this.had_recent_tooltip = false;
                cx.notify();
            });
        }));
    }
}

impl Render for TooltipOverlay {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let Some(content) = self.content.as_ref() else {
            return div().into_any_element();
        };

        let content_view = (content.build)(window, cx);
        let trigger_bounds = content.trigger_bounds;
        let animation_epoch = self.animation_epoch;
        let is_switching = self.is_switching;
        let prev_trigger_bounds = self.prev_trigger_bounds;

        let anchor_position = point(
            trigger_bounds.origin.x + trigger_bounds.size.width.half(),
            trigger_bounds.origin.y,
        );

        deferred(
            anchored()
                .snap_to_window_with_margin(px(4.))
                .position(anchor_position)
                .anchor(Anchor::BottomCenter)
                .child(div().child(content_view).map(|el| {
                    if is_switching {
                        let Some(prev_bounds) = prev_trigger_bounds else {
                            return el.into_any_element();
                        };

                        let is_same_y =
                            (trigger_bounds.origin.y - prev_bounds.origin.y).abs() < px(10.);
                        if !is_same_y {
                            // If the new trigger is at a different Y level, don't slide horizontally
                            // to avoid weird diagonal movement. (We could consider sliding vertically
                            // in this case, but it might be less visually clear.)
                            return el.into_any_element();
                        }

                        let dx = trigger_bounds.center().x - prev_bounds.center().x;

                        Transition::new(SLIDE_DURATION)
                            .ease(ease_in_out_cubic)
                            .slide_x(-dx, px(0.))
                            .apply(
                                el,
                                ElementId::NamedInteger(
                                    "tooltip-slide".into(),
                                    animation_epoch as u64,
                                ),
                            )
                            .into_any_element()
                    } else {
                        // New tooltip: slideDown + fadeIn
                        Transition::new(ENTER_DURATION)
                            .ease(ease_out_cubic)
                            .slide_y(px(4.), px(0.))
                            .fade(0.0, 1.0)
                            .apply(
                                el,
                                ElementId::NamedInteger(
                                    "tooltip-enter".into(),
                                    animation_epoch as u64,
                                ),
                            )
                            .into_any_element()
                    }
                })),
        )
        .with_priority(2)
        .into_any_element()
    }
}

// ── Extension trait for managed tooltips ─────────────────────────────────────

// ── Shared tooltip state for components ─────────────────────────────────────

/// Shared tooltip state that components (Button, Switch, Checkbox, Radio, etc.)
/// can embed to get `.tooltip()` support with minimal boilerplate.
#[derive(Default)]
pub(crate) struct ComponentTooltip {
    pub text: Option<(
        SharedString,
        Option<(Rc<Box<dyn Action>>, Option<SharedString>)>,
    )>,
    pub builder: Option<Rc<dyn Fn(&mut Window, &mut App) -> AnyView>>,
}

impl ComponentTooltip {
    /// Apply this tooltip to a `Stateful<Div>` (or any `ManagedTooltipExt` element).
    pub fn apply<E: ManagedTooltipExt>(self, el: E) -> E {
        if let Some(builder) = self.builder {
            el.managed_tooltip(move |window, cx| builder(window, cx))
        } else if let Some((text, action)) = self.text {
            el.managed_tooltip(move |window, cx| {
                Tooltip::new(text.clone())
                    .when_some(action.clone(), |this, (action, context)| {
                        this.action(
                            action.boxed_clone().as_ref(),
                            context.as_ref().map(|c| c.as_ref()),
                        )
                    })
                    .build(window, cx)
            })
        } else {
            el
        }
    }
}

// ── Internal managed tooltip trait ──────────────────────────────────────────

pub(crate) trait ManagedTooltipExt: StatefulInteractiveElement + crate::ElementExt + Sized {
    fn managed_tooltip(
        self,
        build_tooltip: impl Fn(&mut Window, &mut App) -> AnyView + 'static,
    ) -> Self {
        let build_tooltip = Rc::new(build_tooltip);
        let trigger_bounds_cell: Rc<Cell<Bounds<Pixels>>> = Rc::new(Cell::new(Bounds::default()));
        let bounds_writer = trigger_bounds_cell.clone();

        self.on_prepaint(move |bounds, _, _| {
            bounds_writer.set(bounds);
        })
        .on_hover({
            let trigger_bounds_cell = trigger_bounds_cell.clone();
            let build_tooltip = build_tooltip.clone();
            move |hovered, window, cx| {
                if let Some(overlay) = Root::tooltip_overlay(window, cx) {
                    if *hovered {
                        let bounds = trigger_bounds_cell.get();
                        overlay.update(cx, |o: &mut TooltipOverlay, cx| {
                            o.request_show(
                                TooltipContent {
                                    build: build_tooltip.clone(),
                                    trigger_bounds: bounds,
                                },
                                window,
                                cx,
                            );
                        });
                    } else {
                        overlay.update(cx, |o: &mut TooltipOverlay, cx| {
                            o.request_hide(window, cx);
                        });
                    }
                }
            }
        })
    }
}

impl<E: StatefulInteractiveElement + crate::ElementExt> ManagedTooltipExt for E {}
