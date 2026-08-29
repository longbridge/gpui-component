---
title: Carousel
description: A composable carousel for browsing related content.
---

# Carousel

Carousel displays a set of related items one at a time. It supports horizontal and vertical layouts, keyboard navigation, pointer and trackpad gestures, looping, and controlled selection.

## Import

```rust
use gpui::Axis;
use gpui_component::carousel::{
    Carousel, CarouselContent, CarouselEvent, CarouselItem, CarouselNext,
    CarouselPagination, CarouselPaginationItem, CarouselPrevious, CarouselState,
};
```

## Usage

Create one `CarouselState` for the content and pass it to every carousel part.

```rust
let state = cx.new(|_| CarouselState::new(3));

Carousel::new("projects-carousel", &state)
    .child(
        CarouselContent::new(&state)
            .child(CarouselItem::new("project-1", 0, &state).child("Project one"))
            .child(CarouselItem::new("project-2", 1, &state).child("Project two"))
            .child(CarouselItem::new("project-3", 2, &state).child("Project three")),
    )
    .child(CarouselPrevious::new(&state))
    .child(CarouselNext::new(&state))
```

`CarouselContent` owns the viewport and snap layout. `CarouselItem` identifies one logical slide. The previous and next controls automatically become disabled at the corresponding boundary.

Keep the state's item count equal to the number of direct `CarouselItem` children. A state and its scroll handle belong to one viewport.

## Orientation

Use `with_axis` when creating the state:

```rust
let state = cx.new(|_| {
    CarouselState::new(3).with_axis(Axis::Vertical)
});
```

Horizontal carousels use Left and Right. Vertical carousels use Up and Down.
Give vertical `CarouselContent` an explicit height so each full-height item has a viewport to snap within.

The Carousel root is a tab stop, so keyboard navigation also works when optional controls are omitted. Home and End select the first and last items.

## Looping

Enable looping to wrap navigation from the last item to the first:

```rust
let state = cx.new(|_| CarouselState::new(5).with_looping(true));
```

## Controlled selection

`CarouselState` can be controlled by application state. Use `with_selected_index` for the initial selection and `set_selected_index` for programmatic changes.

```rust
let state = cx.new(|_| CarouselState::new(4).with_selected_index(1));

state.update(cx, |state, cx| {
    state.set_selected_index(3, cx);
});
```

Subscribe to `CarouselEvent::Change` when the application needs to mirror the selected item:

```rust
cx.subscribe(&state, |this, _, event: &CarouselEvent, cx| {
    let CarouselEvent::Change(index) = event;
    this.selected_index = *index;
    cx.notify();
});
```

## Events

| Event | Description |
| --- | --- |
| `CarouselEvent::Change(index)` | Emitted when user navigation selects a new item. |

Keyboard navigation and previous/next controls use the same state transition and emit the same event. Pointer and trackpad gestures select the nearest snap point when the gesture ends.

## Pagination indicators

Pagination is optional and does not impose one visual treatment. Compose indicators with `CarouselPaginationItem`, then style or fill each item as needed:

```rust
CarouselPagination::new().children((0..3).map(|index| {
    CarouselPaginationItem::new(("project-page", index), index, &state)
        .child((index + 1).to_string())
}))
```

`CarouselPaginationItem` uses the same selection transition as pointer, keyboard, and previous/next navigation.

## Control size

`CarouselPrevious`, `CarouselNext`, and `CarouselPaginationItem` implement `Sizable`. Apply the same semantic size to the controls when they should scale together:

```rust
use gpui_component::{Sizable as _, Size};

CarouselPrevious::new(&state).with_size(Size::Large);
CarouselNext::new(&state).with_size(Size::Large);
```

Previous and next controls default to `Size::Small`. Pagination items default to `Size::XSmall`.

## Accessibility

The carousel exposes a labelled region and each item reports its position within the set. Use `with_accessibility_label` when the default "Carousel" label does not describe the content.

Carousel animation follows the application's reduced-motion preference.
