---
title: Styling
description: The fluent style surface, length and colour grammars, semantic theme tokens, and hover / active / focus styles.
order: 4
---

# Styling

The script owns presentation, so this is where most of an application's code goes. Every element accepts the same style surface, written as one fluent chain — exactly what the Rust side writes:

```js
v_flex().size_full().bg("surface").p(12).gap(8).rounded(6);
```

```rust
// The same thing in Rust, on gpui-base.
v_flex().size_full().bg(surface).p(px(12.)).gap(px(8.)).rounded(px(6.))
```

## Two halves, one surface

The style surface has two halves, and they exist for different reasons.

**No-argument methods come from GPUI's reflection table.** `flex_col`, `items_center`, `gap_2`, `rounded_md`, `text_sm`, `size_full`, `font_semibold`, `truncate`, `cursor_pointer` — the whole family, obtained from `gpui_base::styled_ext_reflection_methods` and `gpui::styled_reflection::methods` with no maintenance at all. Not one of these names is written down anywhere in the runtime. When upstream GPUI adds a style method, the script surface has it, and so does the generated `gpui.d.ts`.

The build these pages were written against exposes **3,146** of them. That number is not a design target — it is however many `fn(self) -> Self` style methods GPUI currently has, and it moves when GPUI moves. `gpui-shell types` prints the exact figure for your build.

**Methods that take arguments cannot be reflected**, so there are **57** of them bound by hand. That list is the one hand-maintained table in the styling layer, and it is deliberately small.

The two halves never overlap: a name is in one or the other, and a test fails the build if a name ever lands in both.

## Lengths

A bare number is pixels. A string carries its unit.

```js
.p(12)          // 12px
.w("50%")       // half the parent
.h("auto")
.gap("0.5rem")
```

Which of those a given method accepts follows **its Rust signature**, because that signature is what rejects the bad ones. GPUI has three length types nested inside each other, and the runtime keeps the distinction rather than flattening it:

| Type | Accepts | Rejects |
| --- | --- | --- |
| `Length` | a number, `"12px"`, `"1.5rem"`, `"50%"`, `"auto"` | — |
| `DefiniteLength` | a number, `"12px"`, `"1.5rem"`, `"50%"` | `"auto"` |
| `AbsoluteLength` | a number, `"12px"`, `"1.5rem"` | percentages, `"auto"` |

```text
`p` cannot be "auto"; it expects a definite length such as 12 or "50%"
```

```text
`rounded` expects an absolute length such as 8 or "0.5rem";
percentages and "auto" are not allowed here
```

The distinction is not pedantry: `"auto"` padding and a percentage radius have no meaning in the layout engine underneath, and a runtime that accepted them would have to invent one.

### The parametric methods

| Family | Methods | Argument |
| --- | --- | --- |
| Size | `w` `h` `size` `min_w` `min_h` `min_size` `max_w` `max_h` `max_size` | `Length` |
| Padding | `p` `px` `py` `pt` `pb` `pl` `pr` | `DefiniteLength` |
| Margin | `m` `mx` `my` `mt` `mb` `ml` `mr` | `Length` |
| Position | `inset` `top` `bottom` `left` `right` | `Length` |
| Flex | `gap` `gap_x` `gap_y` | `DefiniteLength` |
| Flex | `flex_basis` | `Length` |
| Flex | `flex_grow` `flex_shrink` | number |
| Border | `border` `border_t` `border_b` `border_l` `border_r` `border_x` `border_y` | `AbsoluteLength` |
| Radius | `rounded` and the `_t` `_b` `_l` `_r` `_tl` `_tr` `_bl` `_br` forms | `AbsoluteLength` |
| Paint | `bg` `text_color` `text_bg` `border_color` | colour |
| Paint | `text_size` | `AbsoluteLength` |
| Paint | `line_height` | `DefiniteLength` |
| Paint | `opacity` | number |

`line_height` is the one exception worth memorizing: a **bare number is a multiplier**, not pixels. `line_height(1.45)` means 1.45× the font size, because that is what it means everywhere else in the industry and 1.45px is never what anyone meant. A string still follows the ordinary grammar.

### What is deliberately not bound

`shadow`, `cursor`, `text_align`, `text_overflow`, `font_weight` and `scrollbar_width` take Rust structs or enums rather than scalars, and are not exposed as parametric methods. Every one of them has a no-argument form that is reflected and works today: `shadow_sm`, `cursor_pointer`, `text_center`, `truncate`, `font_bold`. A real shadow API belongs with the token work, not as a positional argument list.

## Colours

A colour is either a **semantic token name** or a hex literal:

```js
.bg("surface")            // follows the theme
.text_color("#1e88e5")    // does not
```

The palette defines seventeen tokens:

| | |
| --- | --- |
| Ground | `background`, `foreground` |
| Surfaces | `surface`, `surface_foreground` |
| Emphasis | `primary`, `primary_foreground`, `secondary`, `secondary_foreground` |
| Recessive | `muted`, `muted_foreground` |
| Highlight | `accent`, `accent_foreground` |
| Danger | `destructive`, `destructive_foreground` |
| Chrome | `border`, `input`, `ring` |

Hex literals accept `#rgb`, `#rrggbb` and `#rrggbbaa`.

**Prefer a token.** A literal bypasses the theme, and a theme switch will not reach it. The example application makes exactly this point: it follows the visual language of `crates/base/examples/showcase`, which has to write literal colours because Base ships no palette, and reads semantic tokens instead — so the same code follows a theme that the Rust showcase cannot.

A mistyped token names the whole set rather than failing vaguely:

```text
unknown color token `surfacee`; expected one of: background, foreground, surface, … —
or a #rrggbb literal
```

### Why the runtime ships a palette at all

`gpui_base::Theme::default()` derives its colour tokens with `#[derive(Default)]`, which means every one of them starts as `Hsla { h: 0, s: 0, l: 0, a: 0 }` — fully transparent. A runtime that only called `gpui_base::init` would paint an invisible window.

So `gpui-shell` ships a default light and dark palette, loaded from a JSON file that uses the same `Serialize`/`Deserialize`/`JsonSchema` derives a plugin theme would. It is a **convenience, not a contract**: the shell's Rust side makes no other visual decision, and the palette exists only so that "the script owns presentation" does not start from a black rectangle.

## State styles

`hover`, `active` and `focus` take a function, which receives a detached element that collects the declarations:

```js
Button.new("save")
  .bg("surface")
  .border(1)
  .border_color("border")
  .hover((style) => style.bg("muted").border_color("foreground"))
  .active((style) => style.bg("border"))
  .focus((style) => style.border_color("ring"))
  .child(text("Save"));
```

The function's return value is ignored, so a chain and a block body both work. The declarations inside are the **ordinary style methods** — there is no second grammar for "what a style is", and every length and colour rule above applies unchanged.

Two implementation facts leak far enough to be worth knowing:

- **`active` and `focus` need a stable element identity.** A plain `div` acquires one lazily, derived from its position in the description, which is stable across renders for a stable tree. `Button`, `Checkbox` and `Input` already have one.
- **A `Switch` ignores state styles.** The switch root is not the interactive element — its track is — so a state style on it has nowhere to land. The runtime logs a warning saying to style the row around it instead, rather than dropping the declaration silently.

## There is no `class("...")`

A reasonable question, given how the names read: why not accept a string of style names, the way a utility-CSS framework does?

```js
div().class("flex items-center gap-2 bg-surface");   // not available
```

Three reasons, in order of weight.

**A typo in a string is invisible.** `class("items-centre")` changes nothing on screen — it simply fails to. A method call can be checked at the call site, and is: an unknown name throws immediately with a suggestion. That single property is why the surface is methods.

**It would be a second way to write the same thing.** Two equivalent spellings split the examples, the documentation, the generated `.d.ts` and the code a model produces, and the runtime consistently refuses that trade.

**The parser would be a second grammar.** `bg-surface`, `p-12`, `w-1/2` are a language, with its own escaping, its own errors, and its own version skew against GPUI's method names. The method surface has none of that, and costs nothing to maintain because it is generated.

## Unknown methods

```text
unknown style method `text_colour` (did you mean: text_color?)
```

The suggestion is a Levenshtein match against the full name list, with a tight budget — two edits, relaxed to a third of the name for longer identifiers. A wrong suggestion is worse than none.

There is a nice piece of machinery behind that message, and it explains a number in the source. QuickJS reports a missing method as a bare `TypeError: not a function` **without naming the property**, so a mistyped style name would otherwise arrive with no clue at all. Wrapping the element prototype in a `Proxy` fixes that — and measured at roughly 30% of the entire description pass (1.09 ms → 1.42 ms for 443 nodes).

So the runtime keeps a fast plain prototype as the default, and when a render fails with "not a function" it **re-runs that render once** against a diagnostic `Proxy` prototype, purely to produce the message. Errors are rare; a 30% tax on every render is not.

## Not there yet

- **Semantic state styles.** `gpui-base` has a `state_style` layer with a defined priority order for checked, selected and disabled. It is not bound; use `.when(condition, …)` for those states today.
- **Animation.** No transitions and no keyframes on the script surface.
- **Spacing and radius tokens in styles.** The palette carries spacing and radius scales, but style methods take lengths, not token names — only colours resolve a token. Applications define their own scale as a constant, the way the example's `SPACE` object does.
- **Theme switching from a script.** The runtime ships a light and a dark palette and a Rust API to switch between them (`gpui_shell::theme::set_mode`); there is no `gpui.set_theme` on the script surface yet.
