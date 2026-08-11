# Stateful Style Modifier and User-Owned Motion

## Status

Revised Draft

Implementation status: SM1 and SM2 are implemented for the current M1/M2 Base
controls. SM3 typed slots and SM4 interaction-transition integration remain
unimplemented and are not part of the current public contract.

本规格描述两个相互独立、可以分阶段交付的模块：

1. Stateful Style Modifier：把组件语义状态映射为应用定义的样式。
2. User-Owned Motion：为应用提供组件无关的插值与生命周期 primitive。

结论：方向可行，但不能把 GPUI interaction pseudo-state、组件语义状态和动画
当成同一种 `StyleRefinement` 简单合并。第一阶段必须使用现有 GPUI seam，并限制
在 root semantic style；slot styling 和 interaction transition 需要后续独立验证。

---

## 1. Ownership

```text
GPUI
  owns hover / active / focus / focus-visible runtime detection

gpui-component-base
  defines component semantic-state contracts
  normalizes activation and accessibility behavior
  projects the active semantic state into style modifiers
  provides generic interpolation/lifecycle primitives

Application / Registry source
  may own the controlled value (checked, selected, open, ...)
  owns every target style
  owns duration, easing, delay and animation composition
  decides which root or slot is animated
```

因此，下面这句过于绝对：

> Base owns state.

更准确的约束是：

> GPUI owns interaction-state detection. Base defines and interprets semantic
> state contracts. The application may own controlled values and always owns
> their presentation and motion policy.

Base 不得为 `Button`、`Dialog`、`Checkbox` 等具体组件内置 fade、spring、slide
或任何默认动画。

---

## 2. Goals

- 保持 Rust type safety、IDE autocomplete 和 compile-time checking。
- 普通样式继续使用 GPUI `Styled`。
- Interaction style 继续使用 GPUI 原生 `hover`、`active`、`focus`、
  `focus_visible`。
- `.styles(...)` 只增加组件可达的 semantic state modifiers。
- State modifier closure 使用 GPUI 原生 `Styled` 方法。
- State modifier closure 支持 `when`、`when_some`、`when_none`。
- 多个激活状态具有确定、可测试的 merge 顺序。
- 动画 API 与具体组件、具体 style property 解耦。
- 静态 state style 不依赖动画模块；不使用动画时不产生 retained state。

## 3. Non-Goals

第一阶段不提供：

- 新的字符串 DSL 或模板语言。
- 对 GPUI interaction style 的第二套包装。
- compound selector，例如 `checked:hover`。
- Indicator、Thumb、Track 等 slot 的跨树自动 styling。
- 任意 `StyleRefinement` 的自动 diff 或自动插值。
- Button/Dialog 等具体组件的默认动画。
- event system 或 application state mutation。

---

## 4. GPUI Facts

当前锁定的 GPUI revision 为 `cc053a4`。设计必须服从以下事实：

### 4.1 Native interaction modifiers

GPUI 已提供：

```rust
element
    .hover(|s| s.bg(hover))
    .active(|s| s.bg(active))
    .focus(|s| s.border_color(focus))
    .focus_visible(|s| s.border_color(ring))
```

公开命名必须使用 `active`，而不是另造 `pressed` 同义词。`pressed` 只适合
Toggle 等组件自身的 semantic value。

`hover`、`focus`、`focus_visible` 位于 `InteractiveElement`；`active` 位于
`StatefulInteractiveElement`。

### 4.2 Native runtime priority

GPUI 当前按以下顺序 refine，后者覆盖前者的同一属性：

```text
base
→ in_focus
→ focus
→ focus_visible
→ group_hover
→ hover
→ drag styles
→ active
```

这不是 builder 调用顺序。Stateful Style Modifier 不得宣称能够改变这条 GPUI
运行时规则。

### 4.3 FluentBuilder is not available on StyleRefinement

`StyleRefinement` 实现了 `Styled`，但没有实现 `IntoElement`，因此不会获得
GPUI 对 `IntoElement` 提供的 `FluentBuilder` 实现。下面的原生 closure 不能假定
可以调用 `.when(...)`：

```rust,ignore
.hover(|s| s.when(condition, |s| s.bg(color)))
```

Base 必须提供自己的本地 wrapper，并为该本地类型实现 `Styled` 和
`FluentBuilder`。这不创建新 style DSL；所有视觉方法仍来自 GPUI `Styled`。

### 4.4 A native pseudo-state may only have one owner

GPUI 的 `hover` 重复设置在 debug build 会触发断言。Base 无法从 crate 外安全读取
并合并已有的 private interaction refinement。因此第一阶段不得同时暴露：

```rust,ignore
button.hover(...).styles(|s| s.hover(...))
```

Interaction style 保留唯一入口：GPUI 原生 modifier。

---

## 5. Public Interface

普通 instance style 保持不变：

```rust
Button::new("save")
    .px_3()
    .py_2()
    .rounded_md()
    .bg(primary)
    .hover(|s| s.bg(primary_hover))
    .active(|s| s.bg(primary_active))
    .child("Save")
```

注意：Base Button 的第一个参数是稳定 `ElementId`，不是 label。可见内容必须通过
`.child(...)` 提供。

Semantic style 使用独立 namespace，避免与 `.disabled(bool)`、
`.checked(bool)` 等 state setter 冲突：

```rust
Checkbox::new("terms")
    .checked(checked)
    .disabled(disabled)
    .border_1()
    .rounded_sm()
    .styles(|s| {
        s.checked(|s| s.bg(primary).border_color(primary))
            .indeterminate(|s| s.bg(primary).border_color(primary))
            .disabled(|s| s.opacity(0.5))
    })
```

`.styles(...)` 第一阶段不接受普通 base style。下面的写法不支持：

```rust,ignore
.styles(|s| s.bg(background).checked(...))
```

原因是它会与外层 `Styled` 形成两个 instance-style 入口，并使跨 namespace 的
last-call-wins 无法兑现。普通样式始终写在组件本身。

---

## 6. Typed State Contexts

不能让所有组件都获得所有 semantic modifier。以下代码应在编译期失败：

```rust,compile_fail
Button::new("save").styles(|s| s.checked(|s| s.bg(primary)));
Slider::new("volume").styles(|s| s.open(|s| s.bg(surface)));
```

推荐使用 capability-based context：

```rust,ignore
pub struct StateStyle {
    refinement: gpui::StyleRefinement,
}

impl gpui::Styled for StateStyle {
    fn style(&mut self) -> &mut gpui::StyleRefinement {
        &mut self.refinement
    }
}

impl gpui::prelude::FluentBuilder for StateStyle {}

pub struct ComponentStyles<C> {
    states: C,
}

pub trait DisabledStyle {
    fn disabled(self, build: impl FnOnce(StateStyle) -> StateStyle) -> Self;
}

pub trait CheckedStyle {
    fn checked(self, build: impl FnOnce(StateStyle) -> StateStyle) -> Self;
    fn indeterminate(self, build: impl FnOnce(StateStyle) -> StateStyle) -> Self;
}
```

组件只实现它实际支持的 capability。具体内部类型可以使用 marker、宏或私有字段，
但 interface 不应暴露无效状态。

State modifier 内的条件组合可编译：

```rust
.styles(|s| {
    s.checked(|s| {
        s.bg(primary)
            .when(high_contrast, |s| s.border_2())
            .when_some(override_color, |s, color| s.bg(color))
    })
})
```

---

## 7. Semantic State Priority

Base 无法识别 Registry 中的 “variant layer” 或 “size layer”。静态样式顺序由
Registry 源码显式 refine，遵循现有 closest/instance style contract。

每个组件必须声明自己的 semantic priority。共同规则：

1. Registry/default/variant/size 先形成静态样式。
2. Instance `Styled` refinement 是最后一个静态 application layer。
3. 激活的 semantic refinements 按组件声明的固定顺序应用。
4. GPUI interaction refinements按 GPUI 固定运行时顺序应用。

MVP 顺序：

```text
Button:   instance → disabled → GPUI interaction
Checkbox: instance → checked → indeterminate → disabled → GPUI interaction
```

`indeterminate` 与 `checked` 在规范化后的 Checkbox state 中互斥。`disabled` 最后
应用，因此在 semantic refinements 写同一属性时胜出。

Disabled 组件默认不安装 activation handler，但 GPUI 不会自动抑制 hover appearance。
由于原生 interaction refinement 在 semantic refinement 之后应用，应用若不希望 disabled
控件出现 hover/active 外观，必须在 build 时显式 gate 原生 modifier：

```rust
.when(!disabled, |this| {
    this.hover(|s| s.bg(hover)).active(|s| s.bg(active))
})
```

在 interaction modifier 仍由 GPUI 直接拥有的 MVP 中，Base 不能从 crate 外移除或重写
已经注册的 private hover style。

多个状态修改不同属性时全部保留；修改同一属性时按上述顺序决定结果，而不是按
`.styles(...)` 中的书写顺序。

---

## 8. Root and Slot Scope

MVP 只处理 root style。它不能自动改变 child slot：

```text
Checkbox.Root / Checkbox.Indicator
Switch.Root   / Switch.Thumb
Slider.Root   / Slider.Track / Slider.Range / Slider.Thumb
```

这与 shadcn 的分层一致：primitive 暴露 state 和 parts，Registry 源码决定每个 part
的结构和视觉。完整 slot styling 需要后续 typed part interface/state projection，不能用
`AnyElement` child 猜测或下钻样式。

在 slot module 完成之前，Registry 可以使用自己已有的 controlled value来构造 child：

```rust
let indicator = div().when(checked, |this| this.child(CheckIcon));

Checkbox::new("terms")
    .checked(checked)
    .styles(|s| s.checked(|s| s.bg(primary)))
    .child(indicator)
```

这不是重新实现 behavior；它只是应用使用自己拥有的 controlled snapshot来定义 slot
presentation。

---

## 9. User-Owned Motion

### 9.1 Principle

动画不是组件行为默认值：

```text
Base component
  does not know duration
  does not choose easing
  does not choose fade / slide / spring
  does not choose which slot animates

Application / Registry
  supplies target values
  selects motion policy
  applies sampled values to its own root or slots
```

静态 state style 与 motion 必须解耦。删除所有 motion 配置后，组件仍应具有完整且
正确的最终状态视觉。

### 9.2 Generic value transition

建议新增组件无关的 retained-value seam：

```rust,ignore
pub struct Transition {
    duration: Duration,
    delay: Duration,
    easing: Rc<dyn Fn(f32) -> f32>,
}

impl Transition {
    pub fn new(duration: Duration) -> Self;
    pub fn delay(self, delay: Duration) -> Self;
    pub fn ease(self, easing: impl Fn(f32) -> f32 + 'static) -> Self;
}

pub trait Interpolate: Clone + 'static {
    fn interpolate(&self, target: &Self, progress: f32) -> Self;
}

pub struct TransitionId {
    element: ElementId,
    channel: Option<SharedString>,
}

// Direct convenience conversions cover the common scalar forms.
impl From<&'static str> for TransitionId { /* ... */ }
impl From<String> for TransitionId { /* ... */ }
impl From<SharedString> for TransitionId { /* ... */ }
impl From<usize> for TransitionId { /* ... */ }
impl From<i32> for TransitionId { /* ... */ }
impl From<ElementId> for TransitionId { /* ... */ }

// A tuple creates a named animation channel below an element identity.
impl<E, C> From<(E, C)> for TransitionId
where
    E: Into<ElementId>,
    C: Into<SharedString>,
{
    /* ... */
}

pub fn transition<T: Interpolate>(
    id: impl Into<TransitionId>,
    target: T,
    policy: Transition,
    window: &mut Window,
    cx: &mut App,
) -> T;
```

因此常见调用不需要显式构造 ID：

```rust,ignore
transition("dialog-opacity", target, policy, window, cx);
transition(("terms", "fill"), target, policy, window, cx);
transition((component_id.clone(), "thumb-x"), target, policy, window, cx);
```

标量适合只有一个 transition channel 的位置；tuple 表示
`(element identity, channel name)`，用于同一组件同时过渡多个属性或 slot。内部仍统一
转换为 `TransitionId`，确保 keyed state 不会碰撞。UUID、FocusHandle、path 等较少见
的 identity 不重复实现转换，调用者先转换成原生 `ElementId` 即可。

这里采用 CSS/Tailwind 的 `transition` 术语：target style 独立存在，transition 只描述
旧 target 到新 target 之间的 duration、delay 和 timing function。`animation` 保留给
keyframes、循环或其他非 target-change 驱动的动画。

`transition` 的 module implementation 负责：

- 以 `ElementId` 保存上一次 sampled value 和最新 target。
- target 改变时从当前 sampled value开始，而不是从旧 target 跳变。
- 根据 `Transition` 调度 redraw。
- 在 duration 完成后释放 active animation work。
- reduce-motion policy 生效时立即返回 target。

它不读取 Theme，不知道 Button/Dialog，也不产生 `StyleRefinement`。

应用完整定义 Checkbox 动画：

```rust,ignore
let fill = transition(
    ("terms", "fill"),
    if checked { primary } else { surface },
    Transition::new(Duration::from_millis(120)).ease(ease_out_cubic),
    window,
    cx,
);

let mark_opacity = transition(
    ("terms", "mark-opacity"),
    if checked { 1.0 } else { 0.0 },
    Transition::new(Duration::from_millis(90)),
    window,
    cx,
);

Checkbox::new("terms")
    .checked(checked)
    .bg(fill)
    .child(CheckIcon.opacity(mark_opacity))
```

应用完整定义 Dialog 动画；Dialog 本身不携带默认 motion：

```rust,ignore
let opacity = transition(
    (dialog_id.clone(), "opacity"),
    if open { 1.0 } else { 0.0 },
    fade_motion,
    window,
    cx,
);

let offset = transition(
    (dialog_id, "offset-y"),
    if open { px(0.) } else { px(12.) },
    slide_motion,
    window,
    cx,
);

Dialog::new(...)
    .opacity(opacity)
    .top(offset)
```

### 9.3 Supported values

首批 `Interpolate` 只应覆盖有清晰数学语义的类型：

- `f32`
- `Pixels`
- `Point<Pixels>`
- `Hsla`，并明确 hue interpolation policy

不要对整个 `StyleRefinement` 自动插值。Display、layout mode、font family、cursor、
children 和 accessibility properties 等不是普遍可插值值。

### 9.4 Interaction animation limitation

`hover`/`active` 是 GPUI 在 paint 时解析的 runtime state，普通 component render 目前拿不到
稳定的 target snapshot。因此 `transition` 首批只承诺 application/semantic state
transition，例如 checked、selected、open。

Hover/active animation 需要先完成 spike，在以下两种 seam 中选择一种：

1. GPUI 提供只读 interaction-state signal；或
2. Base 提供不改变 event propagation 的通用 interaction observer。

在证明 state edge、redraw 和事件顺序以前，不冻结 `.styles(...).transition(...)` API。

### 9.5 Relationship with existing GPUI animation

GPUI `Animation` / `with_animation` 仍是底层 frame driver。新的
`motion::Transition` 只描述 duration/delay/easing。原有
`animation::Transition` 及其 `.fade()`、`.slide_y()` 为 legacy compatibility
interface，两者必须保持不同类型和模块路径，避免 effect 被 value transition 静默忽略。
新 Base 组件不得调用 legacy effect combinator 来安装默认视觉动画。

---

## 10. Milestones

### SM1 — Semantic root styles

- Button: `disabled`。
- Checkbox: `checked`、`indeterminate`、`disabled`。
- Typed state contexts；无无效 modifier。
- `StateStyle: Styled + FluentBuilder`。
- 确定且经过测试的 semantic priority。
- 不包装 GPUI interaction modifiers。

### SM2 — Generic semantic value transition

- `Transition`、`Interpolate`、`TransitionId`、`transition`。
- target reversal 从当前 sampled value平滑继续。
- stable `ElementId` lifecycle。
- reduce-motion 和 zero-duration behavior。
- 无任何具体组件默认动画。

### SM3 — Typed slots

- Checkbox Indicator。
- Switch Thumb。
- Slider Track/Range/Thumb。
- state projection 不依赖 `AnyElement` introspection。

### SM4 — Interaction transition spike

- 证明 hover/active edge 可观测。
- 证明不会重复注册 GPUI `hover`。
- 证明不会改变 pointer/focus/event propagation。
- spike 通过后再决定 public interface；失败则继续使用 GPUI 原生静态 pseudo-style。

---

## 11. Acceptance Criteria

### Compile-time

- `.styles(...)` 不改变现有 `Styled` interface。
- State closure 内 `bg`、`border_color`、`opacity` 等 GPUI methods 可用。
- State closure 内 `when`、`when_some`、`when_none` 可用。
- Button 不暴露 `checked/open`；Checkbox 不暴露 `open/selected`。
- 使用 `active`、`focus_visible` 等 GPUI/RFC 一致命名。

### Runtime style

- normal、checked、indeterminate、disabled 和 checked+disabled 有精确测试。
- 不同状态修改不同属性时均保留。
- 同属性冲突遵循文档固定 priority。
- `.styles(...)` 调用位置不改变 semantic priority。
- Instance/static style contract 与现有 facade 保持一致。

### Behavior and compatibility

- Modifier 不改变 click、keyboard、focus、event propagation 或 ARIA。
- 不增加 wrapper element，不改变 `ElementId`。
- Base 不依赖 facade、Registry 或 Theme。
- legacy facade 的行为、交互、设计、功能和公开 interface 100% 保持。

### Motion

- 没有 motion 时直接呈现正确 target。
- duration、delay、easing 完全由应用提供。
- Base Button/Dialog/controls 不包含默认 motion。
- target 在进行中反转时无跳变。
- 动画完成后不继续请求 frame。
- reduce-motion 立即稳定到 target。
- application 可对不同 slots 使用不同 motion。

---

## 12. Decision Summary

1. 普通样式继续写在组件本身，不在 `.styles(...)` 中复制入口。
2. GPUI interaction style 继续使用原生 `hover/active/focus/focus_visible`。
3. `.styles(...)` 第一阶段只处理 typed semantic states。
4. `StateStyle` 是本地 wrapper，以获得 `Styled + FluentBuilder`；视觉方法仍来自 GPUI。
5. Semantic priority 按组件固定，不按 closure 书写顺序。
6. 第一阶段只承诺 root；完整 presentation 需要 typed slots。
7. Motion 是应用拥有的 policy；Base 只提供组件无关的插值和生命周期 primitive。
8. 不对整个 `StyleRefinement` 自动插值，不给具体组件预装动画。
9. Interaction transition 在 spike 证明可行前不冻结公开 interface。

核心原则：

> Extend GPUI with typed semantic-state styling; do not replace its style or
> interaction system.

以及：

> Base provides motion mechanics. Applications own every animation decision.
