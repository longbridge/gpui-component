// The script half's presentation layer.
//
// `gpui-shell` binds `gpui-base`, which ships behavior and no visual style: a
// Button here has hit testing, focus and hover state, and not one pixel of
// appearance. Every colour, size and radius below is this file's decision —
// read from the host through `native("theme")`, so changing the gallery's theme
// moves this half too.

import { div, h_flex, v_flex, text, Button, native } from "gpui";

export const SPACE = { xxs: 2, xs: 4, sm: 8, md: 12, lg: 16, xl: 24 };

/// Column widths, row density and type sizes, all mirroring the constants in
/// `shell_story.rs`. The two boards sit side by side, so a number that differs
/// makes the comparison about layout instead of about rendering.
export const COLUMN = { symbol: 78, price: 68, percent: 66, volume: 82 };
export const ROW = { padding: 2, gap: 2, inset: 8, marker: 6 };
export const TYPE = { title: 13, body: 11, lineHeight: 1.4 };

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
export const directionColor = (direction) => {
  const colors = palette();
  if (direction > 0) return colors.success;
  if (direction < 0) return colors.danger;
  return colors.foreground;
};

// -- Type -------------------------------------------------------------------

export const title = (value) =>
  text(value)
    .text_size(TYPE.title)
    .line_height(1.3)
    .font_semibold()
    .text_color(palette().foreground);

export const label = (value) =>
  text(value).text_size(TYPE.body).line_height(TYPE.lineHeight).text_color(palette().foreground);

export const muted = (value) =>
  text(value)
    .text_size(TYPE.body)
    .line_height(TYPE.lineHeight)
    .text_color(palette().muted_foreground);

// -- Surfaces ---------------------------------------------------------------

/// The panel's root: layout only. The Rust `section` around it already draws the
/// card, and the Rust board has no inner frame either.
export const surface = () => v_flex().w_full().gap(SPACE.md);

export const rule = () => div().w_full().h(1).flex_none().bg(palette().border);

// -- Board parts ------------------------------------------------------------

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
/** @param {Quote} quote */
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

export const watchMarker = (watched) =>
  div()
    .w(ROW.marker)
    .h(ROW.marker)
    .flex_none()
    .rounded(ROW.marker / 2)
    .when(watched, (el) => el.bg(palette().primary));

/// A labelled action. Two treatments only — filled and outlined.
export const action = (id, caption, onClick, options = {}) => {
  const { primary = false, disabled = false } = options;
  const colors = palette();

  return Button.new(id)
    .disabled(disabled)
    .flex()
    .items_center()
    .justify_center()
    .h(20)
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
