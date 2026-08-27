// The JavaScript half of the "Shell" story.
//
//   cargo run   →   Gallery   →   Shell
//
// Read from disk when the story opens, and again on "Reload script". Editing
// this file changes the right-hand panel with no `cargo build` in between,
// which is the entire argument for a script layer.
//
// It owns no state. The board lives in a Rust `Entity<Market>`, reached through
// the native module the story registered before the runtime started:
//
//   native("market")   quotes() · ticks() · watch(symbol) · watch_all(on)
//
// Twenty rows of six cells, rebuilt from scratch every time a price moves —
// twenty times a second with the default feed. The counters under the panels
// report what that costs, and what a repaint costs when nothing here changed.

import { View, native } from "gpui";
import { h_flex, v_flex } from "gpui-base";
/** @import { NativeModules } from "gpui" */
import {
  ROW,
  SPACE,
  action,
  header,
  label,
  muted,
  quoteRow,
  rule,
  surface,
  title,
} from "./ui.js";

export default class QuoteBoard extends View {
  render(cx) {
    const market = native("market");
    const quotes = market.quotes();
    const watched = quotes.filter((quote) => quote.watched).length;

    // With the quotes feed running this climbs; with the repaint feed it holds
    // still, because this render is not being called and the frame on screen is
    // the one it produced last time.
    const ticks = market.ticks();

    return surface()
      .child(this.heading(quotes.length, watched, ticks, cx))
      .child(header(cx))
      .child(this.rows(market, quotes, cx))
      .child(rule(cx))
      .child(this.actions(market, quotes.length, watched, cx));
  }

  /**
   * @param {number} total
   * @param {number} watched
   * @param {number} ticks
   * @param {import("gpui").Context} cx
   */
  heading(total, watched, ticks, cx) {
    return h_flex()
      .w_full()
      .items_start()
      .justify_between()
      .gap(ROW.inset)
      .child(
        v_flex()
          .gap(SPACE.xxs)
          .child(title("Live quotes", cx))
          .child(muted("Drawn by main.js · prices read over native(\"market\")", cx)),
      )
      .child(
        v_flex()
          .items_end()
          .gap(SPACE.xxs)
          .child(label(`${watched} / ${total} watched`, cx))
          .child(muted(`tick ${ticks}`, cx)),
      );
  }

  /**
   * @param {NativeModules["market"]} market
   * @param {Quote[]} quotes
   * @param {import("gpui").Context} cx
   */
  rows(market, quotes, cx) {
    if (quotes.length === 0) {
      return muted("The Rust panel is holding no quotes.", cx);
    }

    return v_flex()
      .w_full()
      .gap(ROW.gap)
      .children(
        // No `cx.notify()`: the native call asks Rust to change the board, Rust
        // notifies its observers, and both halves re-render from one change.
        quotes.map((quote) => quoteRow(quote, () => market.watch(quote.symbol), cx)),
      );
  }

  /**
   * @param {NativeModules["market"]} market
   * @param {number} total
   * @param {number} watched
   * @param {import("gpui").Context} cx
   */
  actions(market, total, watched, cx) {
    return h_flex()
      .w_full()
      .items_center()
      .justify_between()
      .gap(ROW.inset)
      // The heading already carries "N / M watched". Only the empty case says
      // something it does not.
      .child(muted(watched === 0 ? "Nothing on the watchlist" : "", cx))
      .child(
        h_flex()
          .gap(SPACE.xs)
          .child(
            action("watch-all", "Watch all", () => market.watch_all(true), cx, {
              primary: true,
              disabled: watched === total,
            }),
          )
          .child(
            action("watch-none", "Clear", () => market.watch_all(false), cx, {
              disabled: watched === 0,
            }),
          ),
      );
  }
}
