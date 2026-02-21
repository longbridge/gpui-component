use std::f32::consts::TAU;

pub trait FieldDomain2D: 'static {
    fn contains_uv(&self, uv: (f32, f32)) -> bool;
    fn clamp_uv(&self, uv: (f32, f32)) -> (f32, f32);
    fn outline_points(&self) -> Vec<(f32, f32)>;
    fn is_rect(&self) -> bool {
        false
    }
    fn is_circle(&self) -> bool {
        false
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct RectDomain;

impl FieldDomain2D for RectDomain {
    fn contains_uv(&self, uv: (f32, f32)) -> bool {
        (0.0..=1.0).contains(&uv.0) && (0.0..=1.0).contains(&uv.1)
    }

    fn clamp_uv(&self, uv: (f32, f32)) -> (f32, f32) {
        (uv.0.clamp(0.0, 1.0), uv.1.clamp(0.0, 1.0))
    }

    fn outline_points(&self) -> Vec<(f32, f32)> {
        vec![(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)]
    }

    fn is_rect(&self) -> bool {
        true
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct CircleDomain;

impl FieldDomain2D for CircleDomain {
    fn contains_uv(&self, uv: (f32, f32)) -> bool {
        let dx = uv.0 - 0.5;
        let dy = uv.1 - 0.5;
        dx * dx + dy * dy <= 0.25 + 1e-6
    }

    fn clamp_uv(&self, uv: (f32, f32)) -> (f32, f32) {
        let uv = (uv.0.clamp(0.0, 1.0), uv.1.clamp(0.0, 1.0));
        let dx = uv.0 - 0.5;
        let dy = uv.1 - 0.5;
        let len_sq = dx * dx + dy * dy;
        if len_sq <= 0.25 {
            return uv;
        }

        let len = len_sq.sqrt();
        if len <= f32::EPSILON {
            return (0.5, 0.5);
        }

        let scale = 0.5 / len;
        (0.5 + dx * scale, 0.5 + dy * scale)
    }

    fn outline_points(&self) -> Vec<(f32, f32)> {
        let segments = 512usize;
        (0..segments)
            .map(|i| {
                let t = TAU * i as f32 / segments as f32;
                (0.5 + 0.5 * t.cos(), 0.5 - 0.5 * t.sin())
            })
            .collect()
    }

    fn is_circle(&self) -> bool {
        true
    }
}

#[derive(Clone, Copy, Debug)]
pub struct TriangleDomain {
    pub a: (f32, f32),
    pub b: (f32, f32),
    pub c: (f32, f32),
}

impl TriangleDomain {
    pub fn up() -> Self {
        Self {
            a: (0.5, 0.0),
            b: (0.0, 1.0),
            c: (1.0, 1.0),
        }
    }

    #[allow(dead_code)]
    pub fn down() -> Self {
        Self {
            a: (0.0, 0.0),
            b: (1.0, 0.0),
            c: (0.5, 1.0),
        }
    }
}

impl Default for TriangleDomain {
    fn default() -> Self {
        Self::up()
    }
}

impl FieldDomain2D for TriangleDomain {
    fn contains_uv(&self, uv: (f32, f32)) -> bool {
        point_in_triangle(uv, self.a, self.b, self.c)
    }

    fn clamp_uv(&self, uv: (f32, f32)) -> (f32, f32) {
        // Keep interaction a tiny distance off the texture edge to avoid edge-adjacent artifacts.
        const EPSILON: f32 = 0.001;
        let uv = (
            uv.0.clamp(EPSILON, 1.0 - EPSILON),
            uv.1.clamp(EPSILON, 1.0 - EPSILON),
        );
        if self.contains_uv(uv) {
            return uv;
        }

        let ab = closest_point_on_segment(uv, self.a, self.b);
        let bc = closest_point_on_segment(uv, self.b, self.c);
        let ca = closest_point_on_segment(uv, self.c, self.a);

        let dab = distance_squared(uv, ab);
        let dbc = distance_squared(uv, bc);
        let dca = distance_squared(uv, ca);

        if dab <= dbc && dab <= dca {
            ab
        } else if dbc <= dca {
            bc
        } else {
            ca
        }
    }

    fn outline_points(&self) -> Vec<(f32, f32)> {
        vec![self.a, self.b, self.c]
    }
}

#[derive(Clone, Debug)]
pub struct PolygonDomain {
    points: Vec<(f32, f32)>,
}

impl PolygonDomain {
    pub fn new(mut points: Vec<(f32, f32)>) -> Self {
        points.retain(|p| p.0.is_finite() && p.1.is_finite());
        if points.len() < 3 {
            points = vec![(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)];
        }
        Self {
            points: points
                .into_iter()
                .map(|(x, y)| (x.clamp(0.0, 1.0), y.clamp(0.0, 1.0)))
                .collect(),
        }
    }

    fn edges(&self) -> impl Iterator<Item = ((f32, f32), (f32, f32))> + '_ {
        self.points
            .iter()
            .enumerate()
            .map(|(ix, p0)| (*p0, self.points[(ix + 1) % self.points.len()]))
    }
}

impl FieldDomain2D for PolygonDomain {
    fn contains_uv(&self, uv: (f32, f32)) -> bool {
        point_in_polygon_even_odd(uv, &self.points)
    }

    fn clamp_uv(&self, uv: (f32, f32)) -> (f32, f32) {
        // Keep interaction a tiny distance off the texture edge to avoid edge-adjacent artifacts.
        const EPSILON: f32 = 0.001;
        let uv = (
            uv.0.clamp(EPSILON, 1.0 - EPSILON),
            uv.1.clamp(EPSILON, 1.0 - EPSILON),
        );
        if self.contains_uv(uv) {
            return uv;
        }

        let mut best = self.points[0];
        let mut best_dist = distance_squared(uv, best);
        for (a, b) in self.edges() {
            let candidate = closest_point_on_segment(uv, a, b);
            let dist = distance_squared(uv, candidate);
            if dist < best_dist {
                best_dist = dist;
                best = candidate;
            }
        }

        best
    }

    fn outline_points(&self) -> Vec<(f32, f32)> {
        self.points.clone()
    }
}

fn point_in_triangle(point: (f32, f32), a: (f32, f32), b: (f32, f32), c: (f32, f32)) -> bool {
    let d1 = signed_area(point, a, b);
    let d2 = signed_area(point, b, c);
    let d3 = signed_area(point, c, a);
    let has_neg = d1 < 0.0 || d2 < 0.0 || d3 < 0.0;
    let has_pos = d1 > 0.0 || d2 > 0.0 || d3 > 0.0;
    !(has_neg && has_pos)
}

fn point_in_polygon_even_odd(point: (f32, f32), polygon: &[(f32, f32)]) -> bool {
    let mut inside = false;
    let mut j = polygon.len() - 1;
    for i in 0..polygon.len() {
        let pi = polygon[i];
        let pj = polygon[j];
        let yi = pi.1;
        let yj = pj.1;
        let xi = pi.0;
        let xj = pj.0;

        let crosses = (yi > point.1) != (yj > point.1);
        if crosses {
            let t = (point.1 - yi) / (yj - yi);
            let x = xi + t * (xj - xi);
            if point.0 < x {
                inside = !inside;
            }
        }
        j = i;
    }
    inside
}

fn signed_area(p1: (f32, f32), p2: (f32, f32), p3: (f32, f32)) -> f32 {
    (p1.0 - p3.0) * (p2.1 - p3.1) - (p2.0 - p3.0) * (p1.1 - p3.1)
}

fn distance_squared(a: (f32, f32), b: (f32, f32)) -> f32 {
    let dx = a.0 - b.0;
    let dy = a.1 - b.1;
    dx * dx + dy * dy
}

fn closest_point_on_segment(point: (f32, f32), a: (f32, f32), b: (f32, f32)) -> (f32, f32) {
    let ab = (b.0 - a.0, b.1 - a.1);
    let len_sq = ab.0 * ab.0 + ab.1 * ab.1;
    if len_sq <= f32::EPSILON {
        return a;
    }

    let ap = (point.0 - a.0, point.1 - a.1);
    let t = ((ap.0 * ab.0 + ap.1 * ab.1) / len_sq).clamp(0.0, 1.0);
    (a.0 + ab.0 * t, a.1 + ab.1 * t)
}
