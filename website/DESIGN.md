# Website Design

Design rules for the documentation site in `website/`. It exists so later
changes stay coherent instead of drifting back into a generic template.

The site is the first thing a visitor sees of a library that renders native
desktop UI. So it has one job before anything else: make it obvious, in a
single screen, what this project is — and prove it is real.

## Principles

1. **Say what it is, immediately.** The headline states the product in plain
   language ("Build native desktop apps in Rust."). Positioning that only makes
   sense to someone who already knows the project — architecture layers,
   "ship fast or own everything" — belongs further down the page, not in the
   hero.
2. **Show the real thing, never a mock.** The landing page embeds the actual
   WebAssembly build of the gallery. No greyed-out wireframe placeholders, no
   invented product screenshots. A fake UI panel on the homepage of a UI
   library reads as an admission that there is nothing to show, and a
   convincing one gets mistaken for a product the project does not ship.
3. **Verifiable facts over adjectives.** Star count, licence, platforms,
   component count, real type names (`DockArea`, `Rope`, `FocusHandle`). A
   developer judges credibility from specifics.
4. **Same palette as the library.** Every colour comes from
   `crates/ui/src/theme/default-theme.json`. The site must not invent a brand
   colour that the components cannot produce.
5. **Restraint carries the design.** Hierarchy comes from type scale, weight
   and hairlines — not from colour fills or decorative gradients.

## Colour

Defined in `.vitepress/theme/style.css`, mapped from the default theme so the
site and the documented components share one palette.

| Token | Light | Dark | Theme source |
| --- | --- | --- | --- |
| `--background` | `#ffffff` | `#0a0a0a` | `background` |
| `--foreground` | `#0a0a0a` | `#fafafa` | `foreground` |
| `--border` | `#e5e5e5` | `#262626` | `border` |
| `--secondary` | `#f5f5f5` | `#262626` | `secondary.background` |
| `--muted-foreground` | `#737373` | `#a3a3a3` | `muted.foreground` |
| `--sidebar` | `#fafafa` | `#0f0f0f` | `sidebar.background` |
| `--titlebar` | `#f8f8f8` | `#171717` | `title_bar.background` |
| `--brand` | `#171717` | `#fafafa` | `primary.background` |
| `--data-1…5` | `#93c5fd` → `#1e40af` | same | `chart_1…chart_5` |
| `--selection` | `#55a0fc` | same | `selection.background` |
| `--success` | `#22c55e` | same | `success.background` |

Rules that follow from this:

- **The brand colour is near-black (near-white in dark mode).** It is used for
  primary buttons, focus rings and the active sidebar indicator — never as an
  "accent" to add interest, because it is the same value as body text.
- **Never use `--brand` as a background behind text you did not also invert.**
  Text selection in particular uses `--selection`, not the brand: black text on
  a near-black selection is unreadable.
- **Saturated colour is reserved for data**, exactly as the theme reserves
  `chart_*`. Charts and syntax highlighting may use `--data-*`; marketing
  surfaces may not.
- **`--success` marks "running"**, such as the live WASM indicator, where
  near-black would not read as a live signal.
- Links are distinguished by a rule, not a hue, since the brand colour equals
  the text colour. See `.vp-doc a`.

## Typography

`Geist Variable` for text, `JetBrains Mono Variable` for code, labels and
numerals. Base size 15px.

- **Display** — `clamp(2.2rem, 4.3vw, 3.6rem)`, weight 660, tracking `-0.042em`.
- **Section heading** — `clamp(2rem, 3.6vw, 3rem)`, weight 660, tracking `-0.045em`.
- **Body** — 1rem, line-height 1.7.
- **Kicker / label** — 0.68rem mono, uppercase, tracking `0.1em`, muted. Small
  mono labels, not colour, mark structure.

Two constraints that are easy to get wrong:

- **Negative tracking is for Latin only.** CJK glyphs sit on a fixed em grid,
  so `html[lang^="zh"]` resets `letter-spacing` to normal. Do not apply tight
  tracking globally.
- **Numerals use `tabular-nums`** in tables, keys and counters so they do not
  reflow when values change.

## Surfaces and the window language

Anything showing the library running is framed as a **macOS window** — the
`.mac-window` class in `style.css`. It is the closest visual analogue to what
the library actually produces, so it reads as a native application rather than
a screenshot card.

The frame is: a hairline outer stroke, an inner top highlight, layered soft
shadows, and real traffic lights (`#ff5f57`, `#febc2e`, `#28c840`). The title
is centred and independent of the lights, as macOS does.

Used by the landing page gallery and by `ComponentExample.vue` on every
component page, so the two never diverge.

**Do not put document tabs inside the window chrome.** A tab strip in the
titlebar fights the traffic lights, and a browser-style tab row below it is not
how gpui-component presents views. View switching uses the library's own
**segmented control** (`.segmented`, mapped from `tab_bar.segmented.background`
and `tab.active.background`), placed in the section heading — outside the
window, which stays pure chrome.

Radii: `--radius-control` 0.375rem for controls, `--radius-card` 0.625rem for
cards, `--radius-surface` 0.875rem for large surfaces, 0.75rem for windows.

## Layout

- Page width `1280px`, gutter `1.5rem` (`1rem` under 640px).
- Navigation is a **toolbar**: brand and sections grouped on the left behind a
  hairline divider, controls collected on the right. Height 3.5rem/56px on both
  the landing page and the docs, kept in sync via `--vp-nav-height`.
- Section rhythm `clamp(4.5rem, 8vw, 7rem)`.
- The hero is a single left-aligned column. It is deliberately compact so the
  live gallery window is already visible at the fold — the real component is
  the strongest thing on the page, and it should not sit below empty space.
- Grid backgrounds are hairline blueprint grids masked to fade at the edges. No
  colour wash behind headlines.

## Motion

Entrances only, and short: `rise` at 620ms on `cubic-bezier(.16, 1, .3, 1)`,
staggered 70ms. Live indicators use a slow 2.4s pulse. Everything sits inside
`@media (prefers-reduced-motion: no-preference)`; nothing conveys meaning
through motion alone.

## Content rules

- The crate is **not published**. Installation must show the git dependency,
  never `cargo add`, and the UI must not display a version number.
- Code samples on the landing page must be real API. Verify against
  `crates/ui` and `crates/base` before adding a snippet.
- Landing-page copy lives in one bilingual `copy` object in `index.vue`. Both
  locales must be updated together, matching the site-wide rule that
  `website/docs/` and `website/zh-CN/docs/` stay in sync.
- Gallery demos should be the components that demonstrate density and
  capability — data tables, charts, lists, sidebars. A demo that looks trivial
  or unstyled undersells the library and should be swapped out.

## Files

| File | Role |
| --- | --- |
| `.vitepress/theme/style.css` | Tokens, `.mac-window`, VitePress overrides, doc typography |
| `index.vue` | Landing page: markup, bilingual copy, page-scoped styles |
| `.vitepress/theme/components/ComponentExample.vue` | Windowed live example on component pages |
| `.vitepress/config.mts` | Navigation, sidebar generation, locales |
