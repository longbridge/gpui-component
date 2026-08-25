// The script half's presentation layer.
//
// `gpui-shell` binds `gpui-base`, which ships behavior and no visual style at
// all: a Button here has hit testing, focus and hover state, and not one pixel
// of appearance. So every color, size and radius below is a decision this file
// makes — that is what "the script owns presentation" means.
//
// The decisions are not invented, though. They are read from the host through
// `native("theme")`, which hands back the very `gpui-component` theme the Rust
// panel across the window is painting with. Change the gallery's theme or its
// radius and this half follows, without a rebuild and without a shared
// stylesheet: the host answered a question, the script drew the answer.

import { div, h_flex, v_flex, text, Button, native } from "gpui";

/// The spacing scale, matching the semantic steps the host uses.
export const SPACE = { xxs: 2, xs: 4, sm: 8, md: 12, lg: 16, xl: 24 };

/// The column widths, matching the Rust panel's constants.
///
/// The two boards sit side by side and the reader is comparing them, so a
/// column that is 68 wide over there and 70 here would turn the comparison into
/// one about alignment.
export const COLUMN = { symbol: 78, price: 68, percent: 66, volume: 82 };

let current = null;

/// Re-reads the host palette. Called once at the top of a render.
///
/// Not cached across renders on purpose: the gallery can switch theme, mode and
/// radius while this view is mounted, and a palette captured once would leave
/// this half painted in the previous theme with no way to notice.
export const refreshPalette = () => {
  current = native("theme").palette();
  return current;
};

/// The palette read at the start of this render.
export const palette = () => current ?? refreshPalette();

/// Up is `success`, down is `danger`, flat is ordinary text — the same question
/// the Rust panel asks of the same theme, which is why the two agree.
export const directionColor = (direction) => {
  const colors = palette();
  if (direction > 0) return colors.success;
  if (direction < 0) return colors.danger;
  return colors.foreground;
};

// -- Type -------------------------------------------------------------------

export const title = (value) =>
  text(value).text_size(13).line_height(1.3).font_semibold().text_color(palette().foreground);

export const label = (value) =>
  text(value).text_size(11).line_height(1.4).text_color(palette().foreground);

export const muted = (value) =>
  text(value).text_size(11).line_height(1.45).text_color(palette().muted_foreground);

// -- Surfaces ---------------------------------------------------------------

/// The one content surface. It sits inside a Rust `GroupBox`, so it reads as
/// inset rather than raised: a hairline border and the window background, no
/// shadow.
export const surface = () =>
  v_flex()
    .w_full()
    .gap(SPACE.md)
    .p(SPACE.md)
    .bg(palette().background)
    .border(1)
    .border_color(palette().border)
    .rounded(palette().radius);

export const rule = () => div().w_full().h(1).flex_none().bg(palette().border);

// -- Board parts ------------------------------------------------------------

/// One cell. Fixed width and right-aligned for the numbers, so the columns line
/// up with the Rust panel's without either side measuring the other.
export const cell = (width, options = {}) => {
  const { right = false } = options;
  const box = div().w(width).flex_none();
  return right ? box.text_right() : box;
};

/// The header row, which is a rule with captions on it rather than a row of its
/// own: the reader needs the column names once, quietly.
export const header = () =>
  h_flex()
    .w_full()
    .items_center()
    .gap(SPACE.sm)
    .px(SPACE.sm)
    .pb(SPACE.xs)
    .border_b(1)
    .border_color(palette().border)
    .child(cell(COLUMN.symbol).child(muted("Symbol")))
    .child(div().flex_1())
    .child(cell(COLUMN.price, { right: true }).child(muted("Last")))
    .child(cell(COLUMN.percent, { right: true }).child(muted("Change")))
    .child(cell(COLUMN.volume, { right: true }).child(muted("Volume")));

/// A full-width row that behaves like a button: no fill of its own, a hover
/// wash, and the whole row is the target.
///
/// The id is the symbol rather than the row's position, so a board that
/// reorders keeps each row's identity — and its pressed state — attached to the
/// instrument rather than to the slot it happened to be in.
export const quoteRow = (quote, onClick) =>
  Button.new(`quote-${quote.symbol}`)
    .accessibility_label(`Watch ${quote.name}`)
    .flex()
    .w_full()
    .items_center()
    .gap(SPACE.sm)
    .px(SPACE.sm)
    .py(SPACE.xs)
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

/// The watched dot at the end of a row. Six pixels, because it is a state and
/// not a control: the row is the control.
export const watchMarker = (watched) =>
  div()
    .w(6)
    .h(6)
    .flex_none()
    .rounded(3)
    .when(watched, (el) => el.bg(palette().primary));

/// A labelled action. Two treatments only — filled and outlined — because a
/// third would be a distinction this panel does not make.
export const action = (id, caption, onClick, options = {}) => {
  const { primary = false, disabled = false } = options;
  const colors = palette();

  return Button.new(id)
    .disabled(disabled)
    .flex()
    .items_center()
    .justify_center()
    .h(24)
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
        .text_size(11)
        .line_height(1)
        .text_color(primary ? colors.primary_foreground : colors.foreground),
    );
};
