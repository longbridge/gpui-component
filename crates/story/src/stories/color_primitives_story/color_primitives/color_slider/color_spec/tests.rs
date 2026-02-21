use super::*;

macro_rules! assert_approx_eq {
    ($a:expr, $b:expr) => {
        assert!(
            ($a - $b).abs() < 1e-4,
            "assertion failed: `(left == right)` (left: `{:?}`, right: `{:?}`)",
            $a,
            $b
        );
    };
}

#[test]
fn test_hsl_spec() {
    let mut hsl = Hsl {
        h: 180.0,
        s: 0.5,
        l: 0.5,
        a: 1.0,
    };

    // Test name
    assert_eq!(hsl.name(), "HSL");

    // Test get_value
    assert_approx_eq!(hsl.get_value(Hsl::HUE), 180.0);
    assert_approx_eq!(hsl.get_value(Hsl::SATURATION), 0.5);
    assert_approx_eq!(hsl.get_value(Hsl::LIGHTNESS), 0.5);
    assert_approx_eq!(hsl.get_value(Hsl::ALPHA), 1.0);

    // Test set_value with clamping
    hsl.set_value(Hsl::HUE, 400.0);
    assert_approx_eq!(hsl.h, 360.0);
    hsl.set_value(Hsl::SATURATION, -0.5);
    assert_approx_eq!(hsl.s, 0.0);

    // Test conversion round-trip
    let original = Hsl {
        h: 120.0,
        s: 0.8,
        l: 0.4,
        a: 0.9,
    };
    let hsla = original.to_hsla();
    let rounded = Hsl::from_hsla(hsla);
    assert_approx_eq!(original.h, rounded.h);
    assert_approx_eq!(original.s, rounded.s);
    assert_approx_eq!(original.l, rounded.l);
    assert_approx_eq!(original.a, rounded.a);
}

#[test]
fn test_hsv_spec() {
    let mut hsv = Hsv {
        h: 240.0,
        s: 0.6,
        v: 0.7,
        a: 1.0,
    };

    // Test name
    assert_eq!(hsv.name(), "HSV");

    // Test get_value
    assert_approx_eq!(hsv.get_value(Hsv::HUE), 240.0);
    assert_approx_eq!(hsv.get_value(Hsv::SATURATION), 0.6);
    assert_approx_eq!(hsv.get_value(Hsv::VALUE), 0.7);

    // Test set_value with clamping
    hsv.set_value(Hsv::HUE, -10.0);
    assert_approx_eq!(hsv.h, 0.0);
    hsv.set_value(Hsv::SATURATION, 1.5);
    assert_approx_eq!(hsv.s, 1.0);

    // Test conversion round-trip
    let original = Hsv {
        h: 30.0,
        s: 0.9,
        v: 0.8,
        a: 1.0,
    };
    let hsla = original.to_hsla();
    let rounded = Hsv::from_hsla(hsla);
    assert_approx_eq!(original.h, rounded.h);
    assert_approx_eq!(original.s, rounded.s);
    assert_approx_eq!(original.v, rounded.v);
    assert_approx_eq!(original.a, rounded.a);
}

#[test]
fn test_hsl_uses_lightness_extremes_not_luminance() {
    // This test intentionally calls out a common misconception:
    // HSL's "L" is lightness, and valid range includes 0.0.
    // At L=0 the result is black, and at L=1 the result is white.
    let black = Hsl {
        h: 210.0,
        s: 1.0,
        l: 0.0,
        a: 1.0,
    }
    .to_hsla()
    .to_rgb();
    assert_approx_eq!(black.r, 0.0);
    assert_approx_eq!(black.g, 0.0);
    assert_approx_eq!(black.b, 0.0);

    let white = Hsl {
        h: 25.0,
        s: 1.0,
        l: 1.0,
        a: 1.0,
    }
    .to_hsla()
    .to_rgb();
    assert_approx_eq!(white.r, 1.0);
    assert_approx_eq!(white.g, 1.0);
    assert_approx_eq!(white.b, 1.0);
}

#[test]
fn test_hsl_channels_are_continuous_and_lightness_can_be_zero_percent() {
    // Another misconception this test guards against:
    // hue/saturation/lightness are continuous values, not integer-only.
    let mut hsl = Hsl {
        h: 0.0,
        s: 0.0,
        l: 0.5,
        a: 1.0,
    };

    hsl.set_value(Hsl::HUE, 42.5);
    hsl.set_value(Hsl::SATURATION, 0.333);
    hsl.set_value(Hsl::LIGHTNESS, 0.0);

    assert_approx_eq!(hsl.h, 42.5);
    assert_approx_eq!(hsl.s, 0.333);
    assert_approx_eq!(hsl.l, 0.0);
    assert_eq!(hsl.format_value(Hsl::LIGHTNESS), "0.000");
}

#[test]
fn test_rgba_spec() {
    let mut rgba = RgbaSpec {
        r: 255.0,
        g: 128.0,
        b: 0.0,
        a: 1.0,
    };

    // Test name
    assert_eq!(rgba.name(), "RGBA");

    // Test get_value
    assert_approx_eq!(rgba.get_value(RgbaSpec::RED), 255.0);
    assert_approx_eq!(rgba.get_value(RgbaSpec::GREEN), 128.0);
    assert_approx_eq!(rgba.get_value(RgbaSpec::BLUE), 0.0);

    // Test set_value with clamping
    rgba.set_value(RgbaSpec::RED, 300.0);
    assert_approx_eq!(rgba.r, 255.0);
    rgba.set_value(RgbaSpec::GREEN, -10.0);
    assert_approx_eq!(rgba.g, 0.0);

    // Test conversion round-trip
    let original = RgbaSpec {
        r: 100.0,
        g: 200.0,
        b: 50.0,
        a: 0.5,
    };
    let hsla = original.to_hsla();
    let rounded = RgbaSpec::from_hsla(hsla);
    assert_approx_eq!(original.r, rounded.r);
    assert_approx_eq!(original.g, rounded.g);
    assert_approx_eq!(original.b, rounded.b);
    assert_approx_eq!(original.a, rounded.a);
}

#[test]
fn test_lab_spec() {
    let mut lab = Lab {
        l: 50.0,
        a: 10.0,
        b: -20.0,
        alpha: 1.0,
        auto_clamp: false,
        dynamic_range: false,
    };

    // Test name
    assert_eq!(lab.name(), "Lab");

    // Test get_value
    assert_approx_eq!(lab.get_value(Lab::LIGHTNESS), 50.0);
    assert_approx_eq!(lab.get_value(Lab::A), 10.0);
    assert_approx_eq!(lab.get_value(Lab::B), -20.0);

    // Test set_value with clamping
    lab.set_value(Lab::LIGHTNESS, 150.0);
    assert_approx_eq!(lab.l, 100.0);
    lab.set_value(Lab::A, -200.0);
    assert_approx_eq!(lab.a, -128.0);

    // Test conversion round-trip
    // Lab is complex and lossy due to RGB clamping, so we expect some drift.
    // We test a known color that is likely within sRGB.
    let original = Lab {
        l: 50.0,
        a: 0.0,
        b: 0.0,
        alpha: 1.0,
        auto_clamp: false,
        dynamic_range: false,
    };
    let hsla = original.to_hsla();
    let rounded = Lab::from_hsla(hsla);
    assert!((original.l - rounded.l).abs() < 0.1);
    assert!((original.a - rounded.a).abs() < 0.1);
    assert!((original.b - rounded.b).abs() < 0.1);

    // A more vibrant color might drift more
    let vibrant = Lab {
        l: 75.0,
        a: 20.0,
        b: 20.0,
        alpha: 1.0,
        auto_clamp: false,
        dynamic_range: false,
    };
    let hsla = vibrant.to_hsla();
    let rounded = Lab::from_hsla(hsla);
    assert!((vibrant.l - rounded.l).abs() < 1.0);
    assert!((vibrant.a - rounded.a).abs() < 2.0);
    assert!((vibrant.b - rounded.b).abs() < 2.0);
}

#[test]
fn test_interpolate_rgb_midpoint() {
    let start = Hsla {
        h: 0.0,
        s: 0.0,
        l: 0.0,
        a: 0.0,
    };
    let end = Hsla {
        h: 0.0,
        s: 0.0,
        l: 1.0,
        a: 1.0,
    };
    let mid = interpolate_rgb(start, end, 0.5).to_rgb();
    assert_approx_eq!(mid.r, 0.5);
    assert_approx_eq!(mid.g, 0.5);
    assert_approx_eq!(mid.b, 0.5);
    assert_approx_eq!(mid.a, 0.5);
}

#[test]
fn test_interpolate_hsl_wraparound_takes_shortest_arc() {
    let start = gpui::hsla(350.0 / 360.0, 1.0, 0.5, 1.0);
    let end = gpui::hsla(10.0 / 360.0, 1.0, 0.5, 1.0);
    let mid = interpolate_hsl(start, end, 0.5);
    assert_approx_eq!(mid.h, 0.0);
    assert_approx_eq!(mid.s, 1.0);
    assert_approx_eq!(mid.l, 0.5);
    assert_approx_eq!(mid.a, 1.0);
}

#[test]
fn test_interpolate_lab_endpoints_are_stable() {
    let start = gpui::hsla(0.2, 0.7, 0.4, 0.3);
    let end = gpui::hsla(0.8, 0.5, 0.6, 0.9);
    let at_start = interpolate_lab(start, end, 0.0).to_rgb();
    let at_end = interpolate_lab(start, end, 1.0).to_rgb();
    let start_rgb = start.to_rgb();
    let end_rgb = end.to_rgb();

    assert!((at_start.r - start_rgb.r).abs() < 0.03);
    assert!((at_start.g - start_rgb.g).abs() < 0.03);
    assert!((at_start.b - start_rgb.b).abs() < 0.03);
    assert!((at_end.r - end_rgb.r).abs() < 0.03);
    assert!((at_end.g - end_rgb.g).abs() < 0.03);
    assert!((at_end.b - end_rgb.b).abs() < 0.03);
    assert_approx_eq!(at_start.a, 0.3);
    assert_approx_eq!(at_end.a, 0.9);
}

#[test]
fn test_hsv_from_rgba_handles_grayscale() {
    let hsv = Hsv::from_rgba(Rgba {
        r: 0.3,
        g: 0.3,
        b: 0.3,
        a: 0.8,
    });
    assert_approx_eq!(hsv.h, 0.0);
    assert_approx_eq!(hsv.s, 0.0);
    assert_approx_eq!(hsv.v, 0.3);
    assert_approx_eq!(hsv.a, 0.8);
}

#[test]
fn test_rgb_pivot_round_trip() {
    for sample in [0.0, 0.02, 0.1, 0.5, 0.9, 1.0] {
        let linear = pivot_rgb(sample);
        let srgb = rev_pivot_rgb(linear);
        assert!((srgb - sample).abs() < 1e-5);
    }
}

#[test]
fn test_hsv_value_zero_produces_black_regardless_of_hue_or_saturation() {
    let black_a = Hsv {
        h: 20.0,
        s: 0.2,
        v: 0.0,
        a: 1.0,
    }
    .to_hsla_ext()
    .to_rgb();
    let black_b = Hsv {
        h: 310.0,
        s: 1.0,
        v: 0.0,
        a: 0.6,
    }
    .to_hsla_ext()
    .to_rgb();

    assert_approx_eq!(black_a.r, 0.0);
    assert_approx_eq!(black_a.g, 0.0);
    assert_approx_eq!(black_a.b, 0.0);
    assert_approx_eq!(black_b.r, 0.0);
    assert_approx_eq!(black_b.g, 0.0);
    assert_approx_eq!(black_b.b, 0.0);
}

#[test]
fn test_hsv_zero_saturation_is_grayscale_and_hue_irrelevant() {
    let hsv_a = Hsv {
        h: 10.0,
        s: 0.0,
        v: 0.35,
        a: 1.0,
    };
    let hsv_b = Hsv {
        h: 250.0,
        s: 0.0,
        v: 0.35,
        a: 1.0,
    };

    let rgb_a = hsv_a.to_hsla_ext().to_rgb();
    let rgb_b = hsv_b.to_hsla_ext().to_rgb();
    assert_approx_eq!(rgb_a.r, rgb_a.g);
    assert_approx_eq!(rgb_a.g, rgb_a.b);
    assert_approx_eq!(rgb_a.r, 0.35);
    assert_approx_eq!(rgb_b.r, rgb_b.g);
    assert_approx_eq!(rgb_b.g, rgb_b.b);
    assert_approx_eq!(rgb_b.r, 0.35);
    assert_approx_eq!(rgb_a.r, rgb_b.r);
}

#[test]
fn test_hsl_zero_saturation_is_grayscale_and_hue_irrelevant() {
    let rgb_a = Hsl {
        h: 15.0,
        s: 0.0,
        l: 0.42,
        a: 1.0,
    }
    .to_hsla()
    .to_rgb();
    let rgb_b = Hsl {
        h: 215.0,
        s: 0.0,
        l: 0.42,
        a: 1.0,
    }
    .to_hsla()
    .to_rgb();

    assert_approx_eq!(rgb_a.r, rgb_a.g);
    assert_approx_eq!(rgb_a.g, rgb_a.b);
    assert_approx_eq!(rgb_a.r, 0.42);
    assert_approx_eq!(rgb_b.r, rgb_b.g);
    assert_approx_eq!(rgb_b.g, rgb_b.b);
    assert_approx_eq!(rgb_b.r, 0.42);
    assert_approx_eq!(rgb_a.r, rgb_b.r);
}

#[test]
fn test_hue_setters_are_clamped_not_wrapped() {
    let mut hsl = Hsl {
        h: 180.0,
        s: 0.5,
        l: 0.5,
        a: 1.0,
    };
    hsl.set_value(Hsl::HUE, -30.0);
    assert_approx_eq!(hsl.h, 0.0);
    hsl.set_value(Hsl::HUE, 390.0);
    assert_approx_eq!(hsl.h, 360.0);

    let mut hsv = Hsv {
        h: 180.0,
        s: 0.5,
        v: 0.5,
        a: 1.0,
    };
    hsv.set_value(Hsv::HUE, -20.0);
    assert_approx_eq!(hsv.h, 0.0);
    hsv.set_value(Hsv::HUE, 460.0);
    assert_approx_eq!(hsv.h, 360.0);
}

#[test]
fn test_alpha_is_preserved_across_model_conversions() {
    let hsla = gpui::hsla(0.31, 0.72, 0.42, 0.137);

    assert_approx_eq!(Hsl::from_hsla(hsla).to_hsla().a, hsla.a);
    assert_approx_eq!(Hsv::from_hsla(hsla).to_hsla().a, hsla.a);
    assert_approx_eq!(RgbaSpec::from_hsla(hsla).to_hsla().a, hsla.a);
    assert_approx_eq!(Lab::from_hsla(hsla).to_hsla().a, hsla.a);
}

#[test]
fn test_interpolation_endpoints_match_start_and_end() {
    let start = gpui::hsla(0.15, 0.35, 0.25, 0.2);
    let end = gpui::hsla(0.65, 0.85, 0.75, 0.8);

    let rgb_start = interpolate_rgb(start, end, 0.0).to_rgb();
    let rgb_end = interpolate_rgb(start, end, 1.0).to_rgb();
    let start_rgb = start.to_rgb();
    let end_rgb = end.to_rgb();
    assert_approx_eq!(rgb_start.r, start_rgb.r);
    assert_approx_eq!(rgb_start.g, start_rgb.g);
    assert_approx_eq!(rgb_start.b, start_rgb.b);
    assert_approx_eq!(rgb_start.a, start_rgb.a);
    assert_approx_eq!(rgb_end.r, end_rgb.r);
    assert_approx_eq!(rgb_end.g, end_rgb.g);
    assert_approx_eq!(rgb_end.b, end_rgb.b);
    assert_approx_eq!(rgb_end.a, end_rgb.a);

    let hsl_start = interpolate_hsl(start, end, 0.0);
    let hsl_end = interpolate_hsl(start, end, 1.0);
    assert_approx_eq!(hsl_start.h, start.h);
    assert_approx_eq!(hsl_start.s, start.s);
    assert_approx_eq!(hsl_start.l, start.l);
    assert_approx_eq!(hsl_start.a, start.a);
    assert_approx_eq!(hsl_end.h, end.h);
    assert_approx_eq!(hsl_end.s, end.s);
    assert_approx_eq!(hsl_end.l, end.l);
    assert_approx_eq!(hsl_end.a, end.a);
}

#[test]
fn test_hsl_interpolation_half_turn_tie_is_deterministic() {
    let forward = interpolate_hsl(
        gpui::hsla(0.0, 1.0, 0.5, 1.0),
        gpui::hsla(0.5, 1.0, 0.5, 1.0),
        0.5,
    );
    let reverse = interpolate_hsl(
        gpui::hsla(0.5, 1.0, 0.5, 1.0),
        gpui::hsla(0.0, 1.0, 0.5, 1.0),
        0.5,
    );
    assert_approx_eq!(forward.h, 0.25);
    assert_approx_eq!(reverse.h, 0.25);
}

#[test]
fn test_lab_to_rgb_extremes_are_finite_and_clamped() {
    let cases = [
        lab_to_rgb(0.0, -128.0, -128.0, 1.0),
        lab_to_rgb(100.0, 127.0, 127.0, 0.5),
        lab_to_rgb(-10.0, 127.0, -128.0, 0.2),
        lab_to_rgb(120.0, -128.0, 127.0, 0.9),
    ];

    for rgb in cases {
        assert!(rgb.r.is_finite());
        assert!(rgb.g.is_finite());
        assert!(rgb.b.is_finite());
        assert!((0.0..=1.0).contains(&rgb.r));
        assert!((0.0..=1.0).contains(&rgb.g));
        assert!((0.0..=1.0).contains(&rgb.b));
    }
}

#[test]
fn test_lab_checked_conversion_reports_in_gamut_value() {
    let lab = Lab {
        l: 50.0,
        a: 0.0,
        b: 0.0,
        alpha: 1.0,
        auto_clamp: false,
        dynamic_range: false,
    };
    let (hsla, out_of_gamut) = lab.to_hsla_checked();
    let rgb = hsla.to_rgb();

    assert!(!out_of_gamut);
    assert!((0.0..=1.0).contains(&rgb.r));
    assert!((0.0..=1.0).contains(&rgb.g));
    assert!((0.0..=1.0).contains(&rgb.b));
}

#[test]
fn test_lab_checked_conversion_reports_out_of_gamut_value() {
    let lab = Lab {
        l: 100.0,
        a: 127.0,
        b: 127.0,
        alpha: 1.0,
        auto_clamp: false,
        dynamic_range: false,
    };
    let (_, out_of_gamut) = lab.to_hsla_checked();
    assert!(out_of_gamut);
}

#[test]
fn test_primary_red_fixture_remains_stable_across_models() {
    let red = gpui::hsla(0.0, 1.0, 0.5, 1.0);

    let hsv = Hsv::from_hsla(red);
    assert_approx_eq!(hsv.h, 0.0);
    assert_approx_eq!(hsv.s, 1.0);
    assert_approx_eq!(hsv.v, 1.0);

    let rgba = RgbaSpec::from_hsla(red);
    assert_approx_eq!(rgba.r, 255.0);
    assert_approx_eq!(rgba.g, 0.0);
    assert_approx_eq!(rgba.b, 0.0);
    assert_approx_eq!(rgba.a, 1.0);
}
