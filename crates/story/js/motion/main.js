// A standalone native-motion ScriptView.  Every animated length target is a
// number, therefore an absolute pixel length that materialize::animate_length
// can sample on GPUI frames.
import { Button, View, div, h_flex, text, v_flex } from "gpui";
/** @import { Theme } from "gpui" */

const segment = (id, label, active, onClick, colors) =>
  Button.new(id)
    .selected(active)
    .h(30)
    .px(12)
    .rounded(colors.radius.sm)
    .border(1)
    .border_color(active ? colors.border : colors.muted)
    .bg(active ? colors.background : colors.muted)
    .hover((style) => style.bg(colors.secondary))
    .focus((style) => style.border_color(colors.foreground))
    .on_click(onClick)
    .child(text(active ? `${label} ✓` : label).text_size(12).font_medium().text_color(active ? colors.foreground : colors.muted_foreground));

const action = (label, active, onClick, colors) =>
  Button.new("motion-trigger")
    .h(36)
    .px(14)
    .rounded(colors.radius.md)
    .border(1)
    .border_color(colors.border)
    .bg(colors.background)
    .hover((style) => style.bg(colors.secondary))
    .focus((style) => style.border_color(colors.foreground))
    .on_click(onClick)
    .child(text(active ? "Send back" : label).text_size(12).font_medium().text_color(colors.foreground));

export default class MotionBoard extends View {
  init() {
    this.policy = "transition";
    this.active = false;
  }

  render(cx) {
    const colors = cx.theme();
    const spring = this.policy === "spring";
    const active = this.active;

    return v_flex()
      .w_full()
      .gap(12)
      .child(
        v_flex()
          .gap(4)
          .child(text("Native motion").text_size(14).font_semibold().text_color(colors.foreground))
          .child(
            text(
              spring
                ? "Spring samples pixel left, width, and opacity targets on native frames."
                : "Transition samples pixel left, width, and opacity targets on native frames.",
            )
              .text_size(12)
              .text_color(colors.muted_foreground),
          ),
      )
      .child(
        h_flex()
          .w_full()
          .items_center()
          .gap(12)
          .child(
            h_flex()
              .id("motion-policy-segment")
              .gap(2)
              .p(2)
              .rounded(colors.radius.md)
              .border(1)
              .border_color(colors.border)
              .bg(colors.muted)
              .child(segment("motion-transition", "Transition", !spring, (_, cx) => this.select("transition", cx), colors))
              .child(segment("motion-spring", "Spring", spring, (_, cx) => this.select("spring", cx), colors)),
          )
          .child(action("Run motion", active, (_, cx) => {
              this.active = !this.active;
              cx.notify();
            }, colors)),
      )
      .child(
        div()
          .relative()
          .w_full()
          .h(176)
          .rounded(colors.radius.md)
          .border(1)
          .border_color(colors.border)
          .bg(colors.muted)
          .children(this.track(colors))
          .child(this.runner(colors)),
      );
  }

  /** @param {"transition" | "spring"} policy @param {Context} cx */
  select(policy, cx) {
    this.policy = policy;
    cx.notify();
  }

  /** @param {Theme} colors */
  runner(colors) {
    const active = this.active;
    return this.motion(
      v_flex()
        .id("motion-runner")
        .absolute()
        .top(48)
        .left(active ? 960 : 20)
        .w(active ? 176 : 148)
        .h(80)
        .gap(8)
        .p(12)
        .rounded(colors.radius.md)
        .border(1)
        .border_color(colors.border)
        .bg(colors.background)
        .opacity(active ? 1 : 0.72)
        .child(
          h_flex()
            .w_full()
            .gap(6)
            .child(text("AAPL").text_size(12).font_semibold().text_color(colors.foreground))
            .child(div().flex_1())
            .child(div().w(6).h(6).rounded(3).bg(colors.accent))
            .child(text("Live").text_size(10).text_color(colors.accent)),
        )
        .child(
          h_flex()
            .w_full()
            .items_center()
            .gap(8)
            .child(text("$228.26").text_size(16).font_semibold().text_color(colors.foreground))
            .child(text("+1.84%").text_size(11).font_medium().text_color(colors.accent)),
        ),
    );
  }

  /** @param {Theme} colors */
  track(colors) {
    return [
      text("OPEN").absolute().top(18).left(20).text_size(9).text_color(colors.muted_foreground),
      text("LIVE TICK").absolute().top(18).left(960).text_size(9).text_color(colors.muted_foreground),
      div().absolute().top(88).left(20).w(1116).h(1).bg(colors.border),
      div().absolute().top(84).left(20).w(8).h(8).rounded(4).bg(colors.accent),
      div().absolute().top(84).left(1132).w(8).h(8).rounded(4).bg(colors.border),
      text("Native frames interpolate the card; JavaScript only changes its target.")
        .absolute()
        .top(144)
        .left(20)
        .text_size(10)
        .text_color(colors.muted_foreground),
    ];
  }

  /** @param {Element} element */
  motion(element) {
    if (this.policy === "spring") {
      return element
        .spring("left", { response: 360, damping: 0.72 })
        .spring("width", { response: 300, damping: 0.8 })
        .spring("opacity", { response: 220, damping: 1 });
    }
    return element
      .transition("left", { duration: 340, easing: "ease-in-out" })
      .transition("width", { duration: 260, easing: "ease-out" })
      .transition("opacity", { duration: 180, easing: "ease-out" });
  }
}
