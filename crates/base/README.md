# gpui-base

[![Crates.io](https://img.shields.io/crates/v/gpui-base.svg)](https://crates.io/crates/gpui-base)
[![Documentation](https://docs.rs/gpui-base/badge.svg)](https://docs.rs/gpui-base)
[![License](https://img.shields.io/crates/l/gpui-base.svg)](../../LICENSE-APACHE)

`gpui-base` 是 [GPUI Component](https://github.com/longbridge/gpui-component) 的基础层，面向需要自行构建设计系统的 GPUI 应用。它提供交互行为、焦点管理、无障碍语义、动画、虚拟列表及主题令牌等基础能力，但不规定组件的视觉风格。

> 如果你希望直接使用带有完整外观的组件，请使用 [`gpui-component`](https://crates.io/crates/gpui-component)。如果你希望应用拥有组件源码和视觉样式，并复用稳定的底层行为，请使用 `gpui-base`。

## 在 GPUI Component 中的位置

`gpui-component` 是项目和生态品牌，`gpui-base` 是其中可独立复用的 foundation crate：

```text
gpui-component
├── gpui-base          交互、状态和基础设施（本 crate）
├── gpui-component     带完整样式的组件库及兼容入口
├── registry           可复制到应用中维护的组件源码
├── blocks             更高层的界面组合
└── CLI                项目初始化和组件安装工具
```

依赖方向始终是从上层到基础层：`gpui-base` 不依赖 `gpui-component`。现有应用可以继续使用 `gpui-component`；只有在开发自有组件或设计系统时，才需要直接依赖 `gpui-base`。

## 设计原则

- **行为归基础层**：处理点击、键盘激活、受控状态、焦点、无障碍角色和基础设施。
- **视觉归应用层**：布局、尺寸、颜色、间距、圆角、边框、阴影、变体和动画由应用或上层组件决定。
- **组件由应用拥有**：基础控件可以直接组合和修改，不要求应用接受一套固定视觉语言。
- **语义优先**：主题提供 `primary`、`surface`、`destructive` 等语义令牌，而不是不断增加组件专用字段。
- **遵循 GPUI**：控件实现 `Styled`、`ParentElement` 等 GPUI 接口，可继续使用 GPUI 的 fluent builder API。

因此，`Button::new("save")` 默认没有内边距、背景色、圆角或尺寸。这里的“无样式”是明确的 API 契约，而不是缺失功能。

## 安装

从 crates.io 使用当前版本：

```toml
[dependencies]
gpui-base = "0.5.2"
gpui = { git = "https://github.com/zed-industries/zed" }
gpui_platform = { git = "https://github.com/zed-industries/zed", features = ["font-kit"] }
```

跟随仓库开发分支时，也可以直接使用 Git 依赖：

```toml
[dependencies]
gpui-base = { git = "https://github.com/longbridge/gpui-component" }
gpui = { git = "https://github.com/zed-industries/zed" }
gpui_platform = { git = "https://github.com/zed-industries/zed", features = ["font-kit"] }
```

`gpui-base` 与仓库使用同一 GPUI 版本。若 Cargo 报告 GPUI 类型不匹配，请检查应用是否引入了不同 revision 的 GPUI。

### 可选功能

| Feature | 默认启用 | 作用 |
| --- | --- | --- |
| `inspector` | 否 | 同时启用 `gpui` 和 `gpui_macros` 的 inspector 支持 |

## 初始化

请在创建窗口或使用基础控件之前调用一次 `gpui_base::init(cx)`。它会安装基础层所需的全局主题和焦点陷阱基础设施。

```rust
use gpui::*;

fn main() {
    gpui_platform::application().run(|cx| {
        gpui_base::init(cx);

        // 在此之后创建窗口和视图。
    });
}
```

如果应用已经调用 `gpui_component::init(cx)`，则不必再调用 `gpui_base::init(cx)`：上层初始化函数已经包含基础层初始化。

## 快速示例

基础控件可以像普通 GPUI 元素一样设置样式和添加子元素：

```rust
use gpui::{Context, IntoElement, Render, Window, div, px, rgb};
use gpui::prelude::*;
use gpui_base::Button;

struct SaveButton;

impl Render for SaveButton {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        Button::new("save")
            .px_3()
            .py_2()
            .rounded(px(6.))
            .bg(rgb(0x2563eb))
            .text_color(rgb(0xffffff))
            .accessibility_label("保存文档")
            .on_click(|_, _, _| println!("save"))
            .child("保存")
    }
}
```

`ElementId` 必须在同一视图中保持稳定，以便 GPUI 保存焦点和元素状态。`Button` 会统一处理鼠标、Enter 和 Space 激活，并允许通过 `disabled`、`selected`、`tab_index` 和 `tab_stop` 配置语义状态及焦点遍历。

### 受控状态

`Checkbox`、`Radio`、`Switch` 和 `Toggle` 都是受控组件。回调只报告下一个值；应用需要更新自己的状态，并在下一次渲染时把它传回组件：

```rust
use gpui::{Context, IntoElement, Render, Window};
use gpui::prelude::*;
use gpui_base::{Checkbox, CheckboxIndicator};

struct Settings {
    telemetry: bool,
}

impl Render for Settings {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let checked = self.telemetry;
        let settings = cx.entity().downgrade();

        Checkbox::new("telemetry")
            .checked(checked)
            .accessibility_label("发送匿名使用数据")
            .on_change(move |state, _, cx| {
                _ = settings.update(cx, |this, cx| {
                    this.telemetry = state == gpui_base::CheckboxState::Checked;
                    cx.notify();
                });
            })
            .child(
                CheckboxIndicator::new()
                    .checked(checked)
                    .child(if checked { "✓" } else { "" }),
            )
            .child("发送匿名使用数据")
    }
}
```

语义状态的视觉效果也由调用方定义。以按钮为例：

```rust
Button::new("menu-trigger")
    .selected(menu_open)
    .disabled(is_busy)
    .styles(|styles| {
        styles
            .selected(|style| style.bg(rgb(0xe2e8f0)))
            .disabled(|style| style.opacity(0.5))
    })
    .child("菜单")
```

调用链上直接设置的样式具有最高优先级；语义状态样式负责表达 checked、pressed、selected、indeterminate 或 disabled 等状态。

## 能力概览

### 无样式控件

| API | 负责的行为 |
| --- | --- |
| `Button` | 点击、键盘激活、焦点、禁用和选中状态、Button 无障碍角色 |
| `Checkbox` / `CheckboxIndicator` | checked、unchecked、indeterminate 三态及对应无障碍语义 |
| `Radio` / `RadioGroup` | 单选激活、焦点和分组容器 |
| `Switch` / `SwitchTrack` / `SwitchThumb` | 受控开关及可独立设计的轨道、滑块部件 |
| `Toggle` / `ToggleGroup` | 受控 pressed 状态及组合容器 |
| `Link` | Link 语义和激活；导航策略由应用通过 `open_with` 注入 |

基础层不会自行打开 URL。这样同一 `Link` 可以用于内部路由、嵌入式 WebView 或系统浏览器。

### 焦点和交互

- `FocusTrapElement`：把交互元素变为焦点陷阱，Tab / Shift-Tab 会在容器内循环。
- `active_focus_trap`：查询当前窗口中激活的焦点陷阱。
- `InteractiveElementExt`：交互元素的扩展行为。
- `ElementExt`：布局完成后、prepaint 阶段的观察扩展。
- `FocusableExt`：按应用主题绘制焦点环。

### 滚动和大数据

- `Scrollbar`：适配 `ScrollHandle`、`UniformListScrollHandle`、`ListState` 和 `VirtualListScrollHandle`，支持水平、垂直及双轴滚动条。
- `ScrollbarMode`：控制滚动条的显示策略。
- `v_virtual_list` / `h_virtual_list`：只渲染可见区域，且允许每一项拥有不同尺寸。
- `VirtualListScrollHandle`：读取或更新虚拟列表滚动位置。
- `AutoScroll`：为拖拽场景提供靠近边缘时的定时自动滚动。

虚拟列表要求调用方提供各项尺寸；纵向列表使用每项高度，横向列表使用每项宽度。它与 GPUI 的 `uniform_list` 不同，适用于高度或宽度不一致的数据。

### 动画

`gpui-base` 提供两类动画接口：

- `motion::transition`：推荐的值过渡接口。调用方选择要动画的属性，支持 duration、delay、自定义 easing、目标反转及 reduce-motion。
- `animation::Transition`：兼容既有代码的元素动画接口，可组合 fade、slide 和 size 效果。

基础控件不会自动安装动画。应用可按自己的视觉语言选择动画属性和时序。

### 主题和样式

- `Theme`：基础层全局配置，包含语义令牌和滚动条默认值。
- `SemanticThemeTokens`：由 `colors`、`radius`、`spacing`、`typography` 和 `shadow` 组成。
- `StateStyle`：可与 `when`、`when_some` 等 fluent builder 方法组合的语义状态样式。
- `StyledExt`：提供 `h_flex`、`v_flex`、边距/内边距、字体粗细、调试边框等通用样式扩展。
- `h_flex` / `v_flex` / `box_shadow`：常用元素和样式构造函数。

可以在初始化后修改全局基础主题：

```rust
use gpui::{px, rgb};
use gpui_base::Theme;

let theme = Theme::global_mut(cx);
theme.tokens.colors.primary = rgb(0x2563eb).into();
theme.tokens.radius.md = px(8.);
```

令牌只描述设计语义，不会自动给无样式控件添加外观。应用需要在自己的组件实现中读取并使用这些令牌。

### 通用数据和布局工具

| API | 用途 |
| --- | --- |
| `History` / `HistoryItem` | 带分组、去重和容量限制的 undo / redo 历史 |
| `SliderState` | 单值或范围值、线性/对数刻度及 Slider 事件状态 |
| `IndexPath` | 表示 section、row、column 的索引路径 |
| `Placement` / `Side` | 弹出层和布局方向描述 |
| `AxisExt` / `LengthExt` / `Edges` | GPUI 几何类型的扩展及可序列化边距 |

## 与 gpui-component 的关系

两者面向不同抽象层，可以在同一个应用中同时使用：

| | `gpui-base` | `gpui-component` |
| --- | --- | --- |
| 定位 | 行为和基础设施 | 完整 UI 组件库 |
| 默认外观 | 无 | 有 |
| 视觉样式所有者 | 应用 | 组件库，可通过 Theme 和 API 定制 |
| 适用场景 | 自建设计系统、Registry 组件、底层复用 | 快速构建完整桌面应用 |
| 初始化 | `gpui_base::init(cx)` | `gpui_component::init(cx)`，内部包含 Base 初始化 |

从 `gpui-component` 迁移时，不应机械地把所有 import 替换为 `gpui-base`。例如 `gpui_component::button::Button` 是带完整外观的上层组件，而 `gpui_base::Button` 是要求调用方提供子元素和全部样式的基础控件。

## 支持平台

平台支持跟随 GPUI 和 GPUI Component：

- macOS（Apple Silicon、Intel）
- Linux（x86_64）
- Windows（x86_64）
- WebAssembly 支持取决于所使用的具体 API 和 GPUI Web 运行时

## 开发与验证

在 GPUI Component 仓库根目录执行：

```bash
# 检查基础 crate
cargo check -p gpui-base

# 运行基础 crate 测试
cargo test -p gpui-base

# 检查格式
cargo fmt --check

# 运行 Clippy
cargo clippy -p gpui-base -- --deny warnings
```

`gpui-base` 仍在随 GPUI Component 的基础层重构持续演进，目前仅作为 workspace 内部 crate 使用。当前 Rust API 以源码为准，架构迁移进度记录在 [`../../specs/BASE-TODO.md`](../../specs/BASE-TODO.md)。

## 相关资源

- [GPUI Component 仓库](https://github.com/longbridge/gpui-component)
- [GPUI Component 文档](https://longbridge.github.io/gpui-component)
- [`gpui-component` crate](https://crates.io/crates/gpui-component)
- [`gpui-base` API 文档](https://docs.rs/gpui-base)
- [GPUI](https://gpui.rs)
- [贡献指南](../../CONTRIBUTING.md)

## 许可证

Apache-2.0，详见 [`../../LICENSE-APACHE`](../../LICENSE-APACHE)。
