// The application's presentation layer.
//
// The visual language follows `crates/base/examples/showcase`: a neutral
// grey scale, one-pixel borders, square corners, small type, and 28px
// controls. Where the showcase writes literal colors — it has to, because base
// ships no palette — this reads the shell's semantic tokens instead, so the
// same code follows a theme.
//
// Spacing follows the semantic scale (2/4/8/12/16/24/32) and type stays on
// 12/13/16/20.

import { div, h_flex, v_flex, text, svg, Button, Checkbox, Input } from "gpui";

export const SPACE = { xxs: 2, xs: 4, sm: 8, md: 12, lg: 16, xl: 24, xxl: 32 };

/// Body text. 12px is the showcase's `text_xs`, and at this density it is the
/// size that keeps a row scannable.
export const label = (value) =>
  text(value).text_size(12).line_height(1).text_color("foreground");

export const muted = (value) =>
  text(value).text_size(12).line_height(1).text_color("muted_foreground");

export const title = (value) =>
  text(value).text_size(16).line_height(1).font_semibold().text_color("foreground");

export const rule = () => div().h(1).w_full().bg("border");

/// The one content surface. Square, hairline bordered, no elevation: hierarchy
/// comes from the border and the spacing, not from a shadow.
export const surface = () =>
  v_flex().flex_1().bg("surface").border(1).border_color("border").overflow_hidden();

/// An icon from the application's own `icons/` directory.
///
/// Asset paths resolve against the application root — the directory passed to
/// gpui-shell — not against this file, unlike the `import` above. It inherits
/// the surrounding text color, so an icon in a muted row is muted.
export const icon = (name, size = 14) =>
  svg(`icons/${name}.svg`).w(size).h(size).flex_none();

// Two variants, not five. The showcase has exactly this pair — a filled
// primary and an outlined secondary — and a third would be a decision nobody
// asked for.
const VARIANTS = {
  primary: { bg: "foreground", fg: "surface", border: "foreground", hover: "muted_foreground" },
  secondary: { bg: "surface", fg: "foreground", border: "border", hover: "muted" },
  ghost: { bg: "surface", fg: "muted_foreground", border: "surface", hover: "muted" },
};

export const button = (id, caption, onClick, options = {}) => {
  const { variant = "secondary", disabled = false, selected = false, icon: name } = options;
  const palette = VARIANTS[variant] ?? VARIANTS.secondary;

  return Button.new(id)
    .disabled(disabled)
    .flex()
    .items_center()
    .justify_center()
    .h(28)
    .px(SPACE.md)
    .gap(SPACE.xs)
    .border(1)
    .border_color(selected ? "foreground" : palette.border)
    .bg(selected ? "muted" : palette.bg)
    .text_size(12)
    .line_height(1)
    .text_color(selected ? "foreground" : palette.fg)
    .when(!disabled, (el) => el.hover((style) => style.bg(palette.hover)))
    .when(disabled, (el) => el.opacity(0.4))
    .when(!disabled, (el) => el.on_click(onClick))
    .when(Boolean(name), (el) => el.child(icon(name, 13)))
    .child(text(caption));
};

/// An icon-only button. It carries an accessibility label, because an icon
/// alone announces nothing to a screen reader.
export const iconButton = (id, name, description, onClick, options = {}) => {
  const { disabled = false } = options;

  return Button.new(id)
    .disabled(disabled)
    .accessibility_label(description)
    .flex()
    .items_center()
    .justify_center()
    .w(28)
    .h(28)
    .border(1)
    .border_color("surface")
    .text_color("muted_foreground")
    .when(!disabled, (el) =>
      el.hover((style) => style.bg("muted").border_color("border").text_color("foreground")),
    )
    .when(disabled, (el) => el.opacity(0.4))
    .when(!disabled, (el) => el.on_click(onClick))
    .child(icon(name, 14));
};

/// A text field. The runtime frames an input as a centered row that focuses on
/// click; height, padding and color are still ours.
export const field = (state) =>
  Input.new(state)
    .flex_1()
    .h(28)
    .px(SPACE.sm)
    .border(1)
    .border_color("input")
    .bg("surface")
    .text_size(12);

/// A checkbox row, indicator and all: base draws neither.
export const checkbox = (id, checked, onChange, content) =>
  Checkbox.new(id)
    .checked(checked)
    .flex()
    .items_center()
    .gap(SPACE.md)
    .w_full()
    .py(SPACE.sm)
    .px(SPACE.md)
    .hover((style) => style.bg("muted"))
    .on_change(onChange)
    .child(
      div()
        .w(16)
        .h(16)
        .flex_none()
        .flex()
        .items_center()
        .justify_center()
        .border(1)
        .border_color("foreground")
        .when(checked, (el) => el.bg("foreground").child(icon("check", 11).text_color("surface"))),
    )
    .child(content);

export const emptyState = (heading, hint) =>
  v_flex()
    .flex_1()
    .items_center()
    .justify_center()
    .gap(SPACE.xs)
    .py(SPACE.xxl)
    .child(label(heading))
    .child(muted(hint));

export const row = () => h_flex().items_center().gap(SPACE.md);
