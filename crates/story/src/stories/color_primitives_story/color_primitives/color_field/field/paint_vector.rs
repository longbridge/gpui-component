use super::super::domain::FieldDomain2D;
use super::super::model::{ColorFieldModel2D, ColorFieldModelKind};
use crate::stories::color_primitives_story::color_spec::Hsv;
use gpui::*;
use std::f32::consts::PI;

pub(super) fn paint_domain_background(
    window: &mut Window,
    bounds: Bounds<Pixels>,
    domain: &dyn FieldDomain2D,
    model: &dyn ColorFieldModel2D,
    hsv: Hsv,
    samples_per_axis: usize,
) {
    let width = bounds.size.width.as_f32();
    let height = bounds.size.height.as_f32();
    if width <= 0.0 || height <= 0.0 {
        return;
    }

    if domain.is_rect() && paint_rect_background_fast(window, bounds, model, hsv) {
        return;
    }

    if model.kind() == ColorFieldModelKind::SvAtHue {
        let points = domain.outline_points();
        if paint_sv_domain_background_fast(window, bounds, &points, hsv.h) {
            return;
        }
    }

    let target_max = samples_per_axis.max(16) as f32;
    let max_dim = width.max(height).max(1.0);
    let scale = (target_max / max_dim).clamp(0.25, 1.0);
    let cols = (width * scale).round().max(1.0) as u32;
    let rows = (height * scale).round().max(1.0) as u32;

    let origin_x = bounds.origin.x.as_f32();
    let origin_y = bounds.origin.y.as_f32();

    for y in 0..rows {
        let y0 = origin_y + height * (y as f32 / rows as f32);
        let y1 = origin_y + height * ((y + 1) as f32 / rows as f32);
        let cell_h = (y1 - y0).max(0.0);
        if cell_h <= 0.0 {
            continue;
        }

        for x in 0..cols {
            let x0 = origin_x + width * (x as f32 / cols as f32);
            let x1 = origin_x + width * ((x + 1) as f32 / cols as f32);
            let cell_w = (x1 - x0).max(0.0);
            if cell_w <= 0.0 {
                continue;
            }

            let uv = (
                ((x as f32 + 0.5) / cols as f32).clamp(0.0, 1.0),
                ((y as f32 + 0.5) / rows as f32).clamp(0.0, 1.0),
            );
            if !domain.contains_uv(uv) {
                continue;
            }

            let color = model.color_at_uv(&hsv, uv);
            window.paint_quad(PaintQuad {
                bounds: Bounds {
                    origin: point(px(x0), px(y0)),
                    size: size(px(cell_w), px(cell_h)),
                },
                corner_radii: Corners::default(),
                background: color.into(),
                border_widths: Edges::default(),
                border_color: transparent_black(),
                border_style: BorderStyle::default(),
            });
        }
    }
}

fn paint_rect_background_fast(
    window: &mut Window,
    bounds: Bounds<Pixels>,
    model: &dyn ColorFieldModel2D,
    hsv: Hsv,
) -> bool {
    match model.kind() {
        ColorFieldModelKind::SvAtHue => {
            paint_sv_rect_background(window, bounds, hsv.h);
            true
        }
        ColorFieldModelKind::HsAtValue => {
            paint_hs_rect_background(window, bounds, hsv.v);
            true
        }
        ColorFieldModelKind::HvAtSaturation => {
            paint_hv_rect_background(window, bounds, hsv.s);
            true
        }
        ColorFieldModelKind::HueSaturationLightness => {
            paint_hsl_photoshop_rect_background(window, bounds);
            true
        }
        _ => false,
    }
}

fn paint_sv_rect_background(window: &mut Window, bounds: Bounds<Pixels>, hue: f32) {
    let hue_color = hsv_color(hue, 1.0, 1.0);
    paint_quad_fill(window, bounds, hue_color.into());
    paint_quad_fill(
        window,
        bounds,
        linear_gradient(
            90.0,
            linear_color_stop(white(), 0.0),
            linear_color_stop(hsla(0.0, 0.0, 1.0, 0.0), 1.0),
        ),
    );
    paint_quad_fill(
        window,
        bounds,
        linear_gradient(
            180.0,
            linear_color_stop(hsla(0.0, 0.0, 0.0, 0.0), 0.0),
            linear_color_stop(black(), 1.0),
        ),
    );
}

fn paint_hs_rect_background(window: &mut Window, bounds: Bounds<Pixels>, value: f32) {
    let gray = hsv_color(0.0, 0.0, value);
    let transparent_gray = Hsla { a: 0.0, ..gray };

    paint_quad_fill(window, bounds, gray.into());
    paint_hue_sweep(window, bounds, 1.0, value);
    paint_quad_fill(
        window,
        bounds,
        linear_gradient(
            180.0,
            linear_color_stop(transparent_gray, 0.0),
            linear_color_stop(gray, 1.0),
        ),
    );
}

fn paint_hv_rect_background(window: &mut Window, bounds: Bounds<Pixels>, saturation: f32) {
    paint_quad_fill(window, bounds, black().into());
    paint_hue_sweep(window, bounds, saturation, 1.0);
    paint_quad_fill(
        window,
        bounds,
        linear_gradient(
            180.0,
            linear_color_stop(hsla(0.0, 0.0, 0.0, 0.0), 0.0),
            linear_color_stop(black(), 1.0),
        ),
    );
}

fn paint_hsl_photoshop_rect_background(window: &mut Window, bounds: Bounds<Pixels>) {
    let half_height = bounds.size.height / 2.0;
    let mid_y = bounds.origin.y + half_height;
    let bar_width = if bounds.size.width < px(6.0) {
        bounds.size.width
    } else {
        px(6.0)
    };
    let bar_x = bounds.origin.x + bounds.size.width - bar_width;

    paint_hue_sweep(window, bounds, 1.0, 1.0);

    paint_quad_fill(
        window,
        Bounds {
            origin: bounds.origin,
            size: size(bounds.size.width, half_height),
        },
        linear_gradient(
            180.0,
            linear_color_stop(white(), 0.0),
            linear_color_stop(hsla(0.0, 0.0, 1.0, 0.0), 1.0),
        ),
    );

    paint_quad_fill(
        window,
        Bounds {
            origin: point(bounds.origin.x, mid_y),
            size: size(bounds.size.width, bounds.size.height - half_height),
        },
        linear_gradient(
            180.0,
            linear_color_stop(hsla(0.0, 0.0, 0.0, 0.0), 0.0),
            linear_color_stop(black(), 1.0),
        ),
    );

    paint_quad_fill(
        window,
        Bounds {
            origin: point(bar_x, bounds.origin.y),
            size: size(bar_width, half_height),
        },
        white().into(),
    );

    paint_quad_fill(
        window,
        Bounds {
            origin: point(bar_x, mid_y),
            size: size(bar_width, bounds.size.height - half_height),
        },
        black().into(),
    );
}

fn paint_hue_sweep(window: &mut Window, bounds: Bounds<Pixels>, saturation: f32, value: f32) {
    const HUE_BAND_COUNT: usize = 36;
    let band_width = bounds.size.width / HUE_BAND_COUNT as f32;

    for i in 0..HUE_BAND_COUNT {
        let start_hue = (i as f32 / HUE_BAND_COUNT as f32) * 360.0;
        let end_hue = ((i + 1) as f32 / HUE_BAND_COUNT as f32) * 360.0;
        let start_x = bounds.origin.x + band_width * i as f32;
        let end_x = if i == HUE_BAND_COUNT - 1 {
            bounds.origin.x + bounds.size.width
        } else {
            bounds.origin.x + band_width * (i + 1) as f32 + px(0.5)
        };

        paint_quad_fill(
            window,
            Bounds {
                origin: point(start_x, bounds.origin.y),
                size: size(end_x - start_x, bounds.size.height),
            },
            linear_gradient(
                90.0,
                linear_color_stop(hsv_color(start_hue, saturation, value), 0.0),
                linear_color_stop(hsv_color(end_hue, saturation, value), 1.0),
            ),
        );
    }
}

fn hsv_color(hue: f32, saturation: f32, value: f32) -> Hsla {
    Hsv {
        h: hue,
        s: saturation,
        v: value,
        a: 1.0,
    }
    .to_hsla_ext()
}

fn paint_quad_fill(window: &mut Window, bounds: Bounds<Pixels>, background: Background) {
    window.paint_quad(PaintQuad {
        bounds,
        corner_radii: Corners::default(),
        background,
        border_widths: Edges::default(),
        border_color: transparent_black(),
        border_style: BorderStyle::default(),
    });
}

fn paint_sv_domain_background_fast(
    window: &mut Window,
    bounds: Bounds<Pixels>,
    points: &[(f32, f32)],
    hue: f32,
) -> bool {
    if points.len() < 3 {
        return false;
    }

    let width = bounds.size.width.as_f32();
    let height = bounds.size.height.as_f32();
    if width <= 0.0 || height <= 0.0 {
        return false;
    }

    let origin_x = bounds.origin.x.as_f32();
    let origin_y = bounds.origin.y.as_f32();
    let rows = height.round().max(1.0) as u32;
    let mut intersections = Vec::with_capacity(points.len());

    for y in 0..rows {
        intersections.clear();

        let v0 = y as f32 / rows as f32;
        let v1 = (y + 1) as f32 / rows as f32;
        let v = (v0 + v1) * 0.5;
        let row_y0 = origin_y + height * v0;
        let row_y1 = origin_y + height * v1;
        let row_h = (row_y1 - row_y0).max(0.0);
        if row_h <= 0.0 {
            continue;
        }

        let mut prev = points[points.len() - 1];
        for &curr in points {
            let (x0, y0) = prev;
            let (x1, y1) = curr;
            let crosses = (y0 > v) != (y1 > v);
            if crosses {
                let dy = y1 - y0;
                if dy.abs() > f32::EPSILON {
                    let t = (v - y0) / dy;
                    intersections.push(x0 + t * (x1 - x0));
                }
            }
            prev = curr;
        }

        if intersections.len() < 2 {
            continue;
        }
        intersections.sort_by(|a, b| a.total_cmp(b));

        let value = (1.0 - v).clamp(0.0, 1.0);
        for segment in intersections.chunks_exact(2) {
            let u_start = segment[0].clamp(0.0, 1.0);
            let u_end = segment[1].clamp(0.0, 1.0);
            if u_end <= u_start {
                continue;
            }

            let x0 = origin_x + width * u_start;
            let x1 = origin_x + width * u_end;
            let row_w = (x1 - x0).max(0.0);
            if row_w <= 0.0 {
                continue;
            }

            let left = hsv_color(hue, u_start, value);
            let right = hsv_color(hue, u_end, value);
            paint_quad_fill(
                window,
                Bounds {
                    origin: point(px(x0), px(row_y0)),
                    size: size(px(row_w), px(row_h)),
                },
                linear_gradient(
                    90.0,
                    linear_color_stop(left, 0.0),
                    linear_color_stop(right, 1.0),
                ),
            );
        }
    }

    true
}

pub(super) fn paint_domain_border(
    window: &mut Window,
    bounds: Bounds<Pixels>,
    domain: &dyn FieldDomain2D,
    color: Hsla,
) {
    let points = domain.outline_points();
    if points.len() < 2 {
        return;
    }

    let mut builder = PathBuilder::stroke(px(1.0));
    let first = normalized_to_point(bounds, points[0]);
    builder.move_to(point(px(first.0), px(first.1)));
    for point_uv in points.iter().skip(1) {
        let p = normalized_to_point(bounds, *point_uv);
        builder.line_to(point(px(p.0), px(p.1)));
    }
    builder.line_to(point(px(first.0), px(first.1)));
    if let Ok(path) = builder.build() {
        window.paint_path(path, color);
    }
}

fn normalized_to_point(bounds: Bounds<Pixels>, uv: (f32, f32)) -> (f32, f32) {
    (
        bounds.origin.x.as_f32() + bounds.size.width.as_f32() * uv.0,
        bounds.origin.y.as_f32() + bounds.size.height.as_f32() * uv.1,
    )
}

pub(super) fn has_non_zero_corner_radius(radii: Corners<Pixels>) -> bool {
    radii.top_left > px(0.0)
        || radii.top_right > px(0.0)
        || radii.bottom_left > px(0.0)
        || radii.bottom_right > px(0.0)
}

fn append_arc_points(
    builder: &mut PathBuilder,
    center_x: f32,
    center_y: f32,
    radius: f32,
    start_angle: f32,
    end_angle: f32,
    steps: usize,
) {
    for i in 1..=steps {
        let t = i as f32 / steps as f32;
        let angle = start_angle + (end_angle - start_angle) * t;
        let x = center_x + radius * angle.cos();
        let y = center_y + radius * angle.sin();
        builder.line_to(point(px(x), px(y)));
    }
}

pub(super) fn paint_corner_occluder(
    window: &mut Window,
    bounds: Bounds<Pixels>,
    radii: Corners<Pixels>,
    color: Hsla,
) {
    let left = bounds.origin.x.as_f32();
    let top = bounds.origin.y.as_f32();
    let right = (bounds.origin.x + bounds.size.width).as_f32();
    let bottom = (bounds.origin.y + bounds.size.height).as_f32();
    let max_radius = (bounds.size.width.min(bounds.size.height).as_f32() / 2.0).max(0.0);
    let steps = 48;

    let top_left = radii.top_left.as_f32().min(max_radius);
    if top_left > 0.0 {
        let mut builder = PathBuilder::fill();
        builder.move_to(point(px(left), px(top)));
        builder.line_to(point(px(left + top_left), px(top)));
        append_arc_points(
            &mut builder,
            left + top_left,
            top + top_left,
            top_left,
            -PI / 2.0,
            -PI,
            steps,
        );
        builder.close();
        if let Ok(path) = builder.build() {
            window.paint_path(path, color);
        }
    }

    let top_right = radii.top_right.as_f32().min(max_radius);
    if top_right > 0.0 {
        let mut builder = PathBuilder::fill();
        builder.move_to(point(px(right), px(top)));
        builder.line_to(point(px(right - top_right), px(top)));
        append_arc_points(
            &mut builder,
            right - top_right,
            top + top_right,
            top_right,
            -PI / 2.0,
            0.0,
            steps,
        );
        builder.close();
        if let Ok(path) = builder.build() {
            window.paint_path(path, color);
        }
    }

    let bottom_left = radii.bottom_left.as_f32().min(max_radius);
    if bottom_left > 0.0 {
        let mut builder = PathBuilder::fill();
        builder.move_to(point(px(left), px(bottom)));
        builder.line_to(point(px(left), px(bottom - bottom_left)));
        append_arc_points(
            &mut builder,
            left + bottom_left,
            bottom - bottom_left,
            bottom_left,
            PI,
            PI / 2.0,
            steps,
        );
        builder.close();
        if let Ok(path) = builder.build() {
            window.paint_path(path, color);
        }
    }

    let bottom_right = radii.bottom_right.as_f32().min(max_radius);
    if bottom_right > 0.0 {
        let mut builder = PathBuilder::fill();
        builder.move_to(point(px(right), px(bottom)));
        builder.line_to(point(px(right - bottom_right), px(bottom)));
        append_arc_points(
            &mut builder,
            right - bottom_right,
            bottom - bottom_right,
            bottom_right,
            PI / 2.0,
            0.0,
            steps,
        );
        builder.close();
        if let Ok(path) = builder.build() {
            window.paint_path(path, color);
        }
    }
}
