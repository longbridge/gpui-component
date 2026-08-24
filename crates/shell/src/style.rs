//! The styling engine behind the the script fluent chain.
//!
//! the script writes exactly what Rust writes — `v_flex():size_full():bg("surface"):p(12)`
//! — so this module has to answer one question for any method name a script
//! calls: is it a style method, and if so how is it applied to a
//! [`StyleRefinement`]?
//!
//! There are two answers, and they exist for different reasons:
//!
//! * **No-argument methods** come from GPUI's inspector reflection
//!   (`gpui_base::styled_ext_reflection_methods` and
//!   `gpui::styled_reflection::methods`). `FunctionReflection::invoke` only
//!   takes a receiver, so reflection covers precisely the `fn(self) -> Self`
//!   style methods — hundreds of names (`flex_col`, `items_center`, `gap_2`,
//!   `rounded_md`, `text_sm`, `size_full`, …) obtained with zero maintenance.
//!   When upstream GPUI adds one, the script gets it for free. They are addressed by a
//!   `u16` index so the spec arena can record a style call in two bytes instead
//!   of a string.
//! * **Methods that take arguments** cannot be reflected and are bound by hand
//!   in [`apply_param`]. This is the only hand-maintained list in the module,
//!   and it is deliberately small: about forty names.
//!
//! Both halves feed [`suggest`], because a mistyped style name must be visible
//! at the call site rather than as a silently ignored no-op — see §13.2 of
//! `docs/gpui-shell.md`.
//!
//! # Availability
//!
//! Reflection lives behind `#[cfg(any(feature = "inspector", debug_assertions))]`
//! in both `gpui-base` and `gpui`. `crates/shell` enables `gpui-base/inspector`
//! (which forwards to `gpui/inspector`), so the table is populated in release
//! builds too. [`tests::the_reflection_table_is_populated`] is the assertion
//! that keeps it that way; run it with `--release` in CI.
//!
//! # Storage
//!
//! `FunctionReflection<StyleRefinement>` is `Copy` and holds only a `&'static
//! str`, a plain `fn` pointer and a `PhantomData`, so it is `Send + Sync` and a
//! `static OnceLock` works — no thread-local fallback is needed. That matters
//! because `nullary_name` is called from `SpecArena::debug_tree`, which runs in
//! tests without a GPUI `App`.

use std::collections::HashMap;
use std::sync::OnceLock;

use crate::error::{Result as ShellResult, ShellError};
use gpui::inspector_reflection::FunctionReflection;
use gpui::{AbsoluteLength, DefiniteLength, Length, StyleRefinement, Styled, px, relative, rems};
use gpui_base::StyledExt as _;

use crate::value::{Bridged, arg};

/// Style methods that take arguments, in the order they are documented.
///
/// Hand-maintained because reflection cannot reach them. The array is also the
/// interning source: [`param_style_name`] hands back a `&'static str` from here
/// so the spec arena can store a name without allocating.
///
/// Deliberately **not** bound, and why:
///
/// * `shadow` — takes a `Vec<BoxShadow>`; the `shadow_*` presets are nullary and
///   already reflected, and a real shadow API belongs with the animation and
///   token work in §13.5 rather than as a positional argument list.
/// * `cursor`, `text_align`, `text_overflow`, `font_weight` — take GPUI enums.
///   They need an enum-name mapping of their own; every variant already has a
///   nullary form (`cursor_pointer`, `text_center`, `font_bold`, …).
/// * `scrollbar_width` — meaningful only together with overflow configuration
///   that the shell does not expose yet.
///
/// `text_bg`, `min_size` and `max_size` are bound even though the design doc
/// does not name them: they are the same one-line shape as their neighbours and
/// leaving them out would be an arbitrary hole in the surface.
const PARAM_STYLES: &[&str] = &[
    // Size — `Length`, so `"auto"` is accepted.
    "w",
    "h",
    "size",
    "min_w",
    "min_h",
    "min_size",
    "max_w",
    "max_h",
    "max_size",
    // Padding — `DefiniteLength`.
    "p",
    "px",
    "py",
    "pt",
    "pb",
    "pl",
    "pr",
    // Margin — `Length`.
    "m",
    "mx",
    "my",
    "mt",
    "mb",
    "ml",
    "mr",
    // Position — `Length`.
    "inset",
    "top",
    "bottom",
    "left",
    "right",
    // Flex.
    "gap",
    "gap_x",
    "gap_y",
    "flex_grow",
    "flex_shrink",
    "flex_basis",
    // Paint.
    "bg",
    "text_color",
    "text_bg",
    "text_size",
    "line_height",
    "opacity",
    // Border and radius — `AbsoluteLength`.
    "border",
    "border_t",
    "border_b",
    "border_l",
    "border_r",
    "border_x",
    "border_y",
    "border_color",
    "rounded",
    "rounded_t",
    "rounded_b",
    "rounded_l",
    "rounded_r",
    "rounded_tl",
    "rounded_tr",
    "rounded_bl",
    "rounded_br",
];

/// The reflected no-argument style methods, plus their name index.
/// No-argument style methods that reflection does not reach.
///
/// `gpui-base` generates its font-weight helpers with a macro, and the
/// reflection pass does not see macro-expanded trait methods, so the whole
/// `font_*` family would otherwise be missing from the script surface. These
/// are appended after the reflected table and addressed by the same `u16`.
type NullaryFn = fn(StyleRefinement) -> StyleRefinement;

const EXTRA_NULLARY: &[(&str, NullaryFn)] = &[
    ("font_thin", |style| style.font_thin()),
    ("font_extralight", |style| style.font_extralight()),
    ("font_light", |style| style.font_light()),
    ("font_normal", |style| style.font_normal()),
    ("font_medium", |style| style.font_medium()),
    ("font_semibold", |style| style.font_semibold()),
    ("font_bold", |style| style.font_bold()),
    ("font_extrabold", |style| style.font_extrabold()),
    ("font_black", |style| style.font_black()),
];

struct StyleTable {
    /// Indexed by the `u16` stored in `SpecOp::NullaryStyle`.
    nullary: Vec<FunctionReflection<StyleRefinement>>,
    by_name: HashMap<&'static str, u16>,
}

fn table() -> &'static StyleTable {
    static TABLE: OnceLock<StyleTable> = OnceLock::new();
    TABLE.get_or_init(|| {
        let nullary: Vec<_> = [
            gpui_base::styled_ext_reflection_methods::<StyleRefinement>(),
            gpui::styled_reflection::methods::<StyleRefinement>(),
        ]
        .into_iter()
        .flatten()
        .collect();

        // Both traits are in scope on `StyleRefinement`, so a name can appear
        // twice; the first wins, matching Rust's own inherent-before-extension
        // resolution closely enough for a diagnostic table.
        let mut by_name = HashMap::with_capacity(nullary.len() + EXTRA_NULLARY.len());
        for (index, method) in nullary.iter().enumerate() {
            by_name.entry(method.name).or_insert(index as u16);
        }
        for (offset, (name, _)) in EXTRA_NULLARY.iter().enumerate() {
            by_name
                .entry(*name)
                .or_insert((nullary.len() + offset) as u16);
        }

        StyleTable { nullary, by_name }
    })
}

/// Builds the reflection table once, so the first script call does not pay for it.
///
/// Idempotent: every accessor in this module initializes on demand anyway, and
/// `nullary_name` in particular is reached from `SpecArena::debug_tree` in tests
/// that never call [`init`].
pub fn init() {
    let _ = table();
}

/// Index of a no-argument style method, if the name is one.
///
/// The dispatcher calls this first: reflection is the larger and cheaper half of
/// the surface, and an index costs the spec arena two bytes per recorded call.
pub fn nullary_index(name: &str) -> Option<u16> {
    table().by_name.get(name).copied()
}

/// Name for an index previously returned by [`nullary_index`].
///
/// Never panics — spec debug dumps must stay printable even when handed a stale
/// index from an earlier render pass.
pub fn nullary_name(index: u16) -> &'static str {
    let table = table();
    if let Some(method) = table.nullary.get(index as usize) {
        return method.name;
    }
    EXTRA_NULLARY
        .get(index as usize - table.nullary.len())
        .map(|(name, _)| *name)
        .unwrap_or("<unknown style>")
}

/// Applies a no-argument style method.
///
/// An out-of-range index is a no-op rather than a panic, for the same reason
/// [`nullary_name`] is total.
pub fn apply_nullary(index: u16, refinement: StyleRefinement) -> StyleRefinement {
    let table = table();
    if let Some(method) = table.nullary.get(index as usize) {
        return method.invoke(refinement);
    }
    match EXTRA_NULLARY.get(index as usize - table.nullary.len()) {
        Some((_, apply)) => apply(refinement),
        None => refinement,
    }
}

/// Returns the interned name if `name` is a style method that takes arguments.
///
/// Returning `&'static str` is the point: the spec arena stores the name by
/// reference, so recording `:bg("surface")` allocates nothing for the method
/// name itself.
pub fn param_style_name(name: &str) -> Option<&'static str> {
    PARAM_STYLES
        .iter()
        .copied()
        .find(|candidate| *candidate == name)
}

/// Applies a style method that takes arguments.
///
/// Coercion is never reimplemented here: numbers become pixels and strings
/// become colors through [`Bridged`], so `:p(12)` and `:bg("#ff0000")` mean the
/// same thing everywhere. The only conversion local to this module is the
/// length grammar (`"auto"`, `"50%"`, `"12px"`, `"1rem"`), which exists because
/// `Bridged` has no length concept beyond bare pixels.
pub fn apply_param(
    name: &str,
    args: &[Bridged],
    refinement: StyleRefinement,
) -> ShellResult<StyleRefinement> {
    /// Reads argument 0 as a `Length` (`"auto"` allowed).
    macro_rules! length {
        () => {
            length(&arg(args, 0, name)?, name)?
        };
    }
    /// Reads argument 0 as a `DefiniteLength` (`"auto"` rejected).
    macro_rules! definite {
        () => {
            definite_length(&arg(args, 0, name)?, name)?
        };
    }
    /// Reads argument 0 as an `AbsoluteLength` (percentages rejected).
    macro_rules! absolute {
        () => {
            absolute_length(&arg(args, 0, name)?, name)?
        };
    }
    /// Reads argument 0 as a color, via token name or `#rrggbb`.
    macro_rules! color {
        () => {
            arg(args, 0, name)?.as_color()?
        };
    }
    /// Reads argument 0 as a bare number.
    macro_rules! number {
        () => {
            arg(args, 0, name)?.as_f32()?
        };
    }

    Ok(match name {
        "w" => refinement.w(length!()),
        "h" => refinement.h(length!()),
        "size" => refinement.size(length!()),
        "min_w" => refinement.min_w(length!()),
        "min_h" => refinement.min_h(length!()),
        "min_size" => refinement.min_size(length!()),
        "max_w" => refinement.max_w(length!()),
        "max_h" => refinement.max_h(length!()),
        "max_size" => refinement.max_size(length!()),

        "p" => refinement.p(definite!()),
        "px" => refinement.px(definite!()),
        "py" => refinement.py(definite!()),
        "pt" => refinement.pt(definite!()),
        "pb" => refinement.pb(definite!()),
        "pl" => refinement.pl(definite!()),
        "pr" => refinement.pr(definite!()),

        "m" => refinement.m(length!()),
        "mx" => refinement.mx(length!()),
        "my" => refinement.my(length!()),
        "mt" => refinement.mt(length!()),
        "mb" => refinement.mb(length!()),
        "ml" => refinement.ml(length!()),
        "mr" => refinement.mr(length!()),

        "inset" => refinement.inset(length!()),
        "top" => refinement.top(length!()),
        "bottom" => refinement.bottom(length!()),
        "left" => refinement.left(length!()),
        "right" => refinement.right(length!()),

        "gap" => refinement.gap(definite!()),
        "gap_x" => refinement.gap_x(definite!()),
        "gap_y" => refinement.gap_y(definite!()),
        "flex_grow" => refinement.flex_grow(number!()),
        "flex_shrink" => refinement.flex_shrink(number!()),
        "flex_basis" => refinement.flex_basis(length!()),

        "bg" => refinement.bg(color!()),
        "text_color" => refinement.text_color(color!()),
        "text_bg" => refinement.text_bg(color!()),
        "text_size" => refinement.text_size(absolute!()),
        // Line height is the one length whose bare number is a multiplier, not
        // pixels: `line_height(1.45)` means 1.45x the font size everywhere else
        // in the industry, and 1.45px is never what anyone meant. A string
        // (`"18px"`, `"120%"`) still goes through the ordinary grammar.
        "line_height" => refinement.line_height(line_height(&arg(args, 0, name)?, name)?),
        "opacity" => refinement.opacity(number!()),

        "border" => refinement.border(absolute!()),
        "border_t" => refinement.border_t(absolute!()),
        "border_b" => refinement.border_b(absolute!()),
        "border_l" => refinement.border_l(absolute!()),
        "border_r" => refinement.border_r(absolute!()),
        "border_x" => refinement.border_x(absolute!()),
        "border_y" => refinement.border_y(absolute!()),
        "border_color" => refinement.border_color(color!()),

        "rounded" => refinement.rounded(absolute!()),
        "rounded_t" => refinement.rounded_t(absolute!()),
        "rounded_b" => refinement.rounded_b(absolute!()),
        "rounded_l" => refinement.rounded_l(absolute!()),
        "rounded_r" => refinement.rounded_r(absolute!()),
        "rounded_tl" => refinement.rounded_tl(absolute!()),
        "rounded_tr" => refinement.rounded_tr(absolute!()),
        "rounded_bl" => refinement.rounded_bl(absolute!()),
        "rounded_br" => refinement.rounded_br(absolute!()),

        other => {
            return Err(ShellError::runtime(unknown_message(other)));
        }
    })
}

/// The closest known style method name, for "did you mean" errors.
///
/// A typo in a style name is otherwise invisible — it does not change the
/// rendering, it just fails to. The threshold is tight on purpose: a wrong
/// suggestion is worse than none, so a candidate is offered only within two
/// edits, relaxed to a third of the name for longer identifiers where two edits
/// is proportionally stricter.
pub fn suggest(name: &str) -> Option<&'static str> {
    let budget = 2.max(name.chars().count() / 3);
    let mut best: Option<(usize, &'static str)> = None;
    for candidate in known_names() {
        let distance = edit_distance(name, candidate);
        if distance > budget {
            continue;
        }
        if best.is_none_or(|(best_distance, _)| distance < best_distance) {
            best = Some((distance, candidate));
        }
    }
    best.map(|(_, candidate)| candidate)
}

/// Every known style method name (nullary + parametric), for diagnostics.
///
/// Sorted so that a dumped list is stable across runs; reflection order is
/// macro-expansion order and carries no meaning for a reader.
pub fn known_names() -> Vec<&'static str> {
    let mut names: Vec<&'static str> = table()
        .nullary
        .iter()
        .map(|method| method.name)
        .chain(EXTRA_NULLARY.iter().map(|(name, _)| *name))
        .chain(PARAM_STYLES.iter().copied())
        .collect();
    names.sort_unstable();
    names.dedup();
    names
}

fn unknown_message(name: &str) -> String {
    match suggest(name) {
        Some(candidate) => format!("unknown style method `{name}` (did you mean: {candidate}?)"),
        None => format!("unknown style method `{name}`"),
    }
}

/// A length as written in a script, before it is narrowed to what a given method
/// accepts.
///
/// The three GPUI length types form a hierarchy (`Length` ⊃ `DefiniteLength` ⊃
/// `AbsoluteLength`), so parsing once and narrowing afterwards lets the error
/// say *which* form was rejected rather than just "bad argument".
enum LengthLiteral {
    Absolute(AbsoluteLength),
    /// A percentage, stored as the fraction GPUI wants.
    Fraction(f32),
    Auto,
}

/// A bare number is pixels — the same rule as [`Bridged::as_pixels`]. Strings
/// carry an explicit unit so `"50%"` and `"1rem"` are unambiguous.
fn parse_length(value: &Bridged, method: &str) -> ShellResult<LengthLiteral> {
    if let Bridged::Str(text) = value {
        let text = text.trim();
        if text == "auto" {
            return Ok(LengthLiteral::Auto);
        }
        if let Some(number) = text.strip_suffix('%') {
            return parse_number(number, text, method)
                .map(|value| LengthLiteral::Fraction(value / 100.));
        }
        if let Some(number) = text.strip_suffix("rem") {
            return parse_number(number, text, method)
                .map(|value| LengthLiteral::Absolute(rems(value).into()));
        }
        if let Some(number) = text.strip_suffix("px") {
            return parse_number(number, text, method)
                .map(|value| LengthLiteral::Absolute(px(value).into()));
        }
        return Err(ShellError::runtime(format!(
            "`{method}` expects a length: a number of pixels, or a string like \
             \"50%\", \"12px\", \"1rem\" or \"auto\"; got \"{text}\""
        )));
    }

    Ok(LengthLiteral::Absolute(value.as_pixels()?.into()))
}

fn parse_number(number: &str, text: &str, method: &str) -> ShellResult<f32> {
    number.trim().parse::<f32>().map_err(|_| {
        ShellError::runtime(format!(
            "`{method}` could not read a number in the length \"{text}\""
        ))
    })
}

fn length(value: &Bridged, method: &str) -> ShellResult<Length> {
    Ok(match parse_length(value, method)? {
        LengthLiteral::Absolute(absolute) => Length::Definite(absolute.into()),
        LengthLiteral::Fraction(fraction) => Length::Definite(relative(fraction)),
        LengthLiteral::Auto => Length::Auto,
    })
}

/// A bare number is a multiplier; anything else follows the length grammar.
fn line_height(value: &Bridged, method: &str) -> ShellResult<DefiniteLength> {
    match value {
        Bridged::Number(multiplier) => Ok(relative(*multiplier as f32)),
        other => definite_length(other, method),
    }
}

fn definite_length(value: &Bridged, method: &str) -> ShellResult<DefiniteLength> {
    match parse_length(value, method)? {
        LengthLiteral::Absolute(absolute) => Ok(absolute.into()),
        LengthLiteral::Fraction(fraction) => Ok(relative(fraction)),
        LengthLiteral::Auto => Err(ShellError::runtime(format!(
            "`{method}` cannot be \"auto\"; it expects a definite length such as 12 or \"50%\""
        ))),
    }
}

fn absolute_length(value: &Bridged, method: &str) -> ShellResult<AbsoluteLength> {
    match parse_length(value, method)? {
        LengthLiteral::Absolute(absolute) => Ok(absolute),
        LengthLiteral::Fraction(_) | LengthLiteral::Auto => Err(ShellError::runtime(format!(
            "`{method}` expects an absolute length such as 8 or \"0.5rem\"; \
             percentages and \"auto\" are not allowed here"
        ))),
    }
}

/// Levenshtein distance over `char`s, with a rolling row so the allocation is
/// one `Vec` per comparison rather than a full matrix.
fn edit_distance(left: &str, right: &str) -> usize {
    let right: Vec<char> = right.chars().collect();
    let mut previous: Vec<usize> = (0..=right.len()).collect();
    let mut current = vec![0; right.len() + 1];

    for (i, left_char) in left.chars().enumerate() {
        current[0] = i + 1;
        for (j, right_char) in right.iter().enumerate() {
            let substitution = previous[j] + usize::from(left_char != *right_char);
            current[j + 1] = substitution.min(previous[j + 1] + 1).min(current[j] + 1);
        }
        std::mem::swap(&mut previous, &mut current);
    }

    previous[right.len()]
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{Fill, Hsla};

    #[test]
    fn the_reflection_table_is_populated() {
        // Guards the `gpui-base/inspector` feature: without it this table is
        // empty in release builds and every no-argument style silently stops
        // working. Run this test with `--release` in CI.
        assert!(
            table().nullary.len() > 100,
            "expected hundreds of reflected style methods, got {}",
            table().nullary.len()
        );
    }

    #[test]
    fn a_nullary_name_round_trips_through_its_index() {
        let index = nullary_index("items_center").expect("items_center is a reflected style");
        assert_eq!(nullary_name(index), "items_center");

        let styled = apply_nullary(index, StyleRefinement::default());
        assert_eq!(styled.align_items, Some(gpui::AlignItems::Center));
    }

    #[test]
    fn an_out_of_range_index_is_printable_and_inert() {
        let index = u16::MAX;
        assert_eq!(nullary_name(index), "<unknown style>");
        assert_eq!(
            apply_nullary(index, StyleRefinement::default()),
            StyleRefinement::default()
        );
    }

    #[test]
    fn bg_sets_a_background() {
        let styled = apply_param(
            "bg",
            &[Bridged::Str("#ff0000".into())],
            StyleRefinement::default(),
        )
        .unwrap();

        let expected: Fill = Hsla::from(gpui::rgba(0xff0000ff)).into();
        assert_eq!(styled.background, Some(expected));
    }

    #[test]
    fn a_bare_number_is_pixels_and_a_percent_string_is_relative() {
        let padded = apply_param("p", &[Bridged::Number(12.)], StyleRefinement::default()).unwrap();
        assert_eq!(padded.padding.top, Some(px(12.).into()));

        let wide = apply_param(
            "w",
            &[Bridged::Str("50%".into())],
            StyleRefinement::default(),
        )
        .unwrap();
        assert_eq!(wide.size.width, Some(Length::Definite(relative(0.5))));

        let auto = apply_param(
            "w",
            &[Bridged::Str("auto".into())],
            StyleRefinement::default(),
        )
        .unwrap();
        assert_eq!(auto.size.width, Some(Length::Auto));
    }

    #[test]
    fn a_wrongly_typed_argument_names_the_expected_type() {
        let error = apply_param("bg", &[Bridged::Number(1.)], StyleRefinement::default())
            .unwrap_err()
            .to_string();
        assert!(
            error.contains("expected a string"),
            "error should name the expected type, got: {error}"
        );

        let error = apply_param(
            "p",
            &[Bridged::Str("auto".into())],
            StyleRefinement::default(),
        )
        .unwrap_err()
        .to_string();
        assert!(
            error.contains("definite length"),
            "error should explain why `auto` is rejected, got: {error}"
        );
    }

    #[test]
    fn a_missing_argument_names_the_method() {
        let error = apply_param("p", &[], StyleRefinement::default())
            .unwrap_err()
            .to_string();
        assert!(error.contains("`p` expects at least 1 argument"), "{error}");
    }

    #[test]
    fn a_close_typo_gets_a_suggestion() {
        assert_eq!(suggest("items_centre"), Some("items_center"));
        assert_eq!(suggest("text_colour"), Some("text_color"));
        assert_eq!(suggest("rounde"), Some("rounded"));
    }

    #[test]
    fn a_name_with_nothing_close_gets_no_suggestion() {
        assert_eq!(suggest("on_click"), None);
        assert_eq!(suggest("completely_unrelated_name"), None);
    }

    #[test]
    fn every_parametric_name_is_bound_and_disjoint_from_reflection() {
        for name in PARAM_STYLES {
            assert_eq!(param_style_name(name), Some(*name));
            assert!(
                nullary_index(name).is_none(),
                "`{name}` is both reflected and hand-bound; the dispatcher would have to \
                 pick one arbitrarily"
            );
            // A bound name must not fall through to the unknown-method arm.
            let error = apply_param(name, &[], StyleRefinement::default())
                .expect_err("a style method needs at least one argument")
                .to_string();
            assert!(!error.contains("unknown style method"), "`{name}`: {error}");
        }
    }

    #[test]
    fn known_names_covers_both_halves() {
        let names = known_names();
        assert!(names.contains(&"items_center"));
        assert!(names.contains(&"bg"));
        assert!(names.windows(2).all(|pair| pair[0] <= pair[1]));
    }

    #[test]
    fn edit_distance_counts_single_edits() {
        assert_eq!(edit_distance("", "abc"), 3);
        assert_eq!(edit_distance("abc", "abc"), 0);
        assert_eq!(edit_distance("items_centre", "items_center"), 2);
    }
}
