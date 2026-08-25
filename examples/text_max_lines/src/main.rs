//! Clamp rendered Markdown to N whole lines with `TextView::max_lines`.
//!
//! - Drag the slider to change the line budget: the first card re-clamps and
//!   the clip always lands on a whole-line boundary (no half-cut glyphs).
//! - An Expand / Collapse button shows only while `is_clamped()` reports true
//!   (or the card is expanded).
//! - The second card fits within the cap, so it renders at natural height and
//!   no button shows.
//! - Resize the window: reflow changes where lines wrap, and the clip keeps
//!   snapping to whole lines.
//!
//! Run: `cargo run -p text_max_lines`

use gpui::{prelude::FluentBuilder as _, *};
use gpui_component::{
    ActiveTheme as _,
    button::Button,
    scroll::ScrollableElement as _,
    slider::{Slider, SliderEvent, SliderState},
    text::{TextView, TextViewState},
    *,
};
use gpui_component_assets::Assets;

const DEFAULT_MAX_LINES: usize = 5;

const LONG_MARKDOWN: &str = r#"### Quarterly summary

**Revenue** grew by *18%* quarter over quarter, driven by the desktop client
rollout and the new [market data](https://longbridge.com) subscriptions —
legacy plans are ~~discontinued~~ and folded into `pro`.

> The clip must land on a whole-line boundary: however you drag the slider,
> no line of glyphs is ever cut in half.

Reviewed by ![reviewer](https://avatars.githubusercontent.com/u/583231?v=4)
octocat together with ![bot](https://avatars.githubusercontent.com/u/5518?v=4)
the tooling team, inline within this very paragraph.

- Desktop DAU is up **24%**
  - macOS **+31%**, Windows *+19%*
  - Linux ships via `install.sh` now
- The `max_lines` preview lands in this release
- Churn stayed flat at 2.1%

| Segment | QoQ    | Note                 |
| ------- | ------ | -------------------- |
| Desktop | +24%   | new dock layout      |
| Mobile  | +9%    | steady               |
| Web     | -3%    | migrating to desktop |

![banner](https://avatars.githubusercontent.com/u/150917089?v=4)

---

```rust
fn main() {
    println!("hidden until expanded");
}
```

That table, banner image and code block only show once you press Expand — a
block that does not fit as a whole is hidden entirely rather than sliced."#;

const SHORT_MARKDOWN: &str = "A **short** note that fits within the cap, so no button shows.";

struct MaxLinesExample {
    long: Entity<TextViewState>,
    short: Entity<TextViewState>,
    slider: Entity<SliderState>,
    max_lines: usize,
    expanded: bool,
}

impl MaxLinesExample {
    fn new(cx: &mut Context<Self>) -> Self {
        let long = cx.new(|cx| TextViewState::markdown(LONG_MARKDOWN, cx));
        let short = cx.new(|cx| TextViewState::markdown(SHORT_MARKDOWN, cx));
        // `is_clamped` is written by TextView during draw; observe the states
        // so the Expand button shows up as soon as the flag flips.
        cx.observe(&long, |_, _, cx| cx.notify()).detach();
        cx.observe(&short, |_, _, cx| cx.notify()).detach();

        let slider = cx.new(|_| {
            SliderState::new()
                .min(1.)
                .max(20.)
                .step(1.)
                .default_value(DEFAULT_MAX_LINES as f32)
        });
        cx.subscribe(&slider, |this, _, event, cx| {
            if let SliderEvent::Change(value) = event {
                this.max_lines = value.start() as usize;
                cx.notify();
            }
        })
        .detach();

        Self {
            long,
            short,
            slider,
            max_lines: DEFAULT_MAX_LINES,
            expanded: false,
        }
    }

    fn card(&self, content: impl IntoElement, cx: &App) -> Div {
        v_flex()
            .max_w(px(480.))
            .p_3()
            .gap_2()
            .rounded(cx.theme().radius_lg)
            .bg(cx.theme().muted)
            .child(content)
    }
}

impl Render for MaxLinesExample {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let clamped = self.long.read(cx).is_clamped();
        let expanded = self.expanded;
        let max_lines = self.max_lines;

        v_flex()
            .size_full()
            .p_4()
            .gap_4()
            .child(
                h_flex()
                    .max_w(px(480.))
                    .gap_3()
                    .items_center()
                    .child(format!("max_lines: {max_lines}"))
                    .child(div().flex_1().child(Slider::new(&self.slider))),
            )
            .child(
                // The cards scroll while the slider stays pinned above.
                v_flex()
                    .flex_1()
                    .min_h_0()
                    .gap_4()
                    .child(
                        self.card(
                            TextView::new(&self.long)
                                .selectable(true)
                                .when(!expanded, |this| this.max_lines(max_lines)),
                            cx,
                        )
                        .when(clamped || expanded, |this| {
                            this.child(
                                h_flex().child(
                                    Button::new("toggle")
                                        .label(if expanded { "Collapse" } else { "Expand" })
                                        .on_click(cx.listener(|this, _, _, cx| {
                                            this.expanded = !this.expanded;
                                            cx.notify();
                                        })),
                                ),
                            )
                        }),
                    )
                    .child(
                        self.card(
                            TextView::new(&self.short)
                                .selectable(true)
                                .max_lines(max_lines),
                            cx,
                        ),
                    )
                    .overflow_y_scrollbar(),
            )
    }
}

fn main() {
    let app = gpui_platform::application().with_assets(Assets);

    app.run(move |cx| {
        // This must be called before using any GPUI Component features.
        gpui_component::init(cx);

        let window_options = WindowOptions {
            window_bounds: Some(WindowBounds::centered(size(px(680.), px(620.)), cx)),
            ..Default::default()
        };

        cx.spawn(async move |cx| {
            cx.open_window(window_options, |window, cx| {
                let view = cx.new(|cx| MaxLinesExample::new(cx));
                // The first level view on the window should be a Root.
                cx.new(|cx| Root::new(view, window, cx))
            })
            .expect("Failed to open window");
        })
        .detach();
    });
}
