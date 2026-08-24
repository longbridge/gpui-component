// The application's own presentation layer.
//
// gpui-base ships behavior with no styling at all, so every visual decision in
// this app is made here — that is the point of building on base rather than on
// a finished component library. Keeping the decisions in one module is also
// what stops them from drifting: sizes, radii and spacing come from these
// helpers, not from the screens.

import { div, h_flex, v_flex, text, Button, Checkbox } from "gpui";

// Spacing follows the semantic scale: 2, 4, 8, 12, 16, 24, 32.
export const SPACE = { xxs: 2, xs: 4, sm: 8, md: 12, lg: 16, xl: 24, xxl: 32 };

// Type stays on 12 / 13 / 14 / 16 / 20 — enough levels to build a hierarchy,
// few enough that the hierarchy stays readable.
export const label = (value) =>
  text(value).text_size(13).line_height(1.45).text_color("foreground");

export const muted = (value) =>
  text(value).text_size(12).line_height(1.45).text_color("muted_foreground");

export const title = (value) =>
  text(value).text_size(20).line_height(1.3).font_semibold().text_color("foreground");

export const sectionTitle = (value) =>
  text(value)
    .text_size(12)
    .line_height(1.4)
    .font_semibold()
    .text_color("muted_foreground");

/// A hairline divider. One pixel, border token, nothing else.
export const rule = () => div().h(1).w_full().bg("border");

/// The window's single content surface. Cards are not nested inside cards.
export const surface = () =>
  v_flex()
    .flex_1()
    .bg("surface")
    .border(1)
    .border_color("border")
    .rounded(8)
    .overflow_hidden();

const VARIANTS = {
  primary: { bg: "primary", fg: "primary_foreground", border: "primary" },
  secondary: { bg: "secondary", fg: "secondary_foreground", border: "border" },
  ghost: { bg: "surface", fg: "muted_foreground", border: "surface" },
};

/// A button. `selected` is a state, not a variant: a selected ghost button is
/// still a ghost button, it just reads as current.
export const button = (id, caption, onClick, options = {}) => {
  const { variant = "secondary", disabled = false, selected = false } = options;
  const palette = VARIANTS[variant] ?? VARIANTS.secondary;
  const background = selected ? "accent" : palette.bg;
  const foreground = selected ? "accent_foreground" : palette.fg;

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
    .bg(background)
    .text_size(13)
    .line_height(1)
    .text_color(foreground)
    .when(!disabled, (el) => el.hover((style) => style.opacity(0.88)))
    .when(!disabled, (el) => el.active((style) => style.opacity(0.75)))
    .when(disabled, (el) => el.opacity(0.45))
    .when(!disabled, (el) => el.on_click(onClick))
    .child(text(caption));
};

/// A checkbox row. Base's Checkbox carries the behavior — activation, keyboard,
/// accessibility — and this draws the indicator, because base draws nothing.
export const checkbox = (id, checked, onChange, content) =>
  Checkbox.new(id)
    .checked(checked)
    .flex()
    .items_center()
    .gap(SPACE.md)
    .w_full()
    .py(SPACE.sm)
    .px(SPACE.lg)
    .rounded(6)
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
        .when(checked, (el) =>
          el.child(text("✓").text_size(11).line_height(1).text_color("primary_foreground")),
        ),
    )
    .child(content);

/// A short classification. Neutral by default; `tone` is spent only where the
/// distinction changes what the reader does.
export const tag = (caption, tone = "neutral") => {
  const palette =
    tone === "blocking"
      ? { bg: "destructive", fg: "destructive_foreground" }
      : { bg: "muted", fg: "muted_foreground" };

  return div()
    .px(SPACE.sm)
    .py(SPACE.xxs)
    .rounded_full()
    .bg(palette.bg)
    .child(text(caption).text_size(11).line_height(1.3).text_color(palette.fg));
};

/// A count that reads at a glance, with its unit underneath.
export const stat = (value, caption) =>
  v_flex()
    .gap(SPACE.xxs)
    .child(text(String(value)).text_size(20).line_height(1.2).font_semibold().text_color("foreground"))
    .child(muted(caption));

/// What a region says when it has nothing to show. It explains the next action
/// rather than apologising.
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
