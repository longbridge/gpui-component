// These exports are public constructors from the current component-shell
// inventory. Constructor calls intentionally use `new`, matching the generated
// gpui-component declarations.
import {
  Alert,
  Avatar,
  Badge,
  Breadcrumb,
  Button,
  Checkbox,
  Clipboard,
  Collapsible,
  GroupBox,
  Kbd,
  Label,
  Link,
  Pagination,
  Progress,
  Radio,
  Rating,
  Separator,
  Skeleton,
  Spinner,
  StatusBar,
  Switch,
  Tag,
  Toggle,
} from "gpui-component";

/** @param {string} surface */
export function registeredExample(surface) {
  switch (surface) {
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
    case "Checkbox":
      return new Checkbox("terms").checked(true).label("Accept terms");
    case "Clipboard":
      return new Clipboard("copy-link")
        .value("https://example.com")
        .tooltip("Copy link");
    case "Collapsible":
      return new Collapsible().open(true).motionId("story-collapsible");
    case "GroupBox":
      return new GroupBox().title("Preferences").variant("outline");
    case "Kbd":
      return new Kbd("cmd-s").outline();
    case "Label":
      return new Label("Account").secondary("Connected");
    case "Link":
      return new Link("documentation")
        .href("https://gpui.rs")
        .child("Documentation");
    case "Pagination":
      return new Pagination("pages")
        .currentPage(2)
        .totalPages(5)
        .visiblePages(5);
    case "Progress":
      return new Progress("upload")
        .value(64)
        .accessibilityLabel("Upload progress");
    case "Radio":
      return new Radio("compact")
        .label("Compact layout")
        .checked(true)
        .tabStop(true);
    case "Rating":
      return new Rating("quality").value(4).max(5).color("amber-500");
    case "Separator":
      return new Separator().label("Account").dashed();
    case "Skeleton":
      return new Skeleton().secondary().w(180).h(20);
    case "Spinner":
      return new Spinner().size("large").color("blue-600").ease("linear");
    case "StatusBar":
      return new StatusBar();
    case "Switch":
      return new Switch("notifications").checked(true).label("Notifications");
    case "Tag":
      return new Tag().variant("success").roundedFull().child("Active");
    case "Toggle":
      return new Toggle("favorite").checked(true).label("Favorite").outline();
    default:
      throw new Error(
        `No JavaScript Story example is defined for registered ${surface}`,
      );
  }
}
