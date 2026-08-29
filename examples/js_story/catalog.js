// Keep every import explicit. A missing family module is a load error instead
// of a silently incomplete gallery, which makes this the reviewable inventory.
import { stories as foundations } from "./stories/foundations.js";
import { stories as actions } from "./stories/actions.js";
import { stories as inputs } from "./stories/inputs.js";
import { stories as navigation } from "./stories/navigation.js";
import { stories as content } from "./stories/content.js";
import { stories as overlays } from "./stories/overlays.js";
import { stories as collections } from "./stories/collections.js";
import { stories as layouts } from "./stories/layouts.js";

/** The complete JavaScript Story route manifest, in Rust Story display order. */
const byRustStory = [
  ...foundations,
  ...actions,
  ...inputs,
  ...navigation,
  ...content,
  ...overlays,
  ...collections,
  ...layouts,
];

// Preserve the order in `crates/story/src/gallery.rs`, even though the source
// files are grouped by family for maintenance. The explicit list is also a
// second audit: adding a family entry without making it reachable is rejected
// during module load.
const RUST_STORY_ORDER = [
  "WelcomeStory",
  "AccordionStory",
  "AlertStory",
  "AlertDialogStory",
  "AvatarStory",
  "BadgeStory",
  "BreadcrumbStory",
  "ButtonStory",
  "CalendarStory",
  "ChartStory",
  "CheckboxStory",
  "ClipboardStory",
  "CollapsibleStory",
  "ColorPickerStory",
  "ComboboxStory",
  "CommandStory",
  "DataTableStory",
  "DatePickerStory",
  "DescriptionListStory",
  "DialogStory",
  "DockStory",
  "DropdownButtonStory",
  "EditorStory",
  "FormStory",
  "GroupBoxStory",
  "HoverCardStory",
  "IconStory",
  "ImageStory",
  "InputStory",
  "KbdStory",
  "LabelStory",
  "ListStory",
  "MenuStory",
  "NativeMenuStory",
  "NotificationStory",
  "NumberInputStory",
  "OtpInputStory",
  "PaginationStory",
  "PopoverStory",
  "ProgressStory",
  "RadioStory",
  "RatingStory",
  "ResizableStory",
  "ScrollbarStory",
  "SelectStory",
  "SeparatorStory",
  "SettingsStory",
  "ShellStory",
  "SheetStory",
  "SidebarStory",
  "SkeletonStory",
  "SliderStory",
  "SpinnerStory",
  "StatusBarStory",
  "StepperStory",
  "SwitchStory",
  "TableStory",
  "TabsStory",
  "TagStory",
  "TextareaStory",
  "ThemeColorsStory",
  "ToggleStory",
  "TooltipStory",
  "TreeStory",
  "VirtualListStory",
];

/** The complete JavaScript Story route manifest, in Rust Story display order. */
export const catalog = RUST_STORY_ORDER.map((rustStory) => {
  const story = byRustStory.find(
    (candidate) => candidate.rustStory === rustStory,
  );
  if (!story)
    throw new Error(`JavaScript Story catalog is missing ${rustStory}`);
  return story;
});

// Inventory registrations are sometimes implementation helpers rather than a
// one-to-one Rust Story (for example Plot is exercised by Chart). Keep those
// relationships explicit instead of silently treating the Story subset as the
// whole component catalog. verify-coverage.mjs checks this map against every
// renderable/platform registration in component-inventory.json.
export const coveredBy = [
  { route: "introduction", registrations: ["Welcome", "Link"] },
  { route: "accordion", registrations: ["Accordion"] },
  { route: "alert", registrations: ["Alert"] },
  { route: "alert-dialog", registrations: ["AlertDialog"] },
  { route: "avatar", registrations: ["Avatar"] },
  { route: "badge", registrations: ["Badge"] },
  { route: "breadcrumb", registrations: ["Breadcrumb"] },
  { route: "button", registrations: ["Button"] },
  { route: "calendar", registrations: ["Calendar"] },
  { route: "chart", registrations: ["Chart", "Plot"] },
  { route: "checkbox", registrations: ["Checkbox"] },
  { route: "clipboard", registrations: ["Clipboard"] },
  { route: "collapsible", registrations: ["Collapsible"] },
  { route: "color-picker", registrations: ["ColorPicker"] },
  { route: "combobox", registrations: ["Combobox"] },
  { route: "command", registrations: ["Command"] },
  { route: "data-table", registrations: ["DataTable"] },
  { route: "date-picker", registrations: ["DatePicker"] },
  { route: "description-list", registrations: ["DescriptionList"] },
  { route: "dialog", registrations: ["Dialog"] },
  { route: "dock", registrations: ["Dock"] },
  { route: "dropdown-button", registrations: ["DropdownButton"] },
  { route: "editor", registrations: ["Editor", "Text"] },
  { route: "form", registrations: ["Form"] },
  { route: "group-box", registrations: ["GroupBox"] },
  { route: "hover-card", registrations: ["HoverCard"] },
  { route: "icon", registrations: ["Icon"] },
  { route: "image", registrations: ["Image"] },
  { route: "input", registrations: ["Input"] },
  { route: "kbd", registrations: ["Kbd"] },
  { route: "label", registrations: ["Label"] },
  { route: "list", registrations: ["List", "SearchableList"] },
  { route: "menu", registrations: ["Menu"] },
  { route: "native-menu", registrations: ["NativeMenu"] },
  { route: "notification", registrations: ["Notification"] },
  { route: "number-input", registrations: ["NumberInput"] },
  { route: "otp-input", registrations: ["OtpInput"] },
  { route: "pagination", registrations: ["Pagination"] },
  { route: "popover", registrations: ["Popover"] },
  { route: "progress", registrations: ["Progress"] },
  { route: "radio", registrations: ["Radio"] },
  { route: "rating", registrations: ["Rating"] },
  { route: "resizable", registrations: ["Resizable"] },
  { route: "scrollbar", registrations: ["Scrollbar", "Scroll"] },
  { route: "select", registrations: ["Select"] },
  { route: "separator", registrations: ["Separator"] },
  { route: "settings", registrations: ["Settings", "Setting"] },
  { route: "shell", registrations: ["Shell"] },
  { route: "sheet", registrations: ["Sheet"] },
  { route: "sidebar", registrations: ["Sidebar"] },
  { route: "skeleton", registrations: ["Skeleton"] },
  { route: "slider", registrations: ["Slider"] },
  { route: "spinner", registrations: ["Spinner"] },
  { route: "status-bar", registrations: ["StatusBar"] },
  { route: "stepper", registrations: ["Stepper"] },
  { route: "switch", registrations: ["Switch"] },
  { route: "table", registrations: ["Table"] },
  { route: "tabs", registrations: ["Tabs", "Tab"] },
  { route: "tag", registrations: ["Tag"] },
  { route: "textarea", registrations: ["Textarea"] },
  { route: "theme-colors", registrations: [] },
  { route: "toggle", registrations: ["Toggle"] },
  { route: "tooltip", registrations: ["Tooltip"] },
  { route: "tree", registrations: ["Tree"] },
  { route: "virtual-list", registrations: ["VirtualList"] },
];

/** @type {Map<string, StoryRoute>} */
export const routesById = new Map(catalog.map((story) => [story.id, story]));

if (routesById.size !== catalog.length) {
  throw new Error("JavaScript Story catalog contains duplicate route ids");
}
if (byRustStory.length !== catalog.length) {
  throw new Error(
    "JavaScript Story catalog has a route missing from the Rust Story order",
  );
}
if (coveredBy.some((entry) => !routesById.has(entry.route))) {
  throw new Error("JavaScript Story coverage references an unknown route");
}

/** @param {string} id */
export function route(id) {
  return routesById.get(id) ?? catalog[0];
}

/** @param {string} query */
export function filterCatalog(query) {
  const needle = query.trim().toLowerCase();
  if (needle === "") return catalog;
  return catalog.filter((story) =>
    [story.title, story.group, story.rustStory, story.id].some((value) =>
      value.toLowerCase().includes(needle),
    ),
  );
}

/**
 * @typedef {object} StoryRoute
 * @property {string} id Stable kebab-case route identifier.
 * @property {string} title Rust Story display title.
 * @property {string} group Sidebar family.
 * @property {string} rustStory Source `Story` implementation in crates/story.
 * @property {string} description
 * @property {string[]} states Examples to provide once the binding is available.
 * @property {"pending" | "platform" | "infrastructure"} availability
 * @property {string} api The expected public gpui-component export.
 * @property {(cx: import("gpui").Context) => import("gpui").Element} render
 */
