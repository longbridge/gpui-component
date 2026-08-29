// These exports are public constructors from the current component-shell
// inventory. Constructor calls intentionally use `new`, matching the generated
// gpui-component declarations.
import {
  Accordion,
  AccordionItem,
  Alert,
  Avatar,
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
  DatePicker,
  DatePickerState,
  DescriptionItem,
  DescriptionList,
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
  NumberInput,
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
  Tabs,
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

/** @param {string} surface */
export function registeredExample(surface) {
  switch (surface) {
    case "Accordion":
      return new Accordion("preferences")
        .bordered(true)
        .multiple(false)
        .child(
          new AccordionItem()
            .title(new Label("Appearance"))
            .open(true)
            .child("Theme and density preferences"),
        )
        .child(
          new AccordionItem()
            .title(new Label("Notifications"))
            .child("Email and desktop notification preferences"),
        );
    case "Alert":
      return new Alert("saved", "Your changes have been saved").title("Saved");
    case "Avatar":
      return new Avatar().name("Ada Lovelace").size("large");
    case "Badge":
      return new Badge().count(12).color("blue-600");
    case "Breadcrumb":
      return new Breadcrumb(["Home", "Settings", "Profile"]);
    case "Button":
      return new Button("save").primary().label("Save changes").loading(false);
    case "Calendar":
      return new Calendar(CalendarState()).numberOfMonths(2);
    case "Checkbox":
      return new Checkbox("terms").checked(true).label("Accept terms");
    case "Clipboard":
      return new Clipboard("copy-link")
        .value("https://example.com")
        .tooltip("Copy link");
    case "Collapsible":
      return new Collapsible().open(true).motionId("story-collapsible");
    case "ColorPicker":
      return new ColorPicker(ColorPickerState())
        .label("Accent color")
        .accessibilityLabel("Choose an accent color");
    case "DatePicker":
      return new DatePicker(DatePickerState()).placeholder("Select a date");
    case "DescriptionList":
      return new DescriptionList()
        .columns(2)
        .bordered(true)
        .child(asElement(new DescriptionItem("Owner").value("Ada Lovelace")))
        .child(asElement(new DescriptionItem("Status").value("Active")));
    case "DropdownButton":
      return new GroupBox()
        .child(
          asElement(
            new DropdownButton("actions", "Actions")
              .variant("primary")
              .menuItem("Open", (_cx) => {}),
          ),
        )
        .child(
          asElement(
            new DropdownMenu("more-actions", "More")
              .item("Rename", (_cx) => {})
              .item("Archive", (_cx) => {}),
          ),
        );
    case "Editor":
      return new Editor(EditorState("fn main() {}"))
        .ariaLabel("Source editor")
        .bordered(true)
        .h(180);
    case "Form":
      return new Form()
        .columns(2)
        .child(
          new Field()
            .label("Account name")
            .required(true)
            .child(new Input(InputState()).ariaLabel("Account name")),
        );
    case "GroupBox":
      return new GroupBox().title("Preferences").variant("outline");
    case "HoverCard":
      return new HoverCard("account-help")
        .triggerElement(
          asElement(new Button("account-help-trigger").label("Account help")),
        )
        .openDelay(250)
        .child("Your account name is visible to collaborators.");
    case "Icon":
      return new Icon("icons/check.svg").size("small");
    case "Image":
      return new Image("assets/pixel.svg").w(64).h(64);
    case "Input":
      return new Input(InputState()).ariaLabel("Project name");
    case "Kbd":
      return new Kbd("cmd-s").outline();
    case "Label":
      return new Label("Account").secondary("Connected");
    case "Link":
      return new Link("documentation")
        .href("https://gpui.rs")
        .child("Documentation");
    case "NumberInput":
      return new NumberInput(InputState()).placeholder("Quantity");
    case "OtpInput":
      return new OtpInput(OtpState(6)).groups(2);
    case "Pagination":
      return new Pagination("pages")
        .currentPage(2)
        .totalPages(5)
        .visiblePages(5);
    case "Popover":
      return new Popover("account-popover", "Open details")
        .content(asElement(new Text("Lazy popover content")))
        .anchor("bottomLeft");
    case "Progress":
      return new Progress("upload")
        .value(64)
        .accessibilityLabel("Upload progress");
    case "Radio":
      return new RadioGroup("layout-density")
        .selectedIndex(1)
        .onChange((_index) => {})
        .child(asElement(new Radio("comfortable").label("Comfortable layout")))
        .child(asElement(new Radio("compact").label("Compact layout")));
    case "Rating":
      return new Rating("quality").value(4).max(5).color("amber-500");
    case "Resizable":
      return new Resizable("story-split")
        .axis("horizontal")
        .crossSize(180)
        .child(asElement(new ResizablePanel().size(140).child("Navigation")))
        .child(asElement(new ResizablePanel().child("Content")));
    case "Scroll": {
      const handle = ScrollbarHandle();
      return new Scroll(handle)
        .axis("vertical")
        .h(160)
        .child("Scrollable content");
    }
    case "Scrollbar": {
      const handle = ScrollbarHandle();
      return new GroupBox()
        .child(asElement(new Scroll(handle).h(160).child("Scrollable content")))
        .child(
          asElement(
            new Scrollbar("story-scrollbar", handle)
              .axis("vertical")
              .mode("always"),
          ),
        );
    }
    case "Separator":
      return new Separator().label("Account").dashed();
    case "Sidebar":
      return new Sidebar("story-sidebar")
        .side("left")
        .collapsible("icon")
        .header(asElement(new SidebarHeader().child("Workspace")))
        .footer(asElement(new SidebarFooter().child("Account")))
        .child(
          asElement(
            new SidebarMenu().child(
              asElement(new SidebarMenuItem("Components").active(true)),
            ),
          ),
        );
    case "Skeleton":
      return new Skeleton().secondary().w(180).h(20);
    case "Slider":
      return new Slider(SliderState()).disabled(false);
    case "Spinner":
      return new Spinner().size("large").color("blue-600").ease("linear");
    case "StatusBar":
      return new StatusBar();
    case "Switch":
      return new Switch("notifications").checked(true).label("Notifications");
    case "Tag":
      return new Tag().variant("success").roundedFull().child("Active");
    case "Tab":
      return new Tab().label("Profile").selected(true);
    case "Tabs":
      return new Tabs("profile-tabs")
        .selectedIndex(0)
        .variant("underline")
        .child(new Tab().label("Profile"))
        .child(new Tab().label("Security"));
    case "Table":
      return new Table()
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
          new TableBody().child(
            new TableRow()
              .child(new TableCell().child("Ada Lovelace"))
              .child(new TableCell().textRight().child("Owner")),
          ),
        )
        .child(
          new TableFooter().child(
            new TableRow().child(new TableCell().colSpan(2).child("1 member")),
          ),
        );
    case "Text":
      return new Text("A text component rendered from JavaScript");
    case "Textarea":
      return new Textarea(TextareaState())
        .ariaLabel("Notes")
        .bordered(true)
        .h(120);
    case "Stepper":
      return new Stepper("onboarding")
        .selectedIndex(1)
        .textCenter(true)
        .child(new StepperItem().child("Account"))
        .child(new StepperItem().child("Profile"))
        .child(new StepperItem().child("Finish"));
    case "Toggle":
      return new Toggle("favorite").checked(true).label("Favorite").outline();
    case "Tree":
      return new Tree("project-tree").child(
        asElement(
          new TreeItem("src", "src")
            .expanded(true)
            .child(asElement(new TreeItem("main", "main.rs")))
            .child(asElement(new TreeItem("lib", "lib.rs"))),
        ),
      );
    default:
      throw new Error(
        `No JavaScript Story example is defined for registered ${surface}`,
      );
  }
}
