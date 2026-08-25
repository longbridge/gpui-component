// The JavaScript half of the "Shell" story.
//
//   cargo run   →   Gallery   →   Shell
//
// This file is read from disk when the story opens and again every time the
// "Reload script" button next to this panel is pressed. Editing it changes what
// the right-hand side of that window draws, with no `cargo build` in between —
// which is the entire argument for a script layer.
//
// It owns no state. The board lives in a Rust `Entity<Market>` that the panel on
// the left renders directly; this file reaches it through the two native
// modules `shell_story.rs` registered before the runtime started:
//
//   native("market")   quotes() · ticks() · watch(symbol) · watch_all(on)
//   native("theme")    palette()
//
// Both are plain data in, plain data out. There is no host object to hold on
// to here: a native call takes numbers, strings and records and gives them
// back, so nothing this file writes can outlive the call that produced it.
//
// # What this panel is for
//
// It is a load, not a demonstration of features. Twelve rows of six cells each,
// rebuilt from scratch every time a price moves — which, with the feed running
// at 50 ms, is twenty times a second. The counters under the two panels report
// what that costs, and what a repaint costs when nothing here has changed.

import { h_flex, v_flex, View, native } from "gpui";
import {
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
    // Read once per render; see `refreshPalette` for why it is not cached.
    refreshPalette();

    const market = native("market");
    const quotes = market.quotes();
    const watched = quotes.filter((quote) => quote.watched).length;

    // The tick count. Painting it is what makes the counters' claim visible in
    // the panel itself: with the quotes feed running this number climbs, and
    // with the repaint feed running it holds still — because this render is not
    // being called at all, and the frame being drawn is the one it produced
    // last time.
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
      .gap(SPACE.sm)
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

  rows(market, quotes) {
    if (quotes.length === 0) {
      return muted("The Rust panel is holding no quotes.");
    }

    return v_flex()
      .w_full()
      .gap(SPACE.xxs)
      .children(
        quotes.map((quote) =>
          // Deliberately no `cx.notify()`. The native call asks Rust to change
          // the board, and the Rust side notifies its observers — which
          // re-renders this view *and* the panel next to it. One change, one
          // notification, both halves in step.
          quoteRow(quote, () => market.watch(quote.symbol)),
        ),
      );
  }

  actions(market, total, watched) {
    return h_flex()
      .w_full()
      .items_center()
      .justify_between()
      .gap(SPACE.sm)
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
