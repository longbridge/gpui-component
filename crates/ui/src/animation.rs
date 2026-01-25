/// A cubic bezier function like CSS `cubic-bezier`.
///
/// Builder:
///
/// https://cubic-bezier.com
pub fn cubic_bezier(x1: f32, y1: f32, x2: f32, y2: f32) -> impl Fn(f32) -> f32 {
  move |t: f32| {
    let one_t = 1.0 - t;
    let one_t2 = one_t * one_t;
    let t2 = t * t;
    let t3 = t2 * t;

    // The Bezier curve function for x and y, where x0 = 0, y0 = 0, x3 = 1, y3 = 1
    let _x = 3.0 * x1 * one_t2 * t + 3.0 * x2 * one_t * t2 + t3;
    let y = 3.0 * y1 * one_t2 * t + 3.0 * y2 * one_t * t2 + t3;

    y
  }
}
