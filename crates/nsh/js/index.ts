/** Compatibility entry; NSH generates component exports and types from Rust. */
export * from "nsh/ui/gpui";
export { Slot, type CompositionProps } from "nsh/native";
export type {
  MenuItemValue as MenuItem,
  MenuItemValue as PopupMenuItem,
  MenuItemValue as DropdownButtonMenuItem,
} from "nsh/ui/gpui";

import type * as Native from "nsh/ui/gpui";

/** Historical convenience types, derived from the generated component props. */
type EventsOf<Props> = {
  [Key in keyof Props as Key extends `on${infer Name}` ? Uncapitalize<Name> : never]-?:
    NonNullable<Props[Key]> extends (...args: infer Args) => unknown
      ? Args extends [] ? void : Args[0]
      : never;
};
export type SizedProps = Pick<Native.ButtonProps, "size">;
export type FocusProps = Pick<Native.ButtonProps, "disabled" | "tabStop" | "tabIndex" | "accessibilityLabel" | "tooltip">;
export type ShimmerProps = Pick<Native.MarkerProps, "duration" | "spread" | "reverse" | "once" | "highlightColor">;
export type EditingProps = Native.TextareaProps;
export type EditingEvents = Pick<EventsOf<Native.TextareaProps>, "change" | "focus" | "blur" | "submit" | "error">;
export type SpeechProps = Pick<Native.SpeechButtonProps, "session" | "disabled" | "showWhenUnsupported">;
export type SpeechEvents = Pick<EventsOf<Native.SpeechButtonProps>, "started" | "partial" | "final" | "cancelled" | "error">;
export type ColorControlProps = Pick<Native.ColorPickerProps, "value" | "open" | "activeTab" | "size" | "label" | "accessibilityLabel" | "featuredColors">;
export type ColorEvents = Pick<EventsOf<Native.ColorPickerProps>, "change" | "hslaChange" | "openChange" | "activeTabChange" | "snapshot" | "error">;
export type ChartProps = Pick<Native.LineChartProps, "data" | "interactive" | "appear">;
