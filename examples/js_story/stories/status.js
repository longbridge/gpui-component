// Script-side projection of the checked-in component inventory. Gallery code
// cannot import repository JSON at runtime; verify-coverage.mjs rejects drift.
export const REGISTERED_SURFACES = [
  "Accordion",
  "Alert",
  "AlertDialog",
  "Avatar",
  "BarChart",
  "Badge",
  "Breadcrumb",
  "Button",
  "Calendar",
  "Checkbox",
  "Clipboard",
  "Collapsible",
  "ColorPicker",
  "Combobox",
  "Command",
  "DatePicker",
  "DataTable",
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
  "List",
  "MenuBar",
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
  "Select",
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
  "TabBar",
  "Table",
  "Text",
  "Textarea",
  "Stepper",
  "Toggle",
  "Tooltip",
  "Tree",
];

/** @type {Record<string, string>} */
export const DEFERRED_SURFACES = {};

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
