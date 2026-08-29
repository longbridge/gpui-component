---
title: Carousel
description: 用于浏览相关内容的可组合 Carousel 组件。
---

# Carousel

Carousel 用于逐项浏览一组相关内容，支持横向和纵向布局、键盘导航、指针与触控板手势、循环以及受控选中项。

## 引入

```rust
use gpui::Axis;
use gpui_component::carousel::{
    Carousel, CarouselContent, CarouselEvent, CarouselItem, CarouselNext,
    CarouselPagination, CarouselPaginationItem, CarouselPrevious, CarouselState,
};
```

## 使用

为内容创建一个 `CarouselState`，并将它传给所有 Carousel 部件。

```rust
let state = cx.new(|_| CarouselState::new(3));

Carousel::new("projects-carousel", &state)
    .child(
        CarouselContent::new(&state)
            .child(CarouselItem::new("project-1", 0, &state).child("项目一"))
            .child(CarouselItem::new("project-2", 1, &state).child("项目二"))
            .child(CarouselItem::new("project-3", 2, &state).child("项目三")),
    )
    .child(CarouselPrevious::new(&state))
    .child(CarouselNext::new(&state))
```

`CarouselContent` 管理 viewport 与吸附布局，`CarouselItem` 标识一个逻辑 slide。到达对应边界时，上一项和下一项按钮会自动禁用。

state 的 item 数量应与直接 `CarouselItem` 子元素的数量一致。一个 state 及其 scroll handle 只服务一个 viewport。

## 方向

创建 state 时使用 `with_axis`：

```rust
let state = cx.new(|_| {
    CarouselState::new(3).with_axis(Axis::Vertical)
});
```

横向 Carousel 使用 Left 和 Right，纵向 Carousel 使用 Up 和 Down。
纵向 `CarouselContent` 需要设置明确的高度，让每个全高 item 都有可供吸附的 viewport。

Carousel 根节点可通过 Tab 获得焦点，因此省略可选控制按钮时仍可使用键盘导航。Home 和 End 用于选择第一项和最后一项。

## 循环

启用循环后，从最后一项继续向后会回到第一项：

```rust
let state = cx.new(|_| CarouselState::new(5).with_looping(true));
```

## 受控选中项

应用可以控制 `CarouselState`。使用 `with_selected_index` 设置初始选中项，使用 `set_selected_index` 进行程序化切换。

```rust
let state = cx.new(|_| CarouselState::new(4).with_selected_index(1));

state.update(cx, |state, cx| {
    state.set_selected_index(3, cx);
});
```

如果应用需要同步当前 slide，可以监听 `CarouselEvent::Change`：

```rust
cx.subscribe(&state, |this, _, event: &CarouselEvent, cx| {
    let CarouselEvent::Change(index) = event;
    this.selected_index = *index;
    cx.notify();
});
```

## 事件

| 事件 | 说明 |
| --- | --- |
| `CarouselEvent::Change(index)` | 用户导航选中新的内容时触发。 |

键盘导航和上一项/下一项按钮使用同一套 state 状态转换，并触发相同事件。指针和触控板手势结束时，会吸附到最近的 snap 点。

## 分页指示器

分页是可选部件，不会固定一种视觉样式。使用 `CarouselPaginationItem` 组合指示器，再按需要设置每一项的样式或内容：

```rust
CarouselPagination::new().children((0..3).map(|index| {
    CarouselPaginationItem::new(("project-page", index), index, &state)
        .child((index + 1).to_string())
}))
```

`CarouselPaginationItem` 与指针、键盘和上一项/下一项导航使用同一套 selection 状态转换。

## 控件尺寸

`CarouselPrevious`、`CarouselNext` 和 `CarouselPaginationItem` 实现了 `Sizable`。需要让这些控件同步缩放时，为它们设置相同的语义尺寸：

```rust
use gpui_component::{Sizable as _, Size};

CarouselPrevious::new(&state).with_size(Size::Large);
CarouselNext::new(&state).with_size(Size::Large);
```

上一项和下一项控件默认使用 `Size::Small`，分页项默认使用 `Size::XSmall`。

## 无障碍

Carousel 会提供带 label 的区域，每个 item 会报告自己在内容集合中的位置。当默认的“轮播”无法准确描述内容时，使用 `with_accessibility_label` 设置更明确的名称。

Carousel 动画会遵循应用的减少动效设置。
