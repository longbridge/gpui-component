use super::paint_vector::{
    has_non_zero_corner_radius, paint_corner_occluder, paint_domain_background, paint_domain_border,
};
use super::state::{ColorFieldRenderer, ColorFieldState, FieldThumbPosition};
use crate::stories::color_primitives_story::color_thumb::ColorThumb;
use gpui::{prelude::*, *};
use gpui_component::{ActiveTheme as _, ElementExt as _, StyledExt as _};
use std::sync::Arc;

#[derive(Clone)]
struct ColorFieldDrag(EntityId);

impl Render for ColorFieldDrag {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        Empty
    }
}

impl Render for ColorFieldState {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        ColorField::new(&cx.entity())
    }
}

#[derive(IntoElement)]
pub struct ColorField {
    state: Entity<ColorFieldState>,
}

struct ColorFieldLayout {
    style: StyleRefinement,
    is_rect: bool,
    is_circle: bool,
    corner_radii: Corners<Pixels>,
    border_color: Hsla,
    outside_color: Hsla,
    show_border: bool,
    renderer: ColorFieldRenderer,
    cached_image: Option<Arc<Image>>,
    samples_per_axis: usize,
    thumb_size: Pixels,
    thumb_offset: Pixels,
    thumb_color: Hsla,
    thumb_uv: (f32, f32),
    thumb_position: FieldThumbPosition,
    should_clip_rect_surface: bool,
}

impl ColorField {
    pub fn new(state: &Entity<ColorFieldState>) -> Self {
        Self {
            state: state.clone(),
        }
    }

    fn compute_layout(state: &ColorFieldState, window: &Window, cx: &App) -> ColorFieldLayout {
        let domain = state.domain.as_ref();
        let model = state.model.as_ref();

        ColorFieldLayout {
            style: state.style.clone(),
            is_rect: domain.is_rect(),
            is_circle: domain.is_circle(),
            corner_radii: state.resolved_corner_radii(window),
            border_color: cx.theme().border,
            outside_color: cx.theme().background,
            show_border: state.show_border,
            renderer: state.renderer,
            cached_image: state.cached_image(),
            samples_per_axis: state.samples_per_axis,
            thumb_size: px(state.thumb_size),
            thumb_offset: px(-state.thumb_size * 0.5),
            thumb_color: model.thumb_color(&state.hsv),
            thumb_uv: state.thumb_uv(domain, model),
            thumb_position: state.thumb_position,
            should_clip_rect_surface: domain.is_rect()
                && state.thumb_position != FieldThumbPosition::EdgeToEdge,
        }
    }

    fn build_background_layer(
        layout: &ColorFieldLayout,
        domain: Arc<dyn super::super::domain::FieldDomain2D>,
        model: Arc<dyn super::super::model::ColorFieldModel2D>,
        hsv: crate::stories::color_primitives_story::color_spec::Hsv,
    ) -> AnyElement {
        match layout.renderer {
            ColorFieldRenderer::Vector => canvas(move |_, _, _| (), {
                let domain = domain.clone();
                let model = model.clone();
                let border_color = layout.border_color;
                let show_border = layout.show_border;
                let is_rect = layout.is_rect;
                let is_circle = layout.is_circle;
                let samples_per_axis = layout.samples_per_axis;
                move |bounds, _, window, _| {
                    paint_domain_background(
                        window,
                        bounds,
                        domain.as_ref(),
                        model.as_ref(),
                        hsv,
                        samples_per_axis,
                    );
                    if show_border && !is_rect && !is_circle {
                        paint_domain_border(window, bounds, domain.as_ref(), border_color);
                    }
                }
            })
            .size_full()
            .into_any_element(),
            ColorFieldRenderer::RasterImage => {
                let cached_image = layout.cached_image.clone();
                let has_cached_image = cached_image.is_some();
                let border_color = layout.border_color;
                let show_border = layout.show_border;
                let is_rect = layout.is_rect;
                let is_circle = layout.is_circle;

                div()
                    .size_full()
                    .relative()
                    .when_some(cached_image, |this, image| {
                        this.child(img(image).size_full().absolute().top_0().left_0())
                    })
                    // Intentionally no vector fallback here in raster mode.
                    // This avoids large first-frame quad bursts before image cache is ready.
                    .when(!has_cached_image, |this| this)
                    .when(show_border && !is_rect && !is_circle, |this| {
                        this.child(
                            canvas(move |_, _, _| (), {
                                let domain = domain.clone();
                                move |bounds, _, window, _| {
                                    paint_domain_border(window, bounds, domain.as_ref(), border_color);
                                }
                            })
                            .absolute()
                            .inset_0()
                            .size_full(),
                        )
                    })
                    .into_any_element()
            }
        }
    }

    fn thumb_node(layout: &ColorFieldLayout) -> Div {
        div()
            .absolute()
            .left(relative(layout.thumb_uv.0.clamp(0.0, 1.0)))
            .top(relative(layout.thumb_uv.1.clamp(0.0, 1.0)))
            .ml(layout.thumb_offset)
            .mt(layout.thumb_offset)
            .size(layout.thumb_size)
            .child(
                ColorThumb::new(layout.thumb_size)
                    .color(layout.thumb_color)
                    .active(false),
            )
    }

    fn with_global_mouse_watch(
        root: Stateful<Div>,
        state_entity: Entity<ColorFieldState>,
    ) -> Stateful<Div> {
        root.child(
            canvas(
                |bounds, window, _| window.insert_hitbox(bounds, HitboxBehavior::Normal),
                {
                    let state_entity = state_entity.clone();
                    move |_bounds: Bounds<Pixels>, hitbox: Hitbox, window, cx| {
                        let pointer = window.mouse_position();
                        let hovered = hitbox.is_hovered(window);
                        let has_any_drag = cx.has_active_drag();

                        let _ = state_entity.update(cx, |state, _| {
                            let external_drag_active = has_any_drag && !state.is_interaction_active();
                            state.update_cursor_state(
                                pointer,
                                hovered,
                                external_drag_active,
                                window,
                                &hitbox,
                            );
                        });

                        // Continue updating while dragging even if pointer moves outside
                        // this element's hitbox.
                        window.on_mouse_event({
                            let state_entity = state_entity.clone();
                            move |ev: &MouseMoveEvent, phase, window, cx| {
                                if !phase.bubble() {
                                    return;
                                }

                                let _ = state_entity.update(cx, |state, cx| {
                                    state.handle_active_move(ev.position, window, cx);
                                });
                            }
                        });

                        // End interaction even when release occurs outside this element.
                        window.on_mouse_event({
                            let state_entity = state_entity.clone();
                            move |ev: &MouseUpEvent, phase, window, cx| {
                                if !phase.bubble() {
                                    return;
                                }
                                let _ = state_entity.update(cx, |state, cx| {
                                    state.handle_pointer_release(ev.position, window, cx);
                                });
                            }
                        });
                    }
                },
            )
            .absolute()
            .inset_0(),
        )
    }

    fn with_interaction_handlers(
        root: Stateful<Div>,
        state_entity: Entity<ColorFieldState>,
        entity_id: EntityId,
        window: &mut Window,
    ) -> Stateful<Div> {
        root.on_mouse_down(
            MouseButton::Left,
            window.listener_for(
                &state_entity,
                |state: &mut ColorFieldState,
                 ev: &MouseDownEvent,
                 window: &mut Window,
                 cx: &mut Context<ColorFieldState>| {
                    if !state.contains_pointer(ev.position) {
                        return;
                    }
                    state.begin_interaction();
                    cx.stop_propagation();
                    state.update_from_mouse(ev.position, window, cx);
                    cx.notify();
                },
            ),
        )
        .on_mouse_up(
            MouseButton::Left,
            window.listener_for(
                &state_entity,
                |state: &mut ColorFieldState,
                 ev: &MouseUpEvent,
                 window: &mut Window,
                 cx: &mut Context<ColorFieldState>| {
                    state.handle_pointer_release(ev.position, window, cx);
                },
            ),
        )
        .on_mouse_move(window.listener_for(
            &state_entity,
            |state: &mut ColorFieldState,
             ev: &MouseMoveEvent,
             _: &mut Window,
             cx: &mut Context<ColorFieldState>| {
                state.update_hover_inside_domain(ev.position, cx);
            },
        ))
        .on_drag(
            ColorFieldDrag(entity_id),
            |drag: &ColorFieldDrag, _, _, cx: &mut App| {
                cx.stop_propagation();
                cx.new(|_| drag.clone())
            },
        )
        .on_drag_move(window.listener_for(
            &state_entity,
            move |state: &mut ColorFieldState,
                  ev: &DragMoveEvent<ColorFieldDrag>,
                  window: &mut Window,
                  cx: &mut Context<ColorFieldState>| {
                if !state.is_interaction_active() {
                    return;
                }
                if ev.drag(cx).0 != entity_id {
                    return;
                }
                state.update_from_mouse(ev.event.position, window, cx);
            },
        ))
    }

    fn with_prepaint(
        root: Stateful<Div>,
        state_entity: Entity<ColorFieldState>,
    ) -> Stateful<Div> {
        root.on_prepaint(move |bounds, _, cx| {
            state_entity.update(cx, |state, cx| {
                state.refresh_bounds_and_render_cache(bounds, cx);
            });
        })
    }
}

impl RenderOnce for ColorField {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let state_entity = self.state.clone();
        let entity_id = state_entity.entity_id();
        let state = state_entity.read(cx);
        let domain = state.domain.clone();
        let model = state.model.clone();
        let hsv = state.hsv;
        let layout = Self::compute_layout(&state, window, cx);
        let background_layer = Self::build_background_layer(&layout, domain.clone(), model.clone(), hsv);

        let root = div()
            .id(state.id.clone())
            .size_full()
            .relative()
            .refine_style(&layout.style)
            .child(
                div()
                    .absolute()
                    .inset_0()
                    .when(layout.should_clip_rect_surface, |this| {
                        this.corner_radii(layout.corner_radii).overflow_hidden()
                    })
                    .child(background_layer),
            )
            .when(layout.show_border && layout.is_circle, |this| {
                this.child(
                    div()
                        .absolute()
                        .inset_0()
                        .rounded_full()
                        .border_1()
                        .border_color(layout.border_color),
                )
            })
            .when(
                layout.is_rect
                    && has_non_zero_corner_radius(layout.corner_radii)
                    && layout.thumb_position == FieldThumbPosition::EdgeToEdge,
                |this| {
                    this.child(
                        canvas(
                            move |_, _, _| (),
                            move |bounds, _, window, _| {
                                paint_corner_occluder(window, bounds, layout.corner_radii, layout.outside_color)
                            },
                        )
                        .absolute()
                        .size_full(),
                    )
                },
            )
            .when(
                layout.show_border
                    && layout.is_rect
                    && layout.thumb_position == FieldThumbPosition::EdgeToEdge,
                |this| {
                    this.child(
                        div()
                            .absolute()
                            .size_full()
                            .corner_radii(layout.corner_radii)
                            .border_1()
                            .border_color(layout.border_color),
                    )
                },
            )
            .when(layout.thumb_position != FieldThumbPosition::EdgeToEdge, |this| {
                this.child(
                    div()
                        .absolute()
                        .inset_0()
                        .when(layout.should_clip_rect_surface, |this| {
                            this.corner_radii(layout.corner_radii).overflow_hidden()
                        })
                        .child(Self::thumb_node(&layout)),
                )
            })
            .when(layout.thumb_position == FieldThumbPosition::EdgeToEdge, |this| {
                this.child(Self::thumb_node(&layout))
            })
            .when(
                layout.is_rect
                    && has_non_zero_corner_radius(layout.corner_radii)
                    && layout.thumb_position != FieldThumbPosition::EdgeToEdge,
                |this| {
                    this.child(
                        canvas(
                            move |_, _, _| (),
                            move |bounds, _, window, _| {
                                paint_corner_occluder(window, bounds, layout.corner_radii, layout.outside_color)
                            },
                        )
                        .absolute()
                        .size_full(),
                    )
                },
            )
            .when(
                layout.show_border
                    && layout.is_rect
                    && layout.thumb_position != FieldThumbPosition::EdgeToEdge,
                |this| {
                    this.child(
                        div()
                            .absolute()
                            .size_full()
                            .corner_radii(layout.corner_radii)
                            .border_1()
                            .border_color(layout.border_color),
                    )
                },
            );

        let root = Self::with_global_mouse_watch(root, state_entity.clone());
        let root = Self::with_interaction_handlers(root, state_entity.clone(), entity_id, window);
        Self::with_prepaint(root, state_entity)
    }
}
