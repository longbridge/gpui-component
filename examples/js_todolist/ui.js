// The application's presentation layer.
//
// gpui-base ships behavior with no styling, so every visual decision lives
// here. Spacing follows the semantic scale (2/4/8/12/16/24/32) and type stays
// on 12/13/14/16/20; colors are semantic tokens, never literals.

import { div, h_flex, v_flex, text, svg, Button, Checkbox, Input } from "gpui";

export const SPACE = { xxs: 2, xs: 4, sm: 8, md: 12, lg: 16, xl: 24, xxl: 32 };

export const label = (value) =>
  text(value).text_size(13).line_height(1).text_color("foreground");

export const muted = (value) =>
  text(value).text_size(12).line_height(1).text_color("muted_foreground");

export const title = (value) =>
  text(value).text_size(20).line_height(1.3).font_semibold().text_color("foreground");

export const rule = () => div().h(1).w_full().bg("border");

export const surface = () =>
  v_flex().flex_1().bg("surface").border(1).border_color("border").overflow_hidden();

const VARIANTS = {
  primary: { bg: "primary", fg: "primary_foreground", border: "primary" },
  secondary: { bg: "secondary", fg: "secondary_foreground", border: "border" },
  ghost: { bg: "surface", fg: "muted_foreground", border: "surface" },
  danger: { bg: "surface", fg: "destructive", border: "border" },
};

export const button = (id, caption, onClick, options = {}) => {
  const { variant = "secondary", disabled = false, selected = false } = options;
  const palette = VARIANTS[variant] ?? VARIANTS.secondary;

  return Button.new(id)
    .disabled(disabled)
    .flex()
    .items_center()
    .justify_center()
    .h(28)
    .px(SPACE.md)
    .gap(SPACE.xs)
    .rounded(6)
    .border(1)
    .border_color(selected ? "accent" : palette.border)
    .bg(selected ? "accent" : palette.bg)
    .text_size(13)
    .line_height(1)
    .text_color(selected ? "accent_foreground" : palette.fg)
    .when(!disabled, (el) => el.hover((style) => style.opacity(0.88)))
    .when(!disabled, (el) => el.active((style) => style.opacity(0.75)))
    .when(disabled, (el) => el.opacity(0.45))
    .when(!disabled, (el) => el.on_click(onClick))
    .when(Boolean(options.icon), (el) => el.child(icon(options.icon, 14)))
    .child(text(caption));
};

/// A text field.
///
/// The runtime frames an input as a centered row that focuses on click; height,
/// padding, radius and color are still ours, because base picks none of them.
export const field = (state) =>
  Input.new(state)
    .flex_1()
    .h(32)
    .px(SPACE.md)
    .rounded(6)
    .border(1)
    .border_color("input")
    .bg("background")
    .text_size(13);

export const checkbox = (id, checked, onChange, content) =>
  Checkbox.new(id)
    .checked(checked)
    .flex()
    .items_center()
    .gap(SPACE.md)
    .w_full()
    .py(SPACE.sm)
    .px(SPACE.lg)
    .hover((style) => style.bg("muted"))
    .on_change(onChange)
    .child(
      div()
        .w(16)
        .h(16)
        .flex()
        .items_center()
        .justify_center()
        .rounded(4)
        .border(1)
        .border_color(checked ? "primary" : "input")
        .bg(checked ? "primary" : "surface")
        .when(checked, (el) => el.child(icon("check", 10).text_color("primary_foreground"))),
    )
    .child(content);

export const emptyState = (heading, hint) =>
  v_flex()
    .flex_1()
    .items_center()
    .justify_center()
    .gap(SPACE.sm)
    .py(SPACE.xxl)
    .child(label(heading))
    .child(muted(hint));

export const row = () => h_flex().items_center().gap(SPACE.md);

/// An icon from the application's own `icons/` directory.
///
/// It takes its color from the text color around it, so an icon inside a muted
/// row is muted without being told.
export const icon = (name, size = 16) =>
  svg(`icons/${name}.svg`).w(size).h(size).flex_none();
