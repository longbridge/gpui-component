---
title: 动画与动效
description: GPUI 应用中的 CSS 对齐 timing、transition、spring、keyframes、presence、reveal 与性能指南。
---

# 动画与动效

GPUI Component 将动画分成两层：`gpui-base` 负责确定性的采样和生命周期，`gpui-component` 负责视觉策略与主题 token。这不是 CSS 解析器，而是把 CSS Transitions、CSS Animations 和 Web Animations 中适合原生 retained UI 的语义做成类型安全的 Rust API。

## 能力一览

| 能力 | API | 说明 |
| --- | --- | --- |
| 目标值过渡 | `transition`、`transition_with_status` | 稳定 identity、delay、easing、中断、反向缩时 |
| 物理运动 | `spring` | 改变目标时保留速度，且与帧率无关 |
| CSS easing | `Easing` | 关键字、cubic Bézier、`steps()`、分段 `linear()` |
| CSS timing | `Timing` | 正负 delay、有限/无限迭代、正向/反向/交替 |
| 关键帧 | `Keyframes`、`animate_keyframes` | offset 校验与分段 easing |
| 离散值 | `Discrete` | 在指定进度切换不可插值状态 |
| 错峰 | `Stagger` | 从首项、末项或中心无分配地计算 delay |
| 挂载生命周期 | `Presence` | enter/present/exit/absent，退出完成前保持挂载 |
| 测量式展开 | `MotionReveal` | 按自然高度测量并裁剪展开 |
| 组合属性 | `MotionTransform` | 位移、缩放、旋转和透明度插值 |
| 产品策略 | `MotionTokens` | 主题统一管理时长、曲线、spring 与距离 |

## Transition 与反向

每个独立运动的值都需要稳定 ID：

```rust,ignore
let opacity = gpui_base::transition(
    ("save-dialog", "opacity"),
    if open { 1.0 } else { 0.0 },
    Transition::new(tokens.duration_normal).easing(tokens.enter.clone()),
    window,
    cx,
);
```

运动途中改变目标，会从当前采样值继续。直接反向还会像 CSS Transitions 一样缩短返回时长，避免只走了 20% 却仍花完整时长返回。`transition_with_status` 额外返回 `Idle`、`Delayed`、`Running` 或 `Finished`。

`SignedDuration::negative(...)` 可表达负 delay，让动画从 active interval 中间开始。

## Easing、Timing 与 Keyframes

`Easing` 支持 CSS 的 `linear`、`ease`、`ease-in`、`ease-out`、`ease-in-out`、cubic Bézier、全部 step position，以及带省略位置补全的分段 linear stops。无效参数返回类型化错误。

`Timing` 使用绝对 elapsed time 采样，不累计帧 delta，因此掉帧不会改变动画结果。它支持有限或无限迭代，以及 normal、reverse、alternate、alternate-reverse。

```rust,ignore
let track = Keyframes::try_new([
    Keyframe::new(0.0, 0.0_f32).ease(Easing::EaseOut),
    Keyframe::new(0.6, 1.08).ease(Easing::EaseInOut),
    Keyframe::new(1.0, 1.0),
])?;

let sample = animate_keyframes(
    "success-pop",
    &track,
    Timing::new(Duration::from_millis(280)),
    window,
    cx,
);
```

offset 必须从 `0` 开始、以 `1` 结束并单调递增。采样使用二分查找。不可插值属性使用 `Discrete::new(from, to).switch_at(progress)`。

## Spring

目标会频繁变化、需要空间连续性时使用 `spring`，例如 tab 指示器、拖拽后的回落和快速切换的控件。Transition 在改目标时保持位置连续；spring 同时保持位置与速度连续。

指针拖动期间不要让值追着指针弹簧移动，可暂时使用 `with_travel(false)`，释放后再恢复。

## Presence 与退出动画

普通条件渲染会立刻卸载内容。`Presence` 将逻辑状态与实际挂载分开，提供 `Entering`、`Present`、`Exiting`、`Absent` 四个阶段，并通过 `should_render()` 指示是否仍应渲染。退出中重新打开会从当前值反向；开启 reduced motion 时会立即完成且不残留动画帧。

## 测量式展开

`MotionReveal` 保持 child 挂载，以自然尺寸测量，再按 progress 裁剪可见高度。常用入口是：

```rust,ignore
Collapsible::new()
    .motion_id("advanced-options")
    .open(show_advanced)
    .content(options)
```

必须使用稳定 ID。不调用 `motion_id` 时，`Collapsible` 保持原来的即时挂载/卸载行为。

## Stagger 与主题 Token

`Stagger` 只负责按 index 和数量计算 delay，不分配时间表，也不接管列表 identity。Styled 组件应从 `cx.theme().motion_tokens()` 读取统一策略：instant/fast/normal/slow 时长，enter/exit/move 曲线，control/move spring，以及 short/medium distance。

## Reduced motion

Transition、spring、keyframes、presence 和相关 reveal 控件都遵守 GPUI reduced-motion 偏好：有限动画直接同步到目标、更新 retained state，并且不留下待处理 frame。动画不能成为理解状态的唯一方式。

## 120 FPS 与性能

稳定采样路径零分配。Release benchmark 每批采样 1,000 个值，并对 scalar timing/easing 设置 `0.10 ms` median 上限。实现只在 delayed/active 阶段请求 frame，使用绝对时间，并对关键帧二分查找。

120 Hz 每帧约 `8.33 ms`，motion sampler 只应占很小部分；layout、paint、文字 shaping 和业务内容仍共享其余预算。能表达同一关系时，优先 opacity 与 paint transform，谨慎动画大面积 layout。

```bash
cargo bench -p gpui-base --bench motion
```

## 如何选择

- 单一目标、固定时长：`transition`
- 目标频繁变化或需要速度连续：`spring`
- 多阶段编排：`Keyframes` + `Timing`
- 列表错峰：`Stagger` + transition/keyframes
- 卸载前退出：`Presence`
- 内容展开/折叠：`MotionReveal` 或 `Collapsible::motion_id`
- 元素局部无限 spinner/skeleton：继续使用 GPUI native animation
