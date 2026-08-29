// Script-side projection of the checked-in component inventory. Gallery code
// cannot import repository JSON at runtime; verify-coverage.mjs rejects drift.
export const REGISTERED_SURFACES = [
  "Accordion",
  "Alert",
  "AlertDialog",
  "Avatar",
  "Badge",
  "Breadcrumb",
  "Button",
  "Calendar",
  "Checkbox",
  "Clipboard",
  "Collapsible",
  "ColorPicker",
  "Command",
  "DatePicker",
  "DescriptionList",
  "Dialog",
  "DropdownButton",
  "Editor",
  "Form",
  "GroupBox",
  "HoverCard",
  "Icon",
  "Image",
  "Input",
  "Kbd",
  "Label",
  "Link",
  "NumberInput",
  "NativeMenuTrigger",
  "Notification",
  "OtpInput",
  "Pagination",
  "Popover",
  "Progress",
  "Radio",
  "Rating",
  "Resizable",
  "Scroll",
  "Scrollbar",
  "Separator",
  "Sheet",
  "Settings",
  "Sidebar",
  "Skeleton",
  "Slider",
  "Spinner",
  "StatusBar",
  "Switch",
  "Tag",
  "Tab",
  "Tabs",
  "Table",
  "Text",
  "Textarea",
  "Stepper",
  "Toggle",
  "Tree",
];

/** @type {Record<string, string>} */
export const DEFERRED_SURFACES = {
  Chart: "chart-family",
  Combobox: "stateful-selection",
  DataTable: "stateful-collection",
  Dock: "dock-layout",
  List: "stateful-collection",
  Menu: "popup-menu-api",
  Plot: "plot-infrastructure",
  SearchableList: "stateful-collection",
  Select: "stateful-selection",
  Tooltip: "element-extension",
  VirtualList: "virtualized-collection",
};

/** @param {string} surface @returns {{ status: "registered" } | { surface: string, status: "deferred", category: string, reason: string } | null} */
export function surfaceStatus(surface) {
  if (REGISTERED_SURFACES.includes(surface)) return { status: "registered" };
  const category = DEFERRED_SURFACES[surface];
  if (!category) return null;
  return {
    surface,
    status: "deferred",
    category,
    reason: `No public ${surface} constructor is registered; the adapter defers this ${category} surface.`,
  };
}
