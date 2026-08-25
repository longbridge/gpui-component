// The native modules `shell_story.rs` registered, as TypeScript declarations.
//
// Hand-written, because only the host knows what it granted: `gpui.d.ts` is
// generated from the runtime's own tables, and the runtime has no idea this
// story exists. Augmenting `NativeModules` is what turns
//
//   native("market")           →  Record<string, (...args: any[]) => any>
//
// into a checked module name whose functions complete and whose records have
// fields. A wrong name, a wrong argument or a misspelled field is an editor
// error here rather than an exception at run time.
//
// Keep it in step with `install_native_modules` in `shell_story.rs`. Nothing
// enforces that — it is two languages describing one boundary — which is why
// both sides are small and sit beside each other in the same commit.

/** One row of the board, as it crosses the boundary. */
interface Quote {
  symbol: string;
  name: string;
  /** Already formatted by Rust, so both halves round the same way. */
  last: string;
  change: string;
  percent: string;
  volume: string;
  /** 1 up, -1 down, 0 unchanged. */
  direction: number;
  watched: boolean;
}

/**
 * The gallery's own theme, as colors a script can paint with.
 *
 * The colors are `` `#${string}` `` rather than `string`, which is what the
 * `gpui` module's own `Color` accepts. Typing them as plain strings compiles
 * here and then fails at every call site that paints with one — the kind of
 * mismatch these declarations exist to surface at the boundary rather than
 * twenty lines later.
 */
interface Palette {
  background: Hex;
  foreground: Hex;
  muted: Hex;
  muted_foreground: Hex;
  border: Hex;
  primary: Hex;
  primary_hover: Hex;
  primary_foreground: Hex;
  secondary: Hex;
  accent: Hex;
  success: Hex;
  danger: Hex;
  radius: number;
  font_size: number;
}

/** What `to_hex` produces on the Rust side. */
type Hex = `#${string}`;

declare module "gpui" {
  interface NativeModules {
    market: {
      /** Every row on the board. */
      quotes(): Quote[];
      /** How many feed ticks have landed. */
      ticks(): number;
      /** Flips one row's watched flag and answers the new value. */
      watch(symbol: string): boolean;
      /** Sets every row, and answers how many actually moved. */
      watch_all(watched: boolean): number;
    };
    theme: {
      palette(): Palette;
    };
  }
}
