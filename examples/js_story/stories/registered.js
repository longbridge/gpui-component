// These exports are public constructors from the current component-shell
// inventory. Constructor calls intentionally use `new`, matching the generated
// gpui-component declarations.
import { div } from "gpui";
import { h_flex, v_flex } from "gpui-base";
import {
  Accordion,
  AccordionItem,
  Alert,
  AlertDialog,
  Avatar,
  BarChart,
  Badge,
  Breadcrumb,
  Button,
  Calendar,
  CalendarState,
  Checkbox,
  Clipboard,
  Collapsible,
  ColorPicker,
  ColorPickerState,
  Combobox,
  Command,
  CommandGroup,
  CommandItem,
  CommandState,
  DatePicker,
  DatePickerState,
  DataTable,
  DataTableState,
  DescriptionItem,
  DescriptionList,
  Dialog,
  DropdownButton,
  DropdownMenu,
  Editor,
  EditorState,
  Field,
  Form,
  GroupBox,
  HoverCard,
  Icon,
  Image,
  Input,
  InputState,
  Kbd,
  Label,
  Link,
  List,
  Menu,
  MenuBar,
  MenuItem,
  MenuSeparator,
  NumberInput,
  NativeMenuItem,
  NativeMenuSeparator,
  NativeMenuTrigger,
  Notification,
  OtpInput,
  OtpState,
  Pagination,
  Popover,
  Progress,
  Radio,
  RadioGroup,
  Rating,
  Resizable,
  ResizablePanel,
  Scroll,
  Scrollbar,
  ScrollbarHandle,
  Separator,
  Select,
  Sheet,
  SettingGroup,
  SettingItem,
  SettingPage,
  Settings,
  Sidebar,
  SidebarFooter,
  SidebarHeader,
  SidebarMenu,
  SidebarMenuItem,
  Skeleton,
  Slider,
  SliderState,
  Spinner,
  StatusBar,
  Switch,
  Tag,
  Tab,
  TabBar,
  Table,
  TableBody,
  TableCaption,
  TableCell,
  TableFooter,
  TableHead,
  TableHeader,
  TableRow,
  Text,
  Textarea,
  TextareaState,
  Stepper,
  StepperItem,
  Toggle,
  Tooltip,
  Tree,
  TreeItem,
} from "gpui-component";

/**
 * Registered component elements are runtime Elements. Some generated fluent
 * method names shadow base Element methods, so bridge that structural typing
 * ambiguity only where a typed child is passed to another component.
 * @param {unknown} value
 * @returns {import("gpui").Element}
 */
const asElement = (value) =>
  /** @type {import("gpui").Element} */ (/** @type {unknown} */ (value));


/**
 * The cases shown for one registered surface.
 *
 * One to three per surface, chosen to introduce the component rather than to
 * mirror the Rust Story exhaustively: what it looks like by default, the one
 * or two variations a reader most needs to see, and any state — disabled,
 * selected, loading — that changes how it reads.
 *
 * @param {string} surface
 * @returns {Array<{ label: string, element: unknown }>}
 */
export function registeredExamples(surface) {
  switch (surface) {
    // ---------------------------------------------------------------- actions
    case "Button":
      return [
        {
          label: "Variants",
          element: h_flex()
            .gap(8)
            .items_center()
            .child(asElement(new Button("btn-primary").primary().label("Primary")))
            .child(asElement(new Button("btn-outline").outline().label("Outline")))
            .child(asElement(new Button("btn-danger").danger().label("Danger")))
            .child(asElement(new Button("btn-ghost").ghost().label("Ghost"))),
        },
        {
          label: "Sizes",
          element: h_flex()
            .gap(8)
            .items_center()
            .child(asElement(new Button("btn-xs").size("xsmall").label("XSmall")))
            .child(asElement(new Button("btn-sm").size("small").label("Small")))
            .child(asElement(new Button("btn-md").size("medium").label("Medium")))
            .child(asElement(new Button("btn-lg").size("large").label("Large"))),
        },
        {
          label: "States",
          element: h_flex()
            .gap(8)
            .items_center()
            .child(asElement(new Button("btn-loading").primary().label("Saving").loading(true)))
            .child(asElement(new Button("btn-compact").label("Compact").compact()))
            .child(asElement(new Button("btn-link").label("Link").link())),
        },
      ];
    case "DropdownButton":
      return [
        {
          label: "Split button with a menu",
          element: h_flex()
            .gap(8)
            .items_center()
            .child(
              asElement(
                new DropdownButton("dd-actions", "Actions")
                  .variant("primary")
                  .menuItem("Open", (_cx) => {})
                  .menuItem("Duplicate", (_cx) => {}),
              ),
            ),
        },
        {
          label: "Menu-only trigger",
          element: h_flex()
            .gap(8)
            .items_center()
            .child(
              asElement(
                new DropdownMenu("dd-more", "More")
                  .item("Rename", (_cx) => {})
                  .item("Archive", (_cx) => {}),
              ),
            ),
        },
      ];
    case "Toggle":
      return [
        {
          label: "Off and on",
          element: h_flex()
            .gap(8)
            .items_center()
            .child(asElement(new Toggle("toggle-off").label("Favorite")))
            .child(asElement(new Toggle("toggle-on").label("Favorite").checked(true))),
        },
        {
          label: "Outlined, and at three sizes",
          element: h_flex()
            .gap(8)
            .items_center()
            .child(asElement(new Toggle("toggle-outline").label("Pin").outline().checked(true)))
            .child(asElement(new Toggle("toggle-sm").label("Pin").size("small")))
            .child(asElement(new Toggle("toggle-lg").label("Pin").size("large"))),
        },
      ];
    case "Link":
      return [
        {
          label: "External link",
          element: asElement(
            new Link("link-docs").href("https://gpui.rs").child("gpui.rs documentation"),
          ),
        },
      ];

    // ----------------------------------------------------------- disclosure
    case "Accordion":
      return [
        {
          label: "One section open at a time",
          element: asElement(
            new Accordion("acc-single")
              .bordered(true)
              .multiple(false)
              .child(
                new AccordionItem()
                  .title(new Label("Appearance"))
                  .open(true)
                  .child("Theme, density and font size."),
              )
              .child(
                new AccordionItem()
                  .title(new Label("Notifications"))
                  .child("Email and desktop notification preferences."),
              ),
          ),
        },
        {
          label: "Several sections open at once",
          element: asElement(
            new Accordion("acc-multiple")
              .multiple(true)
              .child(
                new AccordionItem().title(new Label("Shipping")).open(true).child("Ships in 2 days."),
              )
              .child(
                new AccordionItem().title(new Label("Returns")).open(true).child("Free within 30 days."),
              ),
          ),
        },
      ];
    case "Collapsible":
      return [
        {
          label: "Open, and collapsed",
          element: v_flex()
            .gap(12)
            .child(
              asElement(
                new Collapsible()
                  .open(true)
                  .motionId("story-collapsible-open")
                  .child(asElement(new Text("Visible while the section is open."))),
              ),
            )
            .child(
              asElement(
                new Collapsible()
                  .open(false)
                  .motionId("story-collapsible-closed")
                  .child(asElement(new Text("Hidden while the section is closed."))),
              ),
            ),
        },
      ];

    // --------------------------------------------------------------- inputs
    case "Input":
      return [
        {
          label: "Editable, and disabled",
          element: v_flex()
            .gap(8)
            .child(asElement(new Input(InputState()).ariaLabel("Project name")))
            .child(asElement(new Input(InputState()).ariaLabel("Locked field").disabled(true))),
        },
      ];
    case "NumberInput":
      return [
        {
          label: "Stepper buttons on both ends",
          element: asElement(new NumberInput(InputState()).placeholder("Quantity")),
        },
      ];
    case "OtpInput":
      return [
        {
          label: "Six digits in two groups",
          element: asElement(new OtpInput(OtpState(6)).groups(2)),
        },
        {
          label: "Four digits, ungrouped",
          element: asElement(new OtpInput(OtpState(4))),
        },
      ];
    case "Textarea":
      return [
        {
          label: "Bordered, fixed height",
          element: asElement(
            new Textarea(TextareaState()).ariaLabel("Notes").bordered(true).h(120),
          ),
        },
      ];
    case "Checkbox":
      return [
        {
          label: "Unchecked, checked, and with hover help",
          element: h_flex()
            .gap(16)
            .items_center()
            .child(asElement(new Checkbox("cb-off").label("Remember me")))
            .child(asElement(new Checkbox("cb-on").label("Remember me").checked(true)))
            .child(
              asElement(
                new Checkbox("cb-tip").label("Sync").tooltip("Keeps devices in step"),
              ),
            ),
        },
      ];
    case "Switch":
      return [
        {
          label: "Off, on, and at two sizes",
          element: h_flex()
            .gap(16)
            .items_center()
            .child(asElement(new Switch("sw-off").label("Notifications")))
            .child(asElement(new Switch("sw-on").label("Notifications").checked(true)))
            .child(asElement(new Switch("sw-small").label("Compact").size("small"))),
        },
      ];
    case "Radio":
      return [
        {
          label: "A single-choice group",
          element: asElement(
            new RadioGroup("layout-density")
              .selectedIndex(1)
              .child(asElement(new Radio("comfortable").label("Comfortable")))
              .child(asElement(new Radio("compact").label("Compact")))
              .child(asElement(new Radio("dense").label("Dense"))),
          ),
        },
      ];
    case "Slider":
      return [
        {
          label: "Draggable value",
          element: asElement(new Slider(SliderState())),
        },
        {
          label: "Disabled",
          element: asElement(new Slider(SliderState()).disabled(true)),
        },
      ];
    case "ColorPicker":
      return [
        {
          label: "Labelled trigger",
          element: asElement(
            new ColorPicker(ColorPickerState())
              .label("Accent color")
              .accessibilityLabel("Choose an accent color"),
          ),
        },
      ];
    case "DatePicker":
      return [
        {
          label: "Empty, with a placeholder",
          element: asElement(new DatePicker(DatePickerState()).placeholder("Select a date")),
        },
      ];
    case "Calendar":
      return [
        {
          label: "One month",
          element: asElement(new Calendar(CalendarState())),
        },
        {
          label: "Two months side by side",
          element: asElement(new Calendar(CalendarState()).numberOfMonths(2)),
        },
      ];

    // -------------------------------------------------------------- display
    case "Text":
      return [
        {
          label: "A paragraph of text",
          element: asElement(
            new Text("Text renders a string with the active theme's body style."),
          ),
        },
      ];
    case "Label":
      return [
        {
          label: "Plain, with a secondary value, and masked",
          element: v_flex()
            .gap(8)
            .child(asElement(new Label("Account")))
            .child(asElement(new Label("Account").secondary("Connected")))
            .child(asElement(new Label("API key").masked(true))),
        },
      ];
    case "Icon":
      return [
        {
          label: "Sizes",
          element: h_flex()
            .gap(12)
            .items_center()
            .child(asElement(new Icon("icons/check.svg").size("xsmall")))
            .child(asElement(new Icon("icons/check.svg").size("small")))
            .child(asElement(new Icon("icons/check.svg").size("medium")))
            .child(asElement(new Icon("icons/check.svg").size("large"))),
        },
        {
          label: "Coloured, and rotated a quarter turn",
          element: h_flex()
            .gap(12)
            .items_center()
            .child(asElement(new Icon("icons/check.svg").color("blue-600")))
            .child(asElement(new Icon("icons/check.svg").rotate(Math.PI / 2))),
        },
      ];
    case "Image":
      return [
        {
          label: "An asset from the application directory",
          element: asElement(new Image("assets/pixel.svg").w(64).h(64)),
        },
      ];
    case "Kbd":
      return [
        {
          label: "Default, and outlined",
          element: h_flex()
            .gap(8)
            .items_center()
            .child(asElement(new Kbd("cmd-s")))
            .child(asElement(new Kbd("cmd-shift-p").outline())),
        },
      ];
    case "Separator":
      return [
        {
          label: "Plain, labelled, and dashed",
          element: v_flex()
            .w_full()
            .gap(16)
            .child(asElement(new Separator()))
            .child(asElement(new Separator().label("Account")))
            .child(asElement(new Separator().dashed())),
        },
      ];
    case "Skeleton":
      return [
        {
          label: "A loading placeholder",
          element: v_flex()
            .gap(8)
            .child(asElement(new Skeleton().w(220).h(14)))
            .child(asElement(new Skeleton().secondary().w(160).h(14))),
        },
      ];
    case "Spinner":
      return [
        {
          label: "Sizes",
          element: h_flex()
            .gap(16)
            .items_center()
            .child(asElement(new Spinner().size("small")))
            .child(asElement(new Spinner().size("medium")))
            .child(asElement(new Spinner().size("large"))),
        },
        {
          label: "Alternate icon and easing",
          element: h_flex()
            .gap(16)
            .items_center()
            .child(asElement(new Spinner().icon("loaderCircle").color("blue-600")))
            .child(asElement(new Spinner().ease("linear"))),
        },
      ];
    case "Badge":
      return [
        {
          label: "Count, capped count, and a bare dot",
          element: h_flex()
            .gap(16)
            .items_center()
            .child(asElement(new Badge().count(3)))
            .child(asElement(new Badge().count(120).max(99)))
            .child(asElement(new Badge().dot().color("red-500"))),
        },
      ];
    case "Tag":
      return [
        {
          label: "Variants",
          element: h_flex()
            .gap(8)
            .items_center()
            .child(asElement(new Tag().variant("primary").child("Primary")))
            .child(asElement(new Tag().variant("success").child("Active")))
            .child(asElement(new Tag().variant("warning").child("Pending")))
            .child(asElement(new Tag().variant("danger").child("Failed"))),
        },
        {
          label: "Outlined, and fully rounded",
          element: h_flex()
            .gap(8)
            .items_center()
            .child(asElement(new Tag().variant("info").outline().child("Draft")))
            .child(asElement(new Tag().variant("secondary").roundedFull().child("Beta"))),
        },
      ];
    case "Avatar":
      return [
        {
          label: "Initials, at three sizes",
          element: h_flex()
            .gap(12)
            .items_center()
            .child(asElement(new Avatar().name("Ada Lovelace").size("small")))
            .child(asElement(new Avatar().name("Ada Lovelace").size("medium")))
            .child(asElement(new Avatar().name("Grace Hopper").size("large"))),
        },
      ];
    case "Alert":
      return [
        {
          label: "Titled",
          element: asElement(
            new Alert("alert-saved", "Your changes have been saved.").title("Saved"),
          ),
        },
        {
          label: "As a full-width banner",
          element: asElement(
            new Alert("alert-banner", "Scheduled maintenance begins at 02:00 UTC.")
              .title("Maintenance")
              .banner(),
          ),
        },
      ];
    case "Progress":
      return [
        {
          label: "Determinate",
          element: v_flex()
            .w_full()
            .gap(12)
            .child(asElement(new Progress("p-25").value(25).accessibilityLabel("25 percent")))
            .child(asElement(new Progress("p-64").value(64).accessibilityLabel("64 percent")))
            .child(asElement(new Progress("p-100").value(100).accessibilityLabel("Complete"))),
        },
        {
          label: "Indeterminate, while work is in flight",
          element: asElement(
            new Progress("p-loading").loading(true).accessibilityLabel("Uploading"),
          ),
        },
      ];
    case "Rating":
      return [
        {
          label: "Four of five, and a ten-point scale",
          element: v_flex()
            .gap(12)
            .child(asElement(new Rating("rating-5").value(4).max(5).color("amber-500")))
            .child(asElement(new Rating("rating-10").value(7).max(10))),
        },
      ];
    case "Clipboard":
      return [
        {
          label: "Copies its value, with hover help",
          element: asElement(
            new Clipboard("copy-link").value("https://gpui.rs").tooltip("Copy link"),
          ),
        },
      ];
    case "GroupBox":
      return [
        {
          label: "Variants",
          element: v_flex()
            .w_full()
            .gap(12)
            .child(
              asElement(
                new GroupBox().title("Normal").child(asElement(new Text("Grouped content."))),
              ),
            )
            .child(
              asElement(
                new GroupBox()
                  .title("Outline")
                  .variant("outline")
                  .child(asElement(new Text("Grouped content."))),
              ),
            )
            .child(
              asElement(
                new GroupBox()
                  .title("Fill")
                  .variant("fill")
                  .child(asElement(new Text("Grouped content."))),
              ),
            ),
        },
      ];
    case "StatusBar":
      // The descriptor documents named `left` and `right` slots, but the script
      // surface exposes no way to fill them: `Element` offers `content`,
      // `header`, `footer`, `panel`, `trigger`, `image` and `fallback` only.
      // So this shows the centre region, which is all a script can reach today.
      return [
        {
          label: "Ordinary children fill the centre region",
          element: asElement(
            new StatusBar()
              .w_full()
              .child(asElement(new Text("main")))
              .child(asElement(new Text("2 problems"))),
          ),
        },
      ];

    // ------------------------------------------------------------ structure
    case "Breadcrumb":
      return [
        {
          label: "A path of three segments",
          element: asElement(new Breadcrumb(["Home", "Settings", "Profile"])),
        },
      ];
    case "Pagination":
      return [
        {
          label: "Page 2 of 5",
          element: asElement(
            new Pagination("pages").currentPage(2).totalPages(5).visiblePages(5),
          ),
        },
        {
          label: "Compact, for a narrow toolbar",
          element: asElement(
            new Pagination("pages-compact").currentPage(4).totalPages(20).compact(),
          ),
        },
      ];
    case "Stepper":
      return [
        {
          label: "Horizontal, on the second step",
          element: asElement(
            new Stepper("onboarding")
              .selectedIndex(1)
              .textCenter(true)
              .child(new StepperItem().child("Account"))
              .child(new StepperItem().child("Profile"))
              .child(new StepperItem().child("Finish")),
          ),
        },
        {
          label: "Vertical",
          element: asElement(
            new Stepper("onboarding-vertical")
              .selectedIndex(0)
              .vertical(true)
              .child(new StepperItem().child("Account"))
              .child(new StepperItem().child("Profile")),
          ),
        },
      ];
    case "Tab":
      return [
        {
          label: "Selected, unselected, and disabled",
          element: h_flex()
            .gap(8)
            .items_center()
            .child(asElement(new Tab().label("Profile").selected(true)))
            .child(asElement(new Tab().label("Security")))
            .child(asElement(new Tab().label("Billing").disabled(true))),
        },
      ];
    case "TabBar":
      return [
        {
          label: "Underline variant",
          element: asElement(
            new TabBar("profile-tabs")
              .variant("underline")
              .selectedIndex(0)
              .child(new Tab().label("Profile"))
              .child(new Tab().label("Security"))
              .child(new Tab().label("Billing")),
          ),
        },
        {
          label: "Segmented variant",
          element: asElement(
            new TabBar("view-tabs")
              .variant("segmented")
              .selectedIndex(1)
              .child(new Tab().label("List"))
              .child(new Tab().label("Board")),
          ),
        },
      ];
    case "DescriptionList":
      return [
        {
          label: "Two columns, bordered",
          element: asElement(
            new DescriptionList()
              .columns(2)
              .bordered(true)
              .child(asElement(new DescriptionItem("Owner").value("Ada Lovelace")))
              .child(asElement(new DescriptionItem("Status").value("Active")))
              .child(asElement(new DescriptionItem("Created").value("2026-01-14")))
              .child(asElement(new DescriptionItem("Region").value("eu-west-1"))),
          ),
        },
        {
          label: "Stacked, one field per row",
          element: asElement(
            new DescriptionList()
              .vertical()
              .child(asElement(new DescriptionItem("Owner").value("Ada Lovelace")))
              .child(asElement(new DescriptionItem("Status").value("Active"))),
          ),
        },
      ];
    case "Table":
      return [
        {
          label: "Header, body, footer and caption",
          element: asElement(
            new Table()
              .accessibilityLabel("Team members")
              .child(new TableCaption().child("Current project members"))
              .child(
                new TableHeader().child(
                  new TableRow()
                    .child(new TableHead().child("Name"))
                    .child(new TableHead().textRight().child("Role")),
                ),
              )
              .child(
                new TableBody()
                  .child(
                    new TableRow()
                      .child(new TableCell().child("Ada Lovelace"))
                      .child(new TableCell().textRight().child("Owner")),
                  )
                  .child(
                    new TableRow()
                      .child(new TableCell().child("Grace Hopper"))
                      .child(new TableCell().textRight().child("Maintainer")),
                  ),
              )
              .child(
                new TableFooter().child(
                  new TableRow().child(new TableCell().colSpan(2).child("2 members")),
                ),
              ),
          ),
        },
      ];
    case "Form":
      return [
        {
          label: "Two columns, one field required",
          element: asElement(
            new Form()
              .columns(2)
              .child(
                new Field()
                  .label("Account name")
                  .required(true)
                  .child(new Input(InputState()).ariaLabel("Account name")),
              )
              .child(
                new Field()
                  .label("Region")
                  .child(new Input(InputState()).ariaLabel("Region")),
              ),
          ),
        },
      ];

    // -------------------------------------------------------------- overlays
    case "Popover":
      return [
        {
          label: "Opens below the trigger",
          element: asElement(
            new Popover("popover-account", "Account details")
              .content(asElement(new Text("Signed in as ada@example.com"))),
          ),
        },
        {
          label: "Open on first render",
          element: asElement(
            new Popover("popover-open", "Already open")
              .defaultOpen(true)
              .content(asElement(new Text("Shown without a click."))),
          ),
        },
      ];
    case "HoverCard":
      return [
        {
          label: "Reveals detail after a short hover",
          element: asElement(
            new HoverCard("hover-account")
              .triggerElement(asElement(new Button("hover-trigger").label("Account help")))
              .openDelay(250)
              .child(asElement(new Text("Your account name is visible to collaborators."))),
          ),
        },
      ];
    case "Tooltip":
      return [
        {
          label: "Hover help on a control",
          element: asElement(new Tooltip("tooltip-save", "Save", "Writes changes to disk")),
        },
      ];
    case "Dialog":
      return [
        {
          label: "A modal with a title and body",
          element: asElement(
            new Dialog("dialog-project", "Open dialog", (_message, _cx) => {})
              .title("Project details")
              .content(asElement(new Text("Everything about this project."))),
          ),
        },
      ];
    case "AlertDialog":
      return [
        {
          label: "Destructive confirmation",
          element: asElement(
            new AlertDialog("alert-discard", "Discard changes", (_message, _cx) => {})
              .title("Discard changes?")
              .description("Unsaved changes will be lost.")
              .showCancel(true),
          ),
        },
      ];
    case "Sheet":
      return [
        {
          label: "Slides in from the right, and from the bottom",
          element: h_flex()
            .gap(8)
            .items_center()
            .child(
              asElement(
                new Sheet("sheet-right", "Open inspector", (_message, _cx) => {})
                  .title("Inspector")
                  .placement("right")
                  .content(asElement(new Text("Inspector content"))),
              ),
            )
            .child(
              asElement(
                new Sheet("sheet-bottom", "Open drawer", (_message, _cx) => {})
                  .title("Drawer")
                  .placement("bottom")
                  .content(asElement(new Text("Drawer content"))),
              ),
            ),
        },
      ];
    case "Notification":
      return [
        {
          label: "By type",
          element: h_flex()
            .gap(8)
            .items_center()
            .child(
              asElement(
                new Notification("notify-success", "Success", (_message, _cx) => {})
                  .title("Saved")
                  .message("Your changes were saved.")
                  .type("success"),
              ),
            )
            .child(
              asElement(
                new Notification("notify-error", "Error", (_message, _cx) => {})
                  .title("Upload failed")
                  .message("The connection was reset.")
                  .type("error")
                  .autohide(false),
              ),
            ),
        },
      ];

    // ----------------------------------------------------------- collections
    case "List":
      return [
        {
          label: "Rows built from a data callback",
          element: asElement(
            new List(
              "story-list",
              () => [
                { id: "alpha", label: "Alpha" },
                { id: "beta", label: "Beta" },
                { id: "gamma", label: "Gamma" },
              ],
              (row) => asElement(new Text(/** @type {{label: string}} */ (row).label)),
            ).h(140),
          ),
        },
      ];
    case "Select":
      return [
        {
          label: "Closed, and disabled",
          element: h_flex()
            .gap(8)
            .items_center()
            .child(
              asElement(
                new Select(
                  "select-region",
                  () => [
                    { id: "eu", label: "eu-west-1" },
                    { id: "us", label: "us-east-1" },
                  ],
                  (row) => asElement(new Text(/** @type {{label: string}} */ (row).label)),
                  (_value, _cx) => {},
                ).placeholder("Choose a region"),
              ),
            )
            .child(
              asElement(
                new Select(
                  "select-disabled",
                  () => [],
                  (_row) => asElement(new Text("")),
                  (_value, _cx) => {},
                )
                  .placeholder("Unavailable")
                  .disabled(true),
              ),
            ),
        },
      ];
    case "Combobox":
      return [
        {
          label: "Searchable",
          element: asElement(
            new Combobox(
              "combobox-searchable",
              () => [
                { id: "alpha", label: "Alpha" },
                { id: "beta", label: "Beta" },
                { id: "gamma", label: "Gamma" },
              ],
              (_value, _cx) => {},
              (_value, _cx) => {},
            )
              .placeholder("Choose an option")
              .searchable(true)
              .searchPlaceholder("Filter options"),
          ),
        },
        {
          label: "Without the search field",
          element: asElement(
            new Combobox(
              "combobox-plain",
              () => [{ id: "alpha", label: "Alpha" }],
              (_value, _cx) => {},
              (_value, _cx) => {},
            )
              .placeholder("Choose an option")
              .searchable(false),
          ),
        },
      ];
    case "Tree":
      return [
        {
          label: "An expanded folder with two files",
          element: asElement(
            new Tree("project-tree").child(
              asElement(
                new TreeItem("src", "src")
                  .expanded(true)
                  .child(asElement(new TreeItem("main", "main.rs")))
                  .child(asElement(new TreeItem("lib", "lib.rs"))),
              ),
            ),
          ),
        },
      ];
    case "DataTable":
      return [
        {
          label: "Striped rows, sortable and resizable columns",
          element: asElement(
            new DataTable(
              DataTableState(["name", "status"]),
              () => [
                { name: "Alpha", status: "Ready" },
                { name: "Beta", status: "Building" },
                { name: "Gamma", status: "Failed" },
              ],
              (row, column) =>
                asElement(
                  new Text(String(/** @type {Record<string, string>} */ (row)[column])),
                ),
            )
              .stripe(true)
              .sortable(true)
              .columnResizable(true)
              .h(180),
          ),
        },
      ];
    case "Command":
      return [
        {
          label: "A filterable command palette",
          element: asElement(
            new Command(CommandState())
              .placeholder("Type a command")
              .bordered(true)
              .maxHeight(200)
              .header(asElement(new Text("Commands")))
              .footer(asElement(new Text("Press Enter to run")))
              .child(
                asElement(
                  new CommandGroup("Files")
                    .child(asElement(new CommandItem("Open file").keyword("file").action("open")))
                    .child(asElement(new CommandItem("Save file").keyword("save").action("save"))),
                ),
              )
              .child(
                asElement(
                  new CommandGroup("View").child(
                    asElement(new CommandItem("Toggle sidebar").action("sidebar")),
                  ),
                ),
              ),
          ),
        },
      ];

    // -------------------------------------------------------- layout & panels
    case "Sidebar":
      return [
        {
          label: "Header, menu and footer",
          element: asElement(
            new Sidebar("story-sidebar")
              .side("left")
              .collapsible("icon")
              .h(220)
              .header(asElement(new SidebarHeader().child("Workspace")))
              .footer(asElement(new SidebarFooter().child("Account")))
              .child(
                asElement(
                  new SidebarMenu()
                    .child(asElement(new SidebarMenuItem("Components").selected(true)))
                    .child(asElement(new SidebarMenuItem("Settings")))
                    .child(asElement(new SidebarMenuItem("Archived").disabled(true))),
                ),
              ),
          ),
        },
        {
          label: "Collapsed to icons",
          element: asElement(
            new Sidebar("story-sidebar-collapsed")
              .collapsible("icon")
              .collapsed(true)
              .h(160)
              .child(
                asElement(
                  new SidebarMenu().child(
                    asElement(new SidebarMenuItem("Components").selected(true)),
                  ),
                ),
              ),
          ),
        },
      ];
    case "Resizable":
      return [
        {
          label: "Two panels with a draggable divider",
          element: asElement(
            new Resizable("story-split")
              .axis("horizontal")
              .crossSize(160)
              .child(asElement(new ResizablePanel().size(160).child("Navigation")))
              .child(asElement(new ResizablePanel().child("Content"))),
          ),
        },
      ];
    case "Scroll":
      return [
        {
          label: "A vertical scroll region",
          element: asElement(
            new Scroll(ScrollbarHandle())
              .scrollAxis("vertical")
              .h(140)
              .child(
                v_flex()
                  .gap(8)
                  .children(
                    Array.from({ length: 12 }, (_unused, index) =>
                      asElement(new Text(`Row ${index + 1}`)),
                    ),
                  ),
              ),
          ),
        },
      ];
    case "Scrollbar": {
      const handle = ScrollbarHandle();
      return [
        {
          label: "An always-visible scrollbar beside its region",
          element: asElement(
            new GroupBox()
              .child(
                asElement(
                  new Scroll(handle)
                    .h(140)
                    .child(
                      v_flex()
                        .gap(8)
                        .children(
                          Array.from({ length: 12 }, (_unused, index) =>
                            asElement(new Text(`Row ${index + 1}`)),
                          ),
                        ),
                    ),
                ),
              )
              .child(
                asElement(
                  new Scrollbar("story-scrollbar", handle).scrollAxis("vertical").mode("always"),
                ),
              ),
          ),
        },
      ];
    }
    case "Settings":
      return [
        {
          label: "A page of grouped settings",
          element: asElement(
            new Settings("story-settings")
              .size("medium")
              .sidebarWidth(200)
              .h(260)
              .child(
                asElement(
                  new SettingPage("General").child(
                    asElement(
                      new SettingGroup()
                        .title("Appearance")
                        .child(
                          asElement(
                            new SettingItem("Theme")
                              .description("Follows the system setting.")
                              .content(asElement(new Text("System"))),
                          ),
                        )
                        .child(
                          asElement(
                            new SettingItem("Density").content(asElement(new Text("Comfortable"))),
                          ),
                        ),
                    ),
                  ),
                ),
              ),
          ),
        },
      ];
    case "Editor":
      return [
        {
          label: "Editable, and read-only",
          element: v_flex()
            .gap(12)
            .child(
              asElement(
                new Editor(EditorState("fn main() {\n    println!(\"hello\");\n}"))
                  .ariaLabel("Source editor")
                  .bordered(true)
                  .h(120),
              ),
            )
            .child(
              asElement(
                new Editor(EditorState("// generated, do not edit"))
                  .ariaLabel("Generated source")
                  .bordered(true)
                  .readonly(true)
                  .h(80),
              ),
            ),
        },
      ];

    // ---------------------------------------------------------------- charts
    case "BarChart":
      return [
        {
          label: "With grid lines and both axes",
          element: asElement(
            new BarChart(() => [
              { label: "Mon", value: 42 },
              { label: "Tue", value: 68 },
              { label: "Wed", value: 31 },
              { label: "Thu", value: 75 },
              { label: "Fri", value: 54 },
            ])
              .grid(true)
              .labelAxis(true)
              .valueAxis(true)
              .h(200),
          ),
        },
        {
          label: "Bars only",
          element: asElement(
            new BarChart(() => [
              { label: "Mon", value: 42 },
              { label: "Tue", value: 68 },
              { label: "Wed", value: 31 },
            ]).h(140),
          ),
        },
      ];

    // ------------------------------------------------- platform integration
    case "MenuBar":
      return [
        {
          label: "An application menu installed for this window",
          element: asElement(
            new MenuBar("story-menu-bar").child(
              asElement(
                new Menu("File")
                  .child(asElement(new MenuItem("Open", "open")))
                  .child(asElement(new MenuSeparator()))
                  .child(asElement(new MenuItem("Close", "close").disabled(true))),
              ),
            ),
          ),
        },
      ];
    case "NativeMenuTrigger":
      return [
        {
          label: "Opens the platform's own menu",
          element: asElement(
            new NativeMenuTrigger("native-menu", "Native menu")
              .onEffectError((_message, _cx) => {})
              .child(asElement(new NativeMenuItem("Open", "open")))
              .child(asElement(new NativeMenuSeparator()))
              .child(asElement(new NativeMenuItem("Close", "close").disabled(true))),
          ),
        },
      ];

    default:
      throw new Error(
        `No JavaScript Story example is defined for registered ${surface}`,
      );
  }
}
