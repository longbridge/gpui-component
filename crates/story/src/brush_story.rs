use std::rc::Rc;

use gpui::{
    App, AppContext, Bounds, Context, Entity, Focusable, Hsla, InteractiveElement, IntoElement,
    MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent, ParentElement, PathBuilder, Pixels,
    Point, Render, Styled, Window, prelude::FluentBuilder, px,
};
use gpui_component::{
    ActiveTheme, Colorize as _, IconName, Sizable,
    button::Button,
    checkbox::Checkbox,
    h_flex,
    slider::{Slider, SliderState},
    v_flex,
};

use crate::section;

pub struct BrushStory {
    focus_handle: gpui::FocusHandle,
    canvas_size: (f32, f32),
    brush_size: Entity<SliderState>,
    brush_opacity: Entity<SliderState>,
    brush_color: Hsla,
    // Use Rc to avoid deep cloning on every render
    strokes: Rc<Vec<Stroke>>,
    current_stroke: Option<Stroke>,
    is_drawing: bool,
    show_grid: bool,
    canvas_bounds: Option<Bounds<Pixels>>,
}

#[derive(Clone, Debug)]
struct Stroke {
    points: Vec<Point<gpui::Pixels>>,
    color: Hsla,
    size: f32,
}

impl super::Story for BrushStory {
    fn title() -> &'static str {
        "Brush"
    }

    fn description() -> &'static str {
        "Interactive drawing canvas with brush controls."
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }
}

impl BrushStory {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn new(_: &mut Window, cx: &mut Context<Self>) -> Self {
        let brush_size = cx.new(|_| {
            SliderState::new()
                .min(1.)
                .max(50.)
                .default_value(5.)
                .step(1.)
        });

        let brush_opacity = cx.new(|_| {
            SliderState::new()
                .min(0.1)
                .max(1.0)
                .default_value(1.0)
                .step(0.05)
        });

        Self {
            focus_handle: cx.focus_handle(),
            canvas_size: (600.0, 400.0),
            brush_size,
            brush_opacity,
            brush_color: gpui::black(),
            strokes: Rc::new(vec![]),
            current_stroke: None,
            is_drawing: false,
            show_grid: false, // Disabled by default for better performance
            canvas_bounds: None,
        }
    }

    fn handle_mouse_down(
        &mut self,
        event: &MouseDownEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if event.button == MouseButton::Left {
            self.is_drawing = true;
            let brush_size = self.brush_size.read(cx).value().start();
            let brush_opacity = self.brush_opacity.read(cx).value().start();
            let mut color = self.brush_color;
            color.a = brush_opacity;

            // Convert global coordinates to local canvas coordinates
            let local_pos = if let Some(bounds) = self.canvas_bounds {
                Point::new(
                    event.position.x - bounds.origin.x,
                    event.position.y - bounds.origin.y,
                )
            } else {
                event.position
            };

            self.current_stroke = Some(Stroke {
                points: vec![local_pos],
                color,
                size: brush_size,
            });
            cx.notify();
        }
    }

    fn handle_mouse_move(
        &mut self,
        event: &MouseMoveEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.is_drawing {
            if let Some(ref mut stroke) = self.current_stroke {
                // Convert global coordinates to local canvas coordinates
                let local_pos = if let Some(bounds) = self.canvas_bounds {
                    Point::new(
                        event.position.x - bounds.origin.x,
                        event.position.y - bounds.origin.y,
                    )
                } else {
                    event.position
                };

                // Optimization: Only add point if it's far enough from the last point
                // This reduces the number of points and improves performance
                let should_add = if let Some(last) = stroke.points.last() {
                    let dx_px = local_pos.x - last.x;
                    let dy_px = local_pos.y - last.y;
                    // Check Manhattan distance (simpler, no multiplication needed)
                    let dx_abs = if dx_px < px(0.0) {
                        px(0.0) - dx_px
                    } else {
                        dx_px
                    };
                    let dy_abs = if dy_px < px(0.0) {
                        px(0.0) - dy_px
                    } else {
                        dy_px
                    };
                    dx_abs >= px(1.0) || dy_abs >= px(1.0) // At least 1 pixel apart in any direction
                } else {
                    true
                };

                if should_add {
                    stroke.points.push(local_pos);
                    cx.notify();
                }
            }
        }
    }

    fn handle_mouse_up(&mut self, _event: &MouseUpEvent, _: &mut Window, cx: &mut Context<Self>) {
        if self.is_drawing {
            self.is_drawing = false;
            if let Some(stroke) = self.current_stroke.take() {
                if stroke.points.len() > 1 {
                    // Create new Rc with updated strokes
                    let mut new_strokes = (*self.strokes).clone();
                    new_strokes.push(stroke);
                    self.strokes = Rc::new(new_strokes);
                }
            }
            cx.notify();
        }
    }

    fn clear_canvas(&mut self, cx: &mut Context<Self>) {
        self.strokes = Rc::new(vec![]);
        self.current_stroke = None;
        self.is_drawing = false;
        cx.notify();
    }

    fn set_brush_color(&mut self, color: Hsla, cx: &mut Context<Self>) {
        self.brush_color = color;
        cx.notify();
    }
}

impl Focusable for BrushStory {
    fn focus_handle(&self, _: &gpui::App) -> gpui::FocusHandle {
        self.focus_handle.clone()
    }
}

// Use gpui::canvas for rendering

impl BrushStory {
    fn color_button(&self, color: Hsla, _label: &str, cx: &Context<Self>) -> impl IntoElement {
        let is_selected = self.brush_color.to_hex() == color.to_hex();
        let theme = cx.theme();

        gpui::div()
            .w(px(40.))
            .h(px(40.))
            .rounded(theme.radius)
            .bg(color)
            .border_2()
            .when(is_selected, |this| {
                this.border_color(theme.primary).shadow_md()
            })
            .when(!is_selected, |this| this.border_color(theme.border))
            .cursor_pointer()
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, _, _, cx| {
                    this.set_brush_color(color, cx);
                }),
            )
    }

    fn render_canvas(&mut self, cx: &Context<Self>) -> impl IntoElement {
        let theme = cx.theme();

        // Clone Rc (cheap - just increments ref count) and other data
        let strokes_for_prepaint = self.strokes.clone();
        let current_stroke_for_prepaint = self.current_stroke.clone();
        let show_grid_for_prepaint = self.show_grid;
        let canvas_size_for_prepaint = self.canvas_size;
        let theme_for_prepaint = theme.clone();

        // Clone the state entity to capture bounds
        let state_entity = cx.entity().clone();

        let base_div = gpui::div()
            .id("canvas")
            .w(px(self.canvas_size.0))
            .h(px(self.canvas_size.1))
            .bg(theme.background)
            .border_1()
            .border_color(theme.border)
            .rounded(theme.radius)
            .cursor_pointer()
            .relative()
            .on_mouse_down(MouseButton::Left, cx.listener(Self::handle_mouse_down))
            .on_mouse_move(cx.listener(Self::handle_mouse_move))
            .on_mouse_up(MouseButton::Left, cx.listener(Self::handle_mouse_up))
            // Use an invisible canvas to capture bounds
            .child(
                gpui::canvas(
                    move |bounds, _, cx| {
                        state_entity.update(cx, |state, _| {
                            state.canvas_bounds = Some(bounds);
                        })
                    },
                    |_, _, _, _| {},
                )
                .absolute()
                .size_full(),
            );

        base_div.child(
            gpui::canvas(
                move |bounds, _window, _cx| {
                    // Prepare data for painting - all data is owned
                    // Note: we return bounds here to capture it in paint closure
                    (
                        strokes_for_prepaint,
                        current_stroke_for_prepaint,
                        show_grid_for_prepaint,
                        canvas_size_for_prepaint,
                        theme_for_prepaint,
                        bounds,
                    )
                },
                move |_bounds,
                      (strokes, current_stroke, show_grid, canvas_size, theme, prepaint_bounds),
                      window,
                      _cx| {
                    let origin = prepaint_bounds.origin;

                    // Draw grid if enabled (optimized with larger grid size)
                    if show_grid {
                        let grid_color = theme.border.opacity(0.2);
                        let grid_size = 40.0; // Larger grid = fewer lines = better performance

                        // Vertical lines
                        let mut x = 0.0;
                        while x <= canvas_size.0 {
                            let mut builder = PathBuilder::stroke(px(1.0));
                            builder.move_to(Point::new(origin.x + px(x), origin.y));
                            builder.line_to(Point::new(
                                origin.x + px(x),
                                origin.y + px(canvas_size.1),
                            ));
                            if let Ok(path) = builder.build() {
                                window.paint_path(path, grid_color);
                            }
                            x += grid_size;
                        }

                        // Horizontal lines
                        let mut y = 0.0;
                        while y <= canvas_size.1 {
                            let mut builder = PathBuilder::stroke(px(1.0));
                            builder.move_to(Point::new(origin.x, origin.y + px(y)));
                            builder.line_to(Point::new(
                                origin.x + px(canvas_size.0),
                                origin.y + px(y),
                            ));
                            if let Ok(path) = builder.build() {
                                window.paint_path(path, grid_color);
                            }
                            y += grid_size;
                        }
                    }

                    // Draw completed strokes
                    for stroke in strokes.iter() {
                        if let Some(path) = BrushStory::build_stroke_path(stroke, &prepaint_bounds)
                        {
                            window.paint_path(path, stroke.color);
                        }
                    }

                    // Draw current stroke being drawn
                    if let Some(ref stroke) = current_stroke {
                        if let Some(path) = BrushStory::build_stroke_path(stroke, &prepaint_bounds)
                        {
                            window.paint_path(path, stroke.color);
                        }
                    }
                },
            )
            .w(px(self.canvas_size.0))
            .h(px(self.canvas_size.1)),
        )
    }

    fn build_stroke_path(stroke: &Stroke, bounds: &Bounds<Pixels>) -> Option<gpui::Path<Pixels>> {
        if stroke.points.len() < 2 {
            return None;
        }

        let mut builder = PathBuilder::stroke(px(stroke.size));

        // First point - convert local coordinates to absolute by adding canvas origin
        let first_point = Point::new(
            bounds.origin.x + stroke.points[0].x,
            bounds.origin.y + stroke.points[0].y,
        );
        builder.move_to(first_point);

        // Rest of the points - also convert to absolute coordinates
        for point in stroke.points.iter().skip(1) {
            let abs_point = Point::new(bounds.origin.x + point.x, bounds.origin.y + point.y);
            builder.line_to(abs_point);
        }

        builder.build().ok()
    }
}

impl Render for BrushStory {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let brush_size = self.brush_size.read(cx).value().start();
        let brush_opacity = self.brush_opacity.read(cx).value().start();

        v_flex()
            .gap_6()
            .child(
                section("Brush Controls")
                    .max_w_2xl()
                    .child(
                        h_flex()
                            .gap_4()
                            .items_center()
                            .child("Size:")
                            .child(
                                Slider::new(&self.brush_size)
                                    .w(px(200.))
                                    .bg(theme.primary)
                                    .text_color(theme.primary_foreground),
                            )
                            .child(format!("{:.0}px", brush_size)),
                    )
                    .child(
                        h_flex()
                            .gap_4()
                            .items_center()
                            .child("Opacity:")
                            .child(
                                Slider::new(&self.brush_opacity)
                                    .w(px(200.))
                                    .bg(theme.primary)
                                    .text_color(theme.primary_foreground),
                            )
                            .child(format!("{:.0}%", brush_opacity * 100.0)),
                    ),
            )
            .child(
                section("Color Palette").max_w_2xl().child(
                    h_flex()
                        .gap_3()
                        .flex_wrap()
                        .child(self.color_button(gpui::black(), "Black", cx))
                        .child(self.color_button(gpui::white(), "White", cx))
                        .child(self.color_button(gpui::red(), "Red", cx))
                        .child(self.color_button(gpui::green(), "Green", cx))
                        .child(self.color_button(gpui::blue(), "Blue", cx))
                        .child(self.color_button(gpui::yellow(), "Yellow", cx))
                        .child(self.color_button(gpui::hsla(0.58, 1.0, 0.5, 1.0), "Purple", cx))
                        .child(self.color_button(gpui::hsla(0.083, 1.0, 0.5, 1.0), "Orange", cx)),
                ),
            )
            .child(
                section("Drawing Canvas")
                    .max_w_2xl()
                    .child(
                        h_flex()
                            .gap_3()
                            .mb_2()
                            .child(
                                Button::new("clear-canvas")
                                    .icon(IconName::Close)
                                    .label("Clear Canvas")
                                    .small()
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.clear_canvas(cx);
                                    })),
                            )
                            .child(
                                Checkbox::new("show-grid")
                                    .label("Show Grid")
                                    .checked(self.show_grid)
                                    .on_click(cx.listener(|this, checked, _, cx| {
                                        this.show_grid = *checked;
                                        cx.notify();
                                    })),
                            ),
                    )
                    .child(self.render_canvas(cx)),
            )
            .child(
                section("Instructions").max_w_2xl().child(
                    v_flex()
                        .items_start()
                        .gap_2()
                        .child("• Click and drag on the canvas to draw")
                        .child("• Adjust brush size and opacity using the sliders")
                        .child("• Select different colors from the palette")
                        .child("• Use 'Clear Canvas' to reset the drawing"),
                ),
            )
    }
}
