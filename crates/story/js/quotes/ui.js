// The script half's presentation layer.
//
// `gpui-shell` binds `gpui-base`, which ships behavior and no visual style: a
// Button here has hit testing, focus and hover state, and not one pixel of
// appearance. Every colour, size and radius below is this file's decision —
// read from the host through `native("theme")`, so changing the gallery's theme
// moves this half too.

import { div, h_flex, v_flex, text, Button, native } from "gpui";
/** @import { AbsoluteLength, ClickEvent, Context, Element } from "gpui" */

/// Every measurement here is in **rems**, so the panel scales with the window's
/// text size instead of pinning itself to a pixel grid that only exists at the
/// default zoom. The one exception is a hairline rule, which is a rule at any
/// size rather than a measurement.
///
/// `shell_story.rs` carries the same numbers. The two boards sit side by side,
/// so one that only lines up at 100% lines up by accident.
/** @type {Record<"xxs" | "xs" | "sm" | "md" | "lg" | "xl", AbsoluteLength>} */
export const SPACE = {
  xxs: "0.125rem",
  xs: "0.25rem",
  sm: "0.5rem",
  md: "0.75rem",
  lg: "1rem",
  xl: "1.5rem",
};

/** @type {Record<"symbol" | "price" | "percent" | "volume", AbsoluteLength>} */
export const COLUMN = {
  symbol: "4.875rem",
  price: "4.25rem",
  percent: "4.125rem",
  volume: "5.125rem",
};

/** @type {Record<"padding" | "gap" | "inset" | "marker" | "halfMarker", AbsoluteLength>} */
export const ROW = {
  padding: "0.125rem",
  gap: "0.125rem",
  inset: "0.5rem",
  marker: "0.375rem",
  /// Spelled out because a rem string cannot be halved by dividing.
  halfMarker: "0.1875rem",
};

/** @type {{ title: AbsoluteLength, body: AbsoluteLength, lineHeight: number }} */
export const TYPE = { title: "0.8125rem", body: "0.6875rem", lineHeight: 1.4 };

/** @type {Palette | null} */
let current = null;

/// Re-reads the host palette, once at the top of a render. Not cached across
/// renders: the gallery can switch theme while this view is mounted.
export const refreshPalette = () => {
  current = native("theme").palette();
  return current;
};

export const palette = () => current ?? refreshPalette();

/// Up is `success`, down is `danger`, flat is ordinary text — the same question
/// the Rust panel asks of the same theme.
/** @param {number} direction */
export const directionColor = (direction) => {
  const colors = palette();
  if (direction > 0) return colors.success;
  if (direction < 0) return colors.danger;
  return colors.foreground;
};

// -- Type -------------------------------------------------------------------

/** @param {string} value */
export const title = (value) =>
  text(value)
    .text_size(TYPE.title)
    .line_height(1.3)
    .font_semibold()
    .text_color(palette().foreground);

/** @param {string} value */
export const label = (value) =>
  text(value).text_size(TYPE.body).line_height(TYPE.lineHeight).text_color(palette().foreground);

/** @param {string} value */
export const muted = (value) =>
  text(value)
    .text_size(TYPE.body)
    .line_height(TYPE.lineHeight)
    .text_color(palette().muted_foreground);

// -- Surfaces ---------------------------------------------------------------

/// The panel's root: layout only. The Rust `section` around it already draws the
/// card, and the Rust board has no inner frame either.
export const surface = () => v_flex().w_full().gap(SPACE.md);

// One real pixel: a rule is a rule at any zoom, not a measurement that scales.
// One real pixel: a rule is a rule at any zoom, not a measurement that scales.
export const rule = () => div().w_full().h(1).flex_none().bg(palette().border);

// -- Board parts ------------------------------------------------------------

/** @param {AbsoluteLength} width @param {{ right?: boolean }} [options] */
export const cell = (width, options = {}) => {
  const box = div().w(width).flex_none();
  return options.right ? box.text_right() : box;
};

/// The header. It ends with an empty cell the width of the watched marker,
/// because a trailing column the header does not know about puts every caption
/// out of line with the numbers under it.
export const header = () =>
  h_flex()
    .w_full()
    .items_center()
    .gap(ROW.inset)
    .px(ROW.inset)
    .pb(SPACE.xs)
    .border_b(1)
    .border_color(palette().border)
    .child(cell(COLUMN.symbol).child(muted("Symbol")))
    .child(div().flex_1())
    .child(cell(COLUMN.price, { right: true }).child(muted("Last")))
    .child(cell(COLUMN.percent, { right: true }).child(muted("Change")))
    .child(cell(COLUMN.volume, { right: true }).child(muted("Volume")))
    .child(cell(ROW.marker));

/// A full-width row that behaves like a button. The id is the symbol rather than
/// the row's position, so identity follows the instrument if the board reorders.
/** @param {Quote} quote @param {(event: ClickEvent, cx: Context) => void} onClick */
export const quoteRow = (quote, onClick) =>
  Button.new(`quote-${quote.symbol}`)
    .accessibility_label(`Watch ${quote.name}`)
    .flex()
    .w_full()
    .items_center()
    .gap(ROW.inset)
    .px(ROW.inset)
    .py(ROW.padding)
    .rounded(palette().radius)
    .hover((style) => style.bg(palette().muted))
    .on_click(onClick)
    .child(cell(COLUMN.symbol).child(label(quote.symbol).font_medium()))
    .child(div().flex_1().child(muted(quote.name)))
    .child(
      cell(COLUMN.price, { right: true }).child(
        label(quote.last).text_color(directionColor(quote.direction)),
      ),
    )
    .child(
      cell(COLUMN.percent, { right: true }).child(
        label(quote.percent).text_color(directionColor(quote.direction)),
      ),
    )
    .child(cell(COLUMN.volume, { right: true }).child(muted(quote.volume)))
    .child(watchMarker(quote.watched));

/** @param {boolean} watched */
export const watchMarker = (watched) =>
  div()
    .w(ROW.marker)
    .h(ROW.marker)
    .flex_none()
    .rounded(ROW.halfMarker)
    .when(watched, (el) => el.bg(palette().primary));

/// A labelled action. Two treatments only — filled and outlined.
/**
 * @param {string} id
 * @param {string} caption
 * @param {(event: ClickEvent, cx: Context) => void} onClick
 * @param {{ primary?: boolean, disabled?: boolean }} [options]
 */
export const action = (id, caption, onClick, options = {}) => {
  const { primary = false, disabled = false } = options;
  const colors = palette();

  return Button.new(id)
    .disabled(disabled)
    .flex()
    .items_center()
    .justify_center()
    .h("1.25rem")
    .px(SPACE.sm)
    .rounded(colors.radius)
    .border(1)
    .border_color(primary ? colors.primary : colors.border)
    .bg(primary ? colors.primary : colors.background)
    .when(disabled, (el) => el.opacity(0.5))
    .when(!disabled, (el) =>
      el
        .hover((style) => style.bg(primary ? colors.primary_hover : colors.muted))
        .on_click(onClick),
    )
    .child(
      text(caption)
        .text_size(TYPE.body)
        .line_height(1)
        .text_color(primary ? colors.primary_foreground : colors.foreground),
    );
};
