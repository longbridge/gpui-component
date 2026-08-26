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
/** @import { ClickEvent, Color, Context, Element, InputStateHandle } from "gpui" */

export const SPACE = { xxs: 2, xs: 4, sm: 8, md: 12, lg: 16, xl: 24, xxl: 32 };

/// Body text. 12px is the showcase's `text_xs`, and at this density it is the
/// size that keeps a row scannable.
/** @param {string} value @param {import("gpui").ColorTokens} colors */
export const label = (value, colors) =>
  text(value).text_size(12).line_height(1).text_color(colors.foreground);

/** @param {string} value @param {import("gpui").ColorTokens} colors */
export const muted = (value, colors) =>
  text(value).text_size(12).line_height(1).text_color(colors.muted_foreground);

/** @param {string} value @param {import("gpui").ColorTokens} colors */
export const title = (value, colors) =>
  text(value).text_size(16).line_height(1).font_semibold().text_color(colors.foreground);

/** @param {import("gpui").ColorTokens} colors */
export const rule = (colors) => div().h(1).w_full().bg(colors.border);

/// The one content surface. Square, hairline bordered, no elevation: hierarchy
/// comes from the border and the spacing, not from a shadow.
/** @param {import("gpui").ColorTokens} colors */
export const surface = (colors) =>
  v_flex().flex_1().bg(colors.surface).border(1).border_color(colors.border).overflow_hidden();

/// An icon from the application's own `icons/` directory.
///
/// Asset paths resolve against the application root — the directory passed to
/// gpui-shell — not against this file, unlike the `import` above. It inherits
/// the surrounding text color, so an icon in a muted row is muted.
/** @param {string} name @param {number} [size] */
export const icon = (name, size = 14) =>
  svg(`icons/${name}.svg`).w(size).h(size).flex_none();

// Two variants, not five. The showcase has exactly this pair — a filled
// primary and an outlined secondary — and a third would be a decision nobody
// asked for.
/** @param {import("gpui").ColorTokens} colors @returns {Record<Variant, { bg: Color, fg: Color, border: Color, hover: Color }>} */
const variants = (colors) => ({
  primary: { bg: colors.foreground, fg: colors.surface, border: colors.foreground, hover: colors.muted_foreground },
  secondary: { bg: colors.surface, fg: colors.foreground, border: colors.border, hover: colors.muted },
  ghost: { bg: colors.surface, fg: colors.muted_foreground, border: colors.surface, hover: colors.muted },
  danger: { bg: colors.surface, fg: colors.destructive, border: colors.border, hover: colors.muted },
});

/**
 * @param {string} id
 * @param {string} caption
 * @param {(event: ClickEvent, cx: Context) => void} onClick
 * @param {import("gpui").ColorTokens} colors
 * @param {ButtonOptions} [options]
 */
export const button = (id, caption, onClick, colors, options = {}) => {
  const { variant = "secondary", disabled = false, selected = false, icon: name } = options;
  const palettes = variants(colors);
  const palette = palettes[variant] ?? palettes.secondary;

  return Button.new(id)
    .disabled(disabled)
    .flex()
    .items_center()
    .justify_center()
    .h(28)
    .px(SPACE.md)
    .gap(SPACE.xs)
    .border(1)
    .border_color(selected ? colors.foreground : palette.border)
    .bg(selected ? colors.muted : palette.bg)
    .text_size(12)
    .line_height(1)
    .text_color(selected ? colors.foreground : palette.fg)
    .when(!disabled, (el) => el.hover((style) => style.bg(palette.hover)))
    .when(disabled, (el) => el.opacity(0.4))
    .when(!disabled, (el) => el.on_click(onClick))
    .when(Boolean(name), (el) => el.child(icon(name ?? "", 13)))
    .child(text(caption));
};

/// An icon-only button. It carries an accessibility label, because an icon
/// alone announces nothing to a screen reader.
/**
 * @param {string} id
 * @param {string} name
 * @param {string} description
 * @param {(event: ClickEvent, cx: Context) => void} onClick
 * @param {import("gpui").ColorTokens} colors
 * @param {{ disabled?: boolean }} [options]
 */
export const iconButton = (id, name, description, onClick, colors, options = {}) => {
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
    .border_color(colors.surface)
    .text_color(colors.muted_foreground)
    .when(!disabled, (el) =>
      el.hover((style) => style.bg(colors.muted).border_color(colors.border).text_color(colors.foreground)),
    )
    .when(disabled, (el) => el.opacity(0.4))
    .when(!disabled, (el) => el.on_click(onClick))
    .child(icon(name, 14));
};

/// A text field. The runtime frames an input as a centered row that focuses on
/// click; height, padding and color are still ours.
/** @param {InputStateHandle} state @param {import("gpui").ColorTokens} colors */
export const field = (state, colors) =>
  Input.new(state)
    .flex_1()
    .h(28)
    .px(SPACE.sm)
    .border(1)
    .border_color(colors.input)
    .bg(colors.surface)
    .text_size(12);

/// A checkbox row, indicator and all: base draws neither.
/**
 * @param {string} id
 * @param {boolean} checked
 * @param {(checked: boolean, cx: Context) => void} onChange
 * @param {Element} content
 * @param {import("gpui").ColorTokens} colors
 */
export const checkbox = (id, checked, onChange, content, colors) =>
  Checkbox.new(id)
    .checked(checked)
    .flex()
    .items_center()
    .gap(SPACE.md)
    .w_full()
    .py(SPACE.sm)
    .px(SPACE.md)
    .hover((style) => style.bg(colors.muted))
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
        .border_color(colors.foreground)
        .when(checked, (el) => el.bg(colors.foreground).child(icon("check", 11).text_color(colors.surface))),
    )
    .child(content);

/** @param {string} heading @param {string} hint @param {import("gpui").ColorTokens} colors */
export const emptyState = (heading, hint, colors) =>
  v_flex()
    .flex_1()
    .items_center()
    .justify_center()
    .gap(SPACE.xs)
    .py(SPACE.xxl)
    .child(label(heading, colors))
    .child(muted(hint, colors));

export const row = () => h_flex().items_center().gap(SPACE.md);
