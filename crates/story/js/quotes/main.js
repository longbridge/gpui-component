// The JavaScript half of the "Shell" story.
//
//   cargo run   →   Gallery   →   Shell
//
// Read from disk when the story opens, and again on "Reload script". Editing
// this file changes the right-hand panel with no `cargo build` in between,
// which is the entire argument for a script layer.
//
// It owns no state. The board lives in a Rust `Entity<Market>`, reached through
// two native modules the story registered before the runtime started:
//
//   native("market")   quotes() · ticks() · watch(symbol) · watch_all(on)
//   native("theme")    palette()
//
// Twenty rows of six cells, rebuilt from scratch every time a price moves —
// twenty times a second with the default feed. The counters under the panels
// report what that costs, and what a repaint costs when nothing here changed.

import { h_flex, v_flex, View, native } from "gpui";
import {
  ROW,
  SPACE,
  action,
  header,
  label,
  muted,
  quoteRow,
  refreshPalette,
  rule,
  surface,
  title,
} from "./ui.js";

export default class QuoteBoard extends View {
  render() {
    refreshPalette();

    const market = native("market");
    const quotes = market.quotes();
    const watched = quotes.filter((quote) => quote.watched).length;

    // With the quotes feed running this climbs; with the repaint feed it holds
    // still, because this render is not being called and the frame on screen is
    // the one it produced last time.
    const ticks = market.ticks();

    return surface()
      .child(this.heading(quotes.length, watched, ticks))
      .child(header())
      .child(this.rows(market, quotes))
      .child(rule())
      .child(this.actions(market, quotes.length, watched));
  }

  heading(total, watched, ticks) {
    return h_flex()
      .w_full()
      .items_start()
      .justify_between()
      .gap(ROW.inset)
      .child(
        v_flex()
          .gap(SPACE.xxs)
          .child(title("Live quotes"))
          .child(muted("Drawn by main.js · prices read over native(\"market\")")),
      )
      .child(
        v_flex()
          .items_end()
          .gap(SPACE.xxs)
          .child(label(`${watched} / ${total} watched`))
          .child(muted(`tick ${ticks}`)),
      );
  }

  /** @param {Quote[]} quotes */
  rows(market, quotes) {
    if (quotes.length === 0) {
      return muted("The Rust panel is holding no quotes.");
    }

    return v_flex()
      .w_full()
      .gap(ROW.gap)
      .children(
        // No `cx.notify()`: the native call asks Rust to change the board, Rust
        // notifies its observers, and both halves re-render from one change.
        quotes.map((quote) => quoteRow(quote, () => market.watch(quote.symbol))),
      );
  }

  actions(market, total, watched) {
    return h_flex()
      .w_full()
      .items_center()
      .justify_between()
      .gap(ROW.inset)
      .child(muted(watched === 0 ? "Nothing on the watchlist" : `${watched} watched`))
      .child(
        h_flex()
          .gap(SPACE.xs)
          .child(
            action("watch-all", "Watch all", () => market.watch_all(true), {
              primary: true,
              disabled: watched === total,
            }),
          )
          .child(
            action("watch-none", "Clear", () => market.watch_all(false), {
              disabled: watched === 0,
            }),
          ),
      );
  }
}
