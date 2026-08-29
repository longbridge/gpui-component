// The gallery itself only imports public script modules. This fallback is
// intentional: component constructors move into gpui-component-shell in the
// parallel adapter task, and a route must remain executable before that lands.
import { div } from "gpui";
import { v_flex } from "gpui-base";

/**
 * @param {Omit<StoryRoute, "render">} story
 * @returns {StoryRoute}
 */
export function pendingStory(story) {
  return {
    ...story,
    render: (cx) => availabilityPanel(story, cx),
  };
}

/** @param {Omit<StoryRoute, "render">} story @param {import("gpui").Context} cx */
function availabilityPanel(story, cx) {
  const colors = cx.theme().colors;
  const platformOnly = story.availability === "platform";
  const infrastructure = story.availability === "infrastructure";
  const heading = infrastructure
    ? "Infrastructure coverage"
    : platformOnly
      ? "Platform availability"
      : "Binding not registered yet";
  const detail = infrastructure
    ? "This Story route documents a non-renderable inventory entry. It is exercised through the controls that consume it, not through a fabricated constructor."
    : platformOnly
      ? "This control has a platform-specific implementation. Its route remains in the catalog so coverage is auditable on every host."
      : `The public gpui-component export ${story.api} will be exercised here when gpui-component-shell registers it.`;

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
        .child(heading),
    )
    .child(
      div().text_size(13).text_color(colors.muted_foreground).child(detail),
    )
    .child(
      div()
        .px(12)
        .py(8)
        .bg(colors.muted)
        .rounded(6)
        .text_size(12)
        .text_color(colors.foreground)
        .child(`Public API: ${story.api}`),
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
