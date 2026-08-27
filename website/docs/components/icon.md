---
title: Icon
description: Display SVG icons with various sizes, colors, and transformations.
---

# Icon

A flexible icon component that renders SVG icons from the built-in icon library. Icons are based on Lucide.dev and support customization of size, color, and rotation. The component requires SVG files to be provided by the user in the assets bundle.

Before you start, please make sure you have read: [Icons & Assets](../assets.md) to understand how use SVG in GPUI & GPUI Component application.

## Import

```rust
use gpui_component::{Icon, IconName};
```

## Usage

### Basic Icon

```rust
// Using IconName enum directly
IconName::Heart

// Or creating an Icon explicitly
Icon::new(IconName::Heart)
```

### Icon with Custom Size

```rust
// Predefined sizes
Icon::new(IconName::Search).xsmall()   // size_3()
Icon::new(IconName::Search).small()    // size_3p5()
Icon::new(IconName::Search).medium()   // size_4() (default)
Icon::new(IconName::Search).large()    // size_6()

// Custom pixel size
Icon::new(IconName::Search).with_size(px(20.))
```

### Icon with Custom Color

```rust
// Using theme colors
Icon::new(IconName::Heart)
    .text_color(cx.theme().red)

// Using custom colors
Icon::new(IconName::Star)
    .text_color(gpui::red())
```

### Rotated Icons

```rust
use gpui::Radians;

// Rotate by radians
Icon::new(IconName::ArrowUp)
    .rotate(Radians::from_degrees(90.))

// Transform with custom transformation
Icon::new(IconName::ChevronRight)
    .transform(Transformation::rotate(Radians::PI))
```

### Custom SVG Path

```rust
// Using a custom SVG file from assets
Icon::new(Icon::empty())
    .path("icons/my-custom-icon.svg")
```

## Available Icons

The `IconName` enum provides access to a curated set of icons. Here are some commonly used ones:

### Navigation

- `ArrowUp`, `ArrowDown`, `ArrowLeft`, `ArrowRight`
- `ChevronUp`, `ChevronDown`, `ChevronLeft`, `ChevronRight`
- `ChevronsUpDown`

### Actions

- `Check`, `Close`, `Plus`, `Minus`
- `Copy`, `Delete`, `Search`, `Replace`
- `Maximize`, `Minimize`, `WindowRestore`

### Files & Folders

- `File`, `Folder`, `FolderOpen`, `FolderClosed`
- `BookOpen`, `Inbox`

### UI Elements

- `Menu`, `Settings`, `Settings2`, `Ellipsis`, `EllipsisVertical`
- `Eye`, `EyeOff`, `Bell`, `Info`

### Social & External

- `GitHub`, `Globe`, `ExternalLink`
- `Heart`, `HeartOff`, `Star`, `StarOff`
- `ThumbsUp`, `ThumbsDown`

### Status & Alerts

- `CircleCheck`, `CircleX`, `TriangleAlert`
- `Loader`, `LoaderCircle`

### Panels & Layout

- `PanelLeft`, `PanelRight`, `PanelBottom`
- `PanelLeftOpen`, `PanelRightOpen`, `PanelBottomOpen`
- `LayoutDashboard`, `Frame`

### Users & Profile

- `User`, `CircleUser`, `Bot`

### Other

- `Calendar`, `Map`, `Palette`, `Inspector`
- `Sun`, `Moon`, `Building2`

## Icon Sizes

The Icon component supports several predefined sizes:

| Size        | Method                | CSS Class    | Pixels |
| ----------- | --------------------- | ------------ | ------ |
| Extra Small | `.xsmall()`           | `size_3()`   | 12px   |
| Small       | `.small()`            | `size_3p5()` | 14px   |
| Medium      | `.medium()` (default) | `size_4()`   | 16px   |
| Large       | `.large()`            | `size_6()`   | 24px   |
| Custom      | `.with_size(px(n))`   | -            | n px   |

## Build you own `IconName`.

You can define your own `IconName` to have more specific icons for your application. We have `IconNamed` trait for you to implement for your.

```rust
use gpui_component::IconNamed;

pub enum IconName {
    Encounters,
    Monsters,
    Spells,
}

impl IconNamed for IconName {
    fn path(self) -> gpui::SharedString {
        match self {
            IconName::Encounters => "icons/encounters.svg",
            IconName::Monsters => "icons/monsters.svg",
            IconName::Spells => "icons/spells.svg",
        }
        .into()
    }
}

// This allows for the following interactions (works with anything that has the `.icon(icon)` method.
Button::new("my-button").icon(IconName::Spells);
Icon::new(IconName::Monsters);
```

If you want to directly `render` a custom `IconName` you must implement the `RenderOnce` trait and derive `IntoElement` on the `IconName`.

```rust
impl RenderOnce for IconName {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        Icon::empty().path(self.path())
    }
}

// Now you can use it directly in your element tree:
div()
    .child(IconName::Monsters)
```

## Examples

### Icon in Button

```rust
use gpui_component::button::Button;

Button::new("like-btn")
    .icon(
        Icon::new(IconName::Heart)
            .text_color(cx.theme().red)
            .large()
    )
    .label("Like")
```

### Animated Loading Icon

```rust
Icon::new(IconName::LoaderCircle)
    .text_color(cx.theme().muted_foreground)
    .medium()
    // Add rotation animation in your render logic
```

### Status Icons

```rust
// Success
Icon::new(IconName::CircleCheck)
    .text_color(cx.theme().green)

// Error
Icon::new(IconName::CircleX)
    .text_color(cx.theme().red)

// Warning
Icon::new(IconName::TriangleAlert)
    .text_color(cx.theme().yellow)
```

### Navigation Icons

```rust
// Back button
Icon::new(IconName::ArrowLeft)
    .medium()
    .text_color(cx.theme().foreground)

// Dropdown indicator
Icon::new(IconName::ChevronDown)
    .small()
    .text_color(cx.theme().muted_foreground)
```

### Custom Icon from Assets

```rust
// Using a custom SVG file
Icon::empty()
    .path("icons/my-brand-logo.svg")
    .large()
    .text_color(cx.theme().primary)
```

## Notes

- Icons are rendered as SVG elements and support full CSS styling
- The default size matches the current text size if no explicit size is set
- Icons are flex-shrink-0 by default to prevent unwanted shrinking in flex layouts
- All icon paths are relative to the assets bundle root
- Icons from Lucide.dev are designed to work well at 16px and scale nicely to other sizes
