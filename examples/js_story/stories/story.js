// The gallery imports only public script modules. Registered surfaces render
// through gpui-component; infrastructure entries remain honest status panels.
import { div } from "gpui";
import { v_flex } from "gpui-base";
import { coveredSurfaces } from "./coverage.js";
import { registeredExamples } from "./registered.js";
import { surfaceStatus } from "./status.js";

/**
 * @param {StoryDefinition} story
 * @returns {StoryRoute}
 */
export function pendingStory(story) {
  const surfaces = coveredSurfaces(story.id);
  const states = surfaces.map(surfaceStatus);
  if (
    states.length > 0 &&
    states.every((state) => state?.status === "registered")
  ) {
    return {
      ...story,
      availability: "registered",
      render: (cx) => registeredPanel(story, surfaces, cx),
    };
  }
  return {
    ...story,
    availability: "infrastructure",
    render: (cx) => availabilityPanel(story, cx),
  };
}

/** @param {StoryDefinition} story @param {string[]} surfaces @param {import("gpui").Context} cx */
function registeredPanel(story, surfaces, cx) {
  const colors = cx.theme().colors;
  return v_flex()
    .id(`story-${story.id}`)
    .w_full()
    .max_w(760)
    .gap(16)
    .p(24)
    .bg(colors.surface)
    .border(1)
    .border_color(colors.border)
    .rounded(8)
    .child(
      div()
        .text_size(18)
        .font_semibold()
        .text_color(colors.foreground)
        .child("Registered public surface"),
    )
    .child(
      div()
        .text_size(13)
        .text_color(colors.muted_foreground)
        .child(
          "Each case below is built from public constructors and descriptor methods.",
        ),
    )
    .children(
      surfaces.flatMap((surface) =>
        registeredExamples(surface).map((example) => case_(example, cx)),
      ),
    );
}

/**
 * One labelled example. The caption says what the case is showing, so a reader
 * comparing two cases can tell which knob differs without reading the source.
 * @param {{ label: string, element: unknown }} example
 * @param {import("gpui").Context} cx
 */
function case_(example, cx) {
  const colors = cx.theme().colors;
  return v_flex()
    .w_full()
    .gap(8)
    .child(
      div()
        .text_size(11)
        .font_semibold()
        .text_color(colors.muted_foreground)
        .child(example.label),
    )
    .child(
      div()
        .w_full()
        .p(16)
        .bg(colors.background)
        .border(1)
        .border_color(colors.border)
        .rounded(6)
        .child(
          /** @type {import("gpui").Element} */ (
            /** @type {unknown} */ (example.element)
          ),
        ),
    );
}

/** @param {StoryDefinition} story @param {import("gpui").Context} cx */
function availabilityPanel(story, cx) {
  const colors = cx.theme().colors;

  return v_flex()
    .id(`story-${story.id}`)
    .w_full()
    .max_w(760)
    .gap(16)
    .p(24)
    .bg(colors.surface)
    .border(1)
    .border_color(colors.border)
    .rounded(8)
    .child(
      div()
        .text_size(18)
        .font_semibold()
        .text_color(colors.foreground)
        .child("Infrastructure coverage"),
    )
    .child(
      div()
        .text_size(13)
        .text_color(colors.muted_foreground)
        .child(
          "This Story route documents a non-renderable inventory entry. It is exercised through the controls that consume it, not through a fabricated constructor.",
        ),
    )
    .child(
      div()
        .px(12)
        .py(8)
        .bg(colors.muted)
        .rounded(6)
        .text_size(12)
        .text_color(colors.foreground)
        .child(`Inventory scope: ${story.api}`),
    )
    .child(
      v_flex()
        .gap(6)
        .child(
          div()
            .text_size(12)
            .font_semibold()
            .text_color(colors.foreground)
            .child("Planned examples"),
        )
        .children(
          story.states.map((state) =>
            div()
              .text_size(12)
              .text_color(colors.muted_foreground)
              .child(`• ${state}`),
          ),
        ),
    );
}

/** @typedef {import("../catalog.js").StoryRoute} StoryRoute */
/** @typedef {Omit<StoryRoute, "availability" | "render"> & { availability?: string }} StoryDefinition */
