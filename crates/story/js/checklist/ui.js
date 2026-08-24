// The script half's presentation layer.
//
// `gpui-shell` binds `gpui-base`, which ships behaviour and no visual style at
// all: a Button here has hit testing, focus and hover state, and not one pixel
// of appearance. So every colour, size and radius below is a decision this file
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

let current = null;

/// Re-reads the host palette. Called once at the top of a render.
///
/// Not cached across frames on purpose: the gallery can switch theme, mode and
/// radius while this view is mounted, and a palette captured once would leave
/// this half painted in the previous theme with no way to notice.
export const refreshPalette = () => {
  current = native("theme").palette();
  return current;
};

/// The palette read at the start of this frame.
export const palette = () => current ?? refreshPalette();

// -- Type -------------------------------------------------------------------

export const title = (value) =>
  text(value).text_size(13).line_height(1.3).font_semibold().text_color(palette().foreground);

export const label = (value) =>
  text(value).text_size(12).line_height(1.4).text_color(palette().foreground);

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

export const row = () => h_flex().items_center().gap(SPACE.sm);

// -- Parts ------------------------------------------------------------------

/// A progress bar. Two divs: the track, and a fill measured in percent.
export const bar = (fraction) => {
  const filled = `${Math.round(Math.max(0, Math.min(1, fraction)) * 100)}%`;

  return div()
    .w_full()
    .h(6)
    .flex_none()
    .rounded(3)
    .bg(palette().secondary)
    .child(div().h_full().w(filled).rounded(3).bg(palette().primary));
};

/// The done/not-done marker in front of a step.
export const marker = (done) =>
  div()
    .w(12)
    .h(12)
    .flex_none()
    .rounded(6)
    .border(1)
    .border_color(done ? palette().primary : palette().border)
    .when(done, (el) => el.bg(palette().primary));

/// A full-width row that behaves like a button: no fill of its own, a hover
/// wash, and the whole row is the target.
export const stepButton = (id, description, onClick) =>
  Button.new(id)
    .accessibility_label(description)
    .flex()
    .w_full()
    .items_center()
    .justify_between()
    .gap(SPACE.sm)
    .px(SPACE.sm)
    .py(SPACE.xs)
    .rounded(palette().radius)
    .hover((style) => style.bg(palette().muted))
    .on_click(onClick);

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
      el.hover((style) => style.bg(primary ? colors.primary : colors.muted)),
    )
    .child(
      text(caption)
        .text_size(11)
        .line_height(1)
        .text_color(primary ? colors.primary_foreground : colors.foreground),
    );
};
