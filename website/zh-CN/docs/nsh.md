---
title: NSH
description: 为 NSH 应用提供实验性的 GPUI Base 和 Component 绑定
---

# NSH

实验性的 `gpui-kit-nsh` crate 让 NSH 应用使用真正的 GPUI Component
控件。TSX 组合、signal 和 JavaScript 运行时由 NSH 管理，适配器通过
GPUI Fast 渲染原有的 Rust 组件。目前该 crate 只能从本仓库使用，尚未发布。
它使用独立的 workspace 和 lockfile，避免必需的 GPUI Fast feature 影响
父 workspace 中现有 crate 的后端选择。

目前已为全部 70 个 Component UI 模块提供部分绑定，覆盖输入、集合、布局、
图表、Dock 和窗口浮层等组件族。完整公共 API 对等仍在逐项审核。
`partial` 表示已有可用绑定，但尚未证明所有 Rust builder、callback 与组合路径
逐项等价，并不表示整个模块不可用。
已实现真实 children 与命名 slot 桥接、宿主主题同步及标准 NSH
run/package 运行器，正在进行集成验证；不能将这些源码入口视作已通过全部行为验证。适配器面向 macOS、Windows 和 Linux
上的 GPUI 桌面后端；NSH 的其他后端无法渲染这些控件。

NSH 依赖来自真实的 NSH 源码 checkout，本仓库不 vendor NSH。引擎和运行器
修改在 NSH 仓库维护。本地需要同级的 `gpui-kit/` 与 `nsh/` checkout。
请使用 `.github/workflows/ci.yml` 中 NSH 任务固定的 revision，确保源码版本匹配。
CI 会将该版本与 GPUI Kit 放在同级目录并验证两套适配器；本仓库不保存 NSH 源码快照。

绑定采用由 Rust 持有的 native specs。Rust 的 `Props` 和 `Event` 声明接入
NSH 已有的 `Registry`；同一注册同时提供原生工厂、运行时导出和
`nsh/ui/gpui` 的 TypeScript 声明。组合、受控值回写和批次校验统一由 NSH 的
Rust renderer 处理。普通叶子控件使用小型共享 macro 和原有 Rust builder；
有状态控件保留确实需要的 Entity、订阅及生命周期处理。适配器复用 Component
的渲染与样式。

不再逐组件手写 TS 包装。兼容入口 `crates/nsh/js/index.ts` 只转发生成导出，
并从生成的 props 推导旧的辅助类型，不维护第二份组件 schema。
减少的是重复维护；复杂控件仍需要 Rust 适配，Rust 总行数也不一定下降。

七组件基线、通用组合和受控值拒绝已通过定向运行检查。生成式 CLI 也已通过
类型生成、有效和无效类型检查、HMR 及 macOS dev 打包验证。
完整迁移清单已通过编译和生成类型检查。交互验证单独记录，源码注册不代表
完整组件 API 已逐项对等。

## 对比 Base 方案

两套实现独立保留。`crates/nsh` 通过 `nsh/ui/gpui` 使用有样式的 Component；
`crates/nsh-base` 通过 `nsh/ui/base` 暴露无样式 Base 行为，其依赖图未启用
Component 或 Kit assets。Base 示例的颜色、字体和组合由 NSH 管理。

Base 方案目前从 Button、Checkbox 和 Input 开始，并有独立的真实引擎 UI
验证器，不宣称已暴露所有 Base API。两个相同业务流程的示例分别位于
`crates/nsh/examples/compare` 与 `crates/nsh-base/examples/app`，都有任务名称、
优先级、校验、提交、重置和焦点恢复，便于对比实际 UI。

在仓库根目录运行 Base UI 验证器：

```sh
script/test-nsh-base-ui
```

脚本会构建 Base CLI 和宿主，编译原始任务示例，并通过真实 NSH/GPUI 验证
指针、键盘、受控状态和生命周期。截图保存在 `target/nsh-base-verify`。
目前需要 macOS 或 Linux、可用的 GPUI 无窗口渲染器，以及匹配的同级 NSH 源码。

脚本完成后，可用 Base 自己的宿主打开示例：

```sh
cargo run --manifest-path crates/nsh-base/Cargo.toml --features dev,cli,test-support --bin nsh-gpui-host -- --bundle target/nsh-base-task-app/bundle.nshc
```

按下文安装 Component runtime 后，可打开相同业务的 Component 示例：

```sh
target/nsh-runtime/gpui-kit-nsh build crates/nsh/examples/compare -o target/nsh-component-compare
target/nsh-runtime/gpui/nsh-gpui-host --bundle target/nsh-component-compare/bundle.nshc
```

## 运行示例

在仓库根目录安装已注册 Kit UI set 的 NSH CLI 及配套运行器：

```sh
cargo run --manifest-path crates/nsh/Cargo.toml --bin gpui-kit-nsh-runtime -- --manifest-path crates/nsh/Cargo.toml --profile dev target/nsh-runtime
target/nsh-runtime/gpui-kit-nsh check crates/nsh/examples/app
target/nsh-runtime/gpui-kit-nsh build crates/nsh/examples/app -o target/nsh-example
cargo run --manifest-path crates/nsh/Cargo.toml --example nsh -- target/nsh-example/bundle.nshc
```

示例宿主加载可信的本地构建 bundle，初始化 GPUI Component，并通过
`gpui_kit::open_window` 打开窗口。`gpui-kit-nsh` 在 NSH 原有 CLI 中注册
Kit UI set，并自动发现可执行文件旁的配套运行器；显式设置 `NSH_RUNTIME` 时仍
优先使用该目录。另行安装的全局 `nsh` 不会自动包含 Kit UI set，只选择 GPUI
target 不会注册此 catalog。

可以直接导入生成的 TSX 绑定：

```tsx
import { signal } from "nsh";
import { Button, Input } from "nsh/ui/gpui";

const name = signal("");

<Input
  value={name()}
  accessibilityLabel="Name"
  onChange={(value) => name.set(value)}
  style={{ width: 280, height: 36 }}
/>;

<Button
  label="Clear"
  disabled={name() === ""}
  onPress={() => name.set("")}
  style={{ width: 120, height: 36 }}
/>;
```

CLI 从 Rust 声明生成 `nsh/ui/gpui` 模块和编辑器类型，无须独立的 JavaScript
组件包。迁移期间，仓库的 `crates/nsh/js/index.ts` 保留为兼容入口。
请在 NSH 布局中为每个原生控件指定明确尺寸，目前尚未实现固有尺寸协商。

NSH signal 更新时，Input 保留原有编辑状态。同值回写不会调用
`set_value`，因此会保留当前选区与撤销历史。外部传入不同值时，则按
Input 原有行为重置编辑状态。Input 还提供 `onFocus`、`onBlur` 和
`onSubmit`，其 NSH 节点 ref 支持 `command("focus")`。

Checkbox 和 Switch 使用受控状态：传入 `checked`，并将 `onChange`
收到的布尔值写回所属 signal。Button、Checkbox、Switch、Input、Select 和 Slider 支持 `disabled`；Input 另支持
`readonly` 与 `placeholder`。

## 使用标准 NSH 命令

先把 Kit 的开发与打包运行器安装到独立 runtime 目录，再让 NSH CLI 使用它：

```sh
cargo run --manifest-path crates/nsh/Cargo.toml --bin gpui-kit-nsh-runtime -- --manifest-path crates/nsh/Cargo.toml --profile dev target/nsh-runtime
target/nsh-runtime/gpui-kit-nsh run crates/nsh/examples/app --target gpui
target/nsh-runtime/gpui-kit-nsh package crates/nsh/examples/app --target gpui -o target/nsh-package
```

安装器创建 `gpui-kit-nsh`、`gpui/nsh-gpui-host` 与 `gpui/nsh-gpui-app`，Windows 上带 `.exe`。
`dev` profile 用于本地验证；省略 `--profile dev` 时安装器默认构建 release 运行器。
开发宿主使用 NSH dev server 和热更新；打包宿主加载 NSH 的标准资源布局。
打包格式沿用 NSH：macOS `.app`、Linux tar.gz、Windows zip。
macOS 的 dev profile 已实际验证初次模块加载、源修改后的 HMR 模块执行、
.app 打包及从 /tmp 启动包内 runner 加载自身资源。日志在
`target/nsh-typed-rust-runtime/verification.log`。这不代表 signal 保留或完整 GUI 交互
已经验证；release 优化及 Windows/Linux 运行尚未验证。

此流程适用于普通 NSH TSX 应用。若 `nsh.json` 指定自定义 `rust` crate，
现有 NSH CLI 会优先生成其 Rust runtime，绕开 `NSH_RUNTIME`；该生成路径尚未接入
Kit Root/catalog。自定义 Rust 宿主可以显式调用 `gpui_kit_nsh::host::run`。

Kit 宿主调用 `gpui_kit::init`，通过 `gpui_kit::open_window` 安装 Root，
注册 Component catalog，并将 NSH appearance 同步到 Component theme。
它保留 NSH 自己的 JavaScript 运行时，未引入 gpui-shell。

## Select 和 Slider

Select 接收 `items: { value: string; label: string; disabled?: boolean }[]`
及受控的 `value: string | null`。value 必须唯一，并在 label 变化、过滤和
重排后保持稳定。`onChange` 返回选中项的 value，清空时返回 `null`。
`searchable` 启用内置搜索，`cleanable` 启用清空操作。这些属性或选项数据
变化时，适配器保留已有列表与焦点句柄。节点 ref 支持 `command("focus")`。

Slider 接收数值或有序的 `[start, end]` 区间。`onChange` 持续报告用户
调整的值，`onRelease` 报告用户松手时的值。`min`、`max`、`step`、
`scale`（`linear` 或 `logarithmic`）、`vertical` 和 `reverse` 对应 Rust
组件的同名能力。数值会限制在当前边界内。边界必须有限，并满足 `min < max`
与 `step > 0`；对数刻度还要求 `min` 为正，且 `max/min` 的比值有限。
属性更新作用于已有状态，不会创建新的 Entity 或订阅。程序设置数值不会发出用户事件。

Select 和 Slider 通过 `onError` 报告无效选项、数值或配置。无效更新会保留
上一个可用的原生状态。Select 提供 `item:<value>` 和 `value:<value>` 命名 slot，使用真实 NSH JSX
渲染选项与已选值，并保留内容原有的 NSH owner。

## 组件接入清单

源码中的适配器已扩展到以下组件族。各组件模块仍标记为 `partial`：
声明、注册、实际运行和完整 API 对等是不同的验收项。下表列出实现范围，
不将尚未执行的逐组件交互或跨平台验证写为通过。

| Family | Bindings |
| --- | --- |
| Controls | Button, ButtonGroup, Toggle, ToggleGroup, Checkbox, Switch, Radio, RadioGroup, Rating, Slider |
| Input | Input, Textarea, Editor, NumberInput, OtpInput |
| Display | Icon, Avatar, AvatarGroup, Badge, Tag, Alert, Marker, Spinner, Skeleton, ShimmerText, Progress, ProgressCircle, Separator, Kbd, Label, Clipboard, Link |
| Collections | List, Table, DataTable, Tree, Combobox, Command, Pagination |
| Date and time | Calendar, DatePicker, TimeField |
| Navigation and layout | Accordion, Collapsible, TabBar, Tabs, Sidebar, SidebarHeader, SidebarFooter, Breadcrumb, Form, Field, GroupBox, DescriptionList, Settings, Toolbar, ToolbarGroup, StatusBar, Empty, Carousel |
| Dock and resize | DockArea, Resizable |
| Scrolling | ScrollArea, VirtualList, HorizontalVirtualList |
| Content | TextView, Markdown, Html, CodeBlock, Diff, Image |
| Chart and plot | LineChart, AreaChart, BarChart, PieChart, RadarChart, CandlestickChart, SankeyChart, Plot |
| Messaging | Message, MessageGroup, Bubble, BubbleGroup, Attachment, AttachmentGroup, MessageScroller, Questionnaire |
| Window | Dialog, Sheet, Notification, TitleBar, WindowBorder |
| Popups | Popover, HoverCard, Tooltip, ContextMenu, DropdownMenu, NativeMenu, DropdownButton |
| Color and progress | ColorPicker, ColorSelect, Stepper |
| Speech | SpeechButton, SpeechWaveform |

`Image` 使用 GPUI 的 `img`，在清单中单独分类，不计作 Component 模块。
Speech 需要对应系统后端与权限；API 存在不表示运行环境具备识别能力。
Editor 已实现异步 JavaScript language service 请求与响应通道，运行验证仍在进行；
token:<id> slot 可渲染真实 NSH JSX，showDocument 通过事件交还 NSH。
现有 Base 的 resolve_completions 没有可用调用路径，绑定未伪造该能力；
完整 LSP CompletionItem 字段可直接由 completion 响应返回。Plot 已接入原生 scale、axis、grid 和 line/area/bar/pie/arc primitives；
任意 Rust geometry callback 不直接序列化为 JavaScript。
完整属性、事件和 ref command 以 `crates/nsh/js/*.ts` 的声明及适配器实现为准。

使用 `bun script/check-nsh-catalog.ts --report target/nsh-api-audit.json`
生成公开 Rust 声明、可渲染组件及 part、每个绑定的 props/events/commands 源证据。
报告中尚未逐项审核的内容保留 `unreviewed`；同名组件不代表全部行为已对等。
`--require-complete` 会拒绝任何 `partial` 或 `pending` 模块，防止以注册数量作为完成条件。

## 窗口浮层

Dialog 和 Sheet 通过 `open` 受控显示。`onOpenChange(false)` 报告用户关闭，
Sheet 被其他 Sheet 替换时也会报告关闭。所有者写入 `open={false}` 不会回发事件。
Dialog 还提供 `onConfirm` 和 `onCancel`，`closeOnConfirm` 与 `closeOnCancel`
决定对应操作后是否关闭。`size` 使用 GPUI 逻辑像素，表示 Dialog 宽度或 Sheet 厚度。
title 和 description 使用文本属性；Dialog/Sheet 的 children 与 `footer` slot 接收真实 NSH JSX 子树。

Notification 通过 `open`、`title`、`message`、`variant` 和 `autohide` 控制，
关闭时触发 `onOpenChange(false)`。每个实例使用独立身份；卸载只移除该实例的通知。
内容更新会替换通知并重新计时。当前仅使用应用内通知，尚未暴露系统 delivery；
`content`/`action` slot 与 `actionLabel`/`onAction` 提供真实自定义操作，存在操作时沿用原生不自动隐藏规则。

PopupMenu 使用持久化原生 Entity，支持 `item:<id>` JSX slot、键盘选择、
受控关闭及 ref 的 open/close/focus 命令。AppMenuBar 显式租用应用全局菜单：
最近挂载的 active owner 生效，卸载恢复上一 live owner 或宿主菜单；宿主在其间
显式替换菜单时，卸载不会用旧快照覆盖宿主。菜单动作需要非空且唯一的 id。

Dialog、Sheet 和 Notification 使用 Component 的窗口级 Root，可超出原生节点的布局矩形。
卸载时 Dialog 和 Sheet 通过 scoped handle 关闭自己拥有的实例；不会关闭其他实例或宿主浮层。
浮层的离屏验证按路径记录已通过项；完整 fixture 仍待最后路径确认。

## 验证集成

专用验证入口在 macOS 或 Linux 的 GPUI 离屏窗口中运行真实 NSH 引擎，
需要可用的离屏 GPU 渲染器：

```sh
cargo run --manifest-path crates/nsh/Cargo.toml --features test-support --example verify
```

它检查 signal 往返、Input 同值回写后的光标位置、Checkbox/Switch 共享状态、
Select 重排后的选项身份、Slider change/release 事件、程序设置状态及卸载，
并在开启 reduced motion 的情况下，将完成状态的离屏图片保存至
`target/nsh-verify`。这项针对性集成检查不能证明完整
组件覆盖或所有平台的行为。

独立的 `NSH_VERIFY_FAMILY=datetime` 运行已在 macOS 通过真实日期选择、
disabled 日期、时间段键盘编辑、受控 reset 无反馈、导航 snapshot、range/preset
及 precision/hourCycle 切换验证。真实 checkout 重跑还通过 Calendar、DatePicker
和 TimeField 的 owner veto。Collections 的独立运行也已验证真实 List、DataTable JSX cell、Tree、Command、
grouped Combobox 多选与 Pagination；这些记录不代表其所有 part 或所有平台已通过。

真实 checkout 的 Inputs 运行已通过多行编辑、受控光标/回写、Editor 切换语言后 undo、
Number stepping、OTP completion、富 token 删除、InputGroup mode 切换，以及异步
completion 回应经原生菜单接受。Collections 还通过真实 Tab 进入 Tree，以及关闭
Command 搜索框后 Shift-Tab 返回并继续键盘确认。

Display 的运行已通过真实 Tab/Shift-Tab、跳过禁用项、Rating 受控拒绝/接受、
重排后 group 焦点稳定、Clipboard 与 split Dropdown 交互，以及菜单打开时卸载清理。

Dock 的定向运行已验证真实 tab/splitter 拖动、zoom、布局保存/恢复、独立
ResizeHandle，以及布局恢复后 Input 光标与编辑状态保留。内容族 19 个场景已
验证全部 16 个注册名、原生 Diff 点击、TextView block/inline JSX slot 与
Plot 的原生 bar/pie/scales/grid/axis。Speech 只验证 idle 渲染，没有启动
麦克风或真实识别。迁移到真实 NSH checkout 后，menus、layout、窗口浮层及
AlertDialog 整套 fixture 均已通过，覆盖 scoped 归属和焦点链、Notification
关闭、owner veto/reset、真实嵌套内容、仅 active tab 参与焦点及应用菜单归属恢复。
跨组件族串联运行另行检查相互影响。

`bun script/check-nsh-catalog.ts` 对照 Component 公开的模块边界及 Rust/TSX
注册名称检查清单。清单区分部分绑定、待接入模块和共享基础设施，不会将一个
注册名称视为完整 API 支持。

适配器通过 `gpui_kit_nsh::components()` 创建原生组件注册表，赋给 NSH
的 `AppOptions::components`。它启用 Kit 的 `gpui-fast` feature；
普通 Kit 应用继续使用原有依赖和初始化方式。

Button、Checkbox、Switch、Radio、Toggle 和 Clipboard 的 Rust builder
提供 `track_focus(&FocusHandle)`；DropdownButton 另有
`track_menu_focus(&FocusHandle)`，让 NSH 使用控件真实的焦点句柄。
组合控件按 item value 保存句柄，并按视觉顺序提供已启用的 Tab 停靠点。
普通 Rust 调用方可省略这些方法，继续使用原有持久化句柄。
