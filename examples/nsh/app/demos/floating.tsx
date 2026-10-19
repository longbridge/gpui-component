// The gallery's demo of Popover, HoverCard and Tooltip. Each is its trigger
// here; GPUI Base places the floating content beside it in the window,
// flipping or clamping it to stay inside, and dismisses it.

import { signal, View } from "nsh";
import { HoverCard, Popover, ToggleGroup, Tooltip } from "../../../../crates/nsh/types/gpui-base";
import { accent, button, ring, segmented, segment, log, Section } from "../look";

// GPUI Kit's floating look: a bordered surface with a shadow.
const surface = "w_72 p_3 flex flex_col gap_1 rounded_lg border_1 border_border bg_background shadow_lg";
const title = "text_sm font_semibold";
const description = "text_sm text_muted_foreground";
/** GPUI Kit's tooltip: small text on a bordered surface. */
const tooltip = "px_2 py_0p5 rounded_md border_1 border_border bg_background shadow_md text_sm";
/** An arrow in the surface's colors, its base clear of the `rounded_lg` corners. */
const arrow = { arrowSize: 6, arrowCornerRadius: 8, arrowClass: "bg_background border_border" };
const triggerStates = { triggerHoverClass: "bg_accent", triggerActiveClass: "bg_muted", triggerFocusClass: ring };
/** A link-like trigger, as GPUI Kit's hover cards use. */
const link = "text_sm text_primary";
const linkStates = { triggerHoverClass: "underline", triggerActiveClass: "opacity_80" };

const placements = ["bottom", "top", "left", "right"] as const;
const aligns = ["start", "center", "end"] as const;

const placement = signal(0);
const align = signal(0);
const open = signal(false);

export function FloatingDemo() {
  return (
    <View style={{ gap: 20 }}>
      <Section
        title="Popover"
        note="A click, Enter or Space opens it; Escape or a press outside closes it. Left and right are centered; the arrow aims at the anchored point."
      >
        <ToggleGroup
          items={["Bottom", "Top", "Left", "Right"]}
          value={placement()}
          onChange={(e) => placement.set(e.value)}
          rootClass={segmented}
          toggleClass={segment}
          toggleHoverClass="text_foreground"
          toggleFocusClass={ring}
          pressedClass="bg_background text_foreground"
        />
        <ToggleGroup
          items={["Start", "Center", "End"]}
          value={align()}
          onChange={(e) => align.set(e.value)}
          rootClass={segmented}
          toggleClass={segment}
          toggleHoverClass="text_foreground"
          toggleFocusClass={ring}
          pressedClass="bg_background text_foreground"
        />
        <Popover
          label="Dimensions"
          title="Dimensions"
          description="Set the width and height of the layer. Changes apply to the selection."
          open={open()}
          onOpenChange={(e) => {
            open.set(e.open);
            log("Popover.openChange", e);
          }}
          placement={placements[placement()]}
          align={aligns[align()]}
          offset={10}
          {...arrow}
          triggerClass={button}
          {...triggerStates}
          triggerOpenClass={accent}
          contentClass={surface}
          titleClass={title}
          descriptionClass={description}
        />
        <Popover
          label="Disabled"
          description="Never shown."
          disabled
          triggerClass={button}
          disabledClass="opacity_50"
        />
      </Section>
      <Section title="HoverCard" note="Rest the pointer on the name; the card stays while the pointer is over it.">
        <HoverCard
          label="@gpui-kit"
          title="GPUI Kit"
          description="Cross-platform desktop UI components for GPUI."
          onOpenChange={(e) => log("HoverCard.openChange", e)}
          placement="bottom"
          align="start"
          offset={10}
          {...arrow}
          openDelay={500}
          closeDelay={300}
          triggerClass={link}
          {...linkStates}
          contentClass={surface}
          titleClass={title}
          descriptionClass={description}
        />
        <HoverCard
          label="Above"
          description="Opens above its trigger, at once."
          onOpenChange={(e) => log("HoverCard.openChange", e)}
          placement="top"
          openDelay={0}
          triggerClass={link}
          {...linkStates}
          contentClass={surface}
          descriptionClass={description}
        />
      </Section>
      <Section title="Tooltip" note="Shows after the pointer rests on the trigger; a press hides it.">
        {placements.map((side) => (
          <Tooltip
            label={side[0].toUpperCase() + side.slice(1)}
            content={`A tooltip on the ${side}`}
            placement={side}
            onOpenChange={(e) => log("Tooltip.openChange", e)}
            triggerClass={button}
            triggerHoverClass="bg_accent"
            triggerActiveClass="bg_muted"
            contentClass={tooltip}
          />
        ))}
        <Tooltip
          label="No delay"
          content="Shown at once"
          delay={0}
          triggerClass={button}
          triggerHoverClass="bg_accent"
          triggerActiveClass="bg_muted"
          contentClass={tooltip}
        />
      </Section>
    </View>
  );
}
