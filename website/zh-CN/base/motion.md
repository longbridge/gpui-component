---
title: 动画与动效
description: gpui-base 的类型化 transition、spring、keyframes、presence、stagger 与 reduced-motion 行为。
order: 3
example: motion
exampleKind: base
---

# 动画与动效

`gpui-base` 负责确定性的动效采样与生命周期，并把视觉选择留给应用。它提供稳定 keyed state、中断与反向、animation frame 请求和 reduced-motion 处理，不强加产品级时长或样式。

运行本文配套的交互示例：

```bash
cargo run -p gpui-base --example motion
```

示例包含五个相互独立的页面，可通过顶部标签逐个查看。

## 能力一览

| 示例 | API | 演示内容 |
| --- | --- | --- |
| Sliding time | `transition` | 08:00–20:00 的四位独立滚动数字，目标会在前一次过渡完成前继续变化 |
| Spring | `spring` | 快速切换目标时仍保持速度连续的分段选择器指示块 |
| Keyframes | `Keyframes`、`Timing`、`animate_keyframes` | 持续循环的多段活动信号 |
| Stagger | `Stagger` | 无分配地为列表计算错峰时间 |
| Presence | `Presence` | 退出动画完成前继续挂载内容 |

此外还提供 `Easing`、`Discrete`、`MotionTransform` 和 `MotionReveal`，它们与同一套 primitive 组合，不需要额外动画 runtime。

## Transition

已知时长、向目标值变化时使用 `transition`。每个独立运动值都要有稳定 ID：

```rust,ignore
let opacity = transition(
    ("save-dialog", "opacity"),
    if open { 1.0 } else { 0.0 },
    Transition::new(Duration::from_millis(180)).easing(Easing::EaseOut),
    window,
    cx,
);
```

运动中改变目标会从当前采样值继续；直接反向还会缩短返回时长。`transition_with_status` 额外返回 `Idle`、`Delayed`、`Running` 或 `Finished`。

`Easing` 支持 CSS 关键字曲线、cubic Bézier、全部 step position 和分段 `linear()` stops，无效参数会返回类型化错误。

## Spring

目标可能在运动中变化时使用 `spring`。它同时保留位置与速度，适合选择指示器和空间值回落。

```rust,ignore
let x = spring(
    "selected-indicator",
    selected_x,
    Spring::new(Duration::from_millis(420)).with_damping(0.72),
    window,
    cx,
);
```

指针直接控制数值时，不要让 spring 追赶指针；拖动中使用 `with_travel(false)`，释放后再恢复。

## Keyframes 与 Timing

`Keyframes` 定义经过校验的值序列；`Timing` 按绝对 elapsed time 采样，支持正负 delay、有限或无限迭代，以及 normal、reverse 和 alternate 播放方向。

offset 必须从 `0` 开始、以 `1` 结束并保持单调。不可插值属性使用 `Discrete`。

## Presence 与 Stagger

`Presence` 将逻辑可见性与实际挂载分开，阶段包括 entering、present、exiting 和 absent。`should_render()` 为 true 时继续渲染，并把 `progress` 应用到所选视觉属性。退出中重新打开会从当前进度反向。

`Stagger` 可以从首项、末项、中心或指定位置开始，为每个 index 计算 delay；它不分配时间表，也不接管列表 identity。

## 测量式展开

`MotionReveal` 按 child 的自然尺寸测量，再根据 progress 裁剪可见高度。`Collapsible::motion_id(...)` 是控件层的便捷入口；没有 motion ID 时仍保持即时挂载/卸载。

## Reduced motion 与性能

Transition、spring、keyframes、presence 和 reveal 控件都遵守 GPUI 的 reduced-motion 偏好。有限动画会直接同步目标、更新 retained state，并且不留下待处理 frame。动画不能成为表达状态的唯一方式。

稳定采样路径零分配，采样使用绝对时间，关键帧查找使用二分搜索。运行 release benchmark：

```bash
cargo bench -p gpui-base --bench motion
```

选择最小且合适的 primitive：固定时长目标使用 `transition`，频繁变化的空间目标使用 `spring`，编排序列使用 keyframes，卸载前退出使用 `Presence`，列表错峰使用 `Stagger`。
