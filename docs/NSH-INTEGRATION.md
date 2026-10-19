# NSH 主架构下的 GPUI Component 接入

## 状态与目标

实现进行中，尚未覆盖完整组件库。应用以 NSH 为主：TSX 编写界面，NSH 管理响应式状态、
模块、脚本运行时、热更新和打包；GPUI Component 提供桌面组件及其原有行为。
目标是 NSH 应用能够使用完整 GPUI Component，而非仅在 Rust 宿主中嵌入
NSH 页面，也不是新增一个 gpui-shell 脚本引擎。

既有 Rust 应用及 `gpui_kit::component::*` 的兼容性必须保留。
本工作不修改 gpui-base；适配位于 NSH 与 Component 之上。

## 已检查的接入边界

依据本地 NSH 源码，提交 `7cff5bfae246030af7ca3f5266f500036fedd05c`：

- `crates/gpui/nsh-platform-gpui/src/native.rs` 的 `NativeComponent`
  支持 GPUI 元素渲染、JSON 属性更新、命令、focus handle。
  `ComponentContext` 支持事件、尺寸上报和重绘。
- `Components` 为每个 AppView 提供可共享的注册表；未知名称目前只警告并显示空白。
- native 元素由 NSH 分配矩形，在 surface 的 prepaint 阶段作为 GPUI root
  布局，再在对应树位置绘制。每个实例有独立 element namespace。
- `nsh/native` 的 `nativeComponent` 是现成的脚本入口；
  `nsh_core::ui::Registry` 则是 Rust 响应式 UI 组件的声明与生成接口。
  两者不可未经验证就视作相同的组件协议。
- NSH GPUI workspace 使用 GPUI Fast `0.1.9`；本仓库已有可选
  `gpui-fast` 路径。必须检查 Cargo 最终解析为同一 GPUI 类型来源。
- gpui-shell 已有 Component catalog，但其描述符、状态句柄和回调依赖
  ShellRuntime。不能让 NSH 接入依赖第二套脚本运行时。

以上证明存在原生嵌入边界，不证明完整组件绑定、组合或 overlay 已可用。

## 接入结构

```text
NSH 应用（TSX、signal、模块、热更新）
                 |
        Component 的 NSH 绑定
       /                       \
NSH 属性、事件、ref         GPUI 原生适配器
                               |
                         gpui-component
                               |
                            gpui-base
                               |
                           GPUI Fast
```

新增独立适配 crate，先通过实际编译验证 NSH 发布依赖与 GPUI feature
组合，再确定公开包名及入口。默认 gpui-kit 不引入 NSH 或 QuickJS。
NSH 宿主负责加载注册表和脚本模块、初始化 Component，并通过 Kit 的
window helper 安装窗口 Root。普通 NSH run/package 路径也必须能装载
该集成；仅自定义 Rust 宿主可运行不构成完成。

适配 crate 使用独立 workspace 和 lockfile，根 workspace 明确排除
`crates/nsh`。这是为了防止 NSH 必需的 Fast feature 在 `--workspace`
构建时传播到原本使用 GPUI Pre 的 crate；通过 `--manifest-path`
显式构建适配器，原 workspace 的默认构建保持自己的依赖图。

### 当前实现与验证

`crates/nsh` 提供 `gpui-kit-nsh`，直接使用真实 NSH 源码仓库。
根据用户要求，不在本仓库 vendor NSH，也不把 NSH 修复维护为本仓库快照。
NSH 引擎、组合桥接与运行器所需修改在 NSH 仓库维护；本仓库保留 Component 适配器。
按后续要求，Component 与独立的 `crates/nsh-base` 同时保留，不以 Base 替换或删除
Component。Base 从 Button、Checkbox、Input 的真实原生行为开始，NSH 负责视觉样式，
不启用 Component 或 Kit assets，也不宣称已经暴露所有 Base API。两套任务示例分别位于
`crates/nsh/examples/compare` 与 `crates/nsh-base/examples/app`。
各组件族均已有部分绑定；以下验证结论只覆盖明确列出的路径。
全部 152 个绑定入口已统一到 NSH 根据 Rust Props/Event 生成的 `nsh/ui/gpui`；示例中事件
更新 NSH signal，由 signal 回写 controlled props。
InputState 和 Subscription 跟随原生实例保存，同值 value 不调用 set_value。
Button、Checkbox、Switch 新增可选 `track_focus(&FocusHandle)`，用于
向 NSH 暴露真实控件的焦点；默认调用保持原有 keyed focus handle。
Select 按稳定 value 同步选项、选择值与搜索设置，保留 SelectState；新增
`SelectState::set_searchable(&mut self, bool, &mut Context<Self>)` 支持原地切换搜索。
Slider 同时支持单值与区间，将 change/release 分开发送；配置同步保留原有
Entity、订阅和手势字段，输入配置会先校验，避免无效的浮点范围触发 panic。

`crates/nsh/catalog.json` 记录 Component 的公开模块及本地 re-export 模块边界。
`bun script/check-nsh-catalog.ts` 递归检查适配器源目录中的模块遗漏及 Rust/TSX 注册名一致性。
使用 `--report target/nsh-api-audit.json` 可生成逐 binding 的源文件、声明的 props/events
以及 Component 公开 Rust 声明的待审证据。该报告不把源文件存在视为已接入公开入口，
也不把 props/events 声明视为已验证；commands、slot、状态、feature、平台仍需逐项审查。
并行接入期间模块数量持续变化，以脚本输出和 catalog 文件为准；当前没有完整
接入模块。`partial` 表示仍未证明完整 Rust API/平台行为等价；目前全部注册入口已通过
编译和生成类型检查，实际交互结果单独记录在 `docs/NSH-COMPONENT-REGRESSION.md`。
该清单尚不等于逐个组件、状态、事件、方法与 slot 的完整 API 对照表。

已通过 macOS `cargo check --manifest-path crates/nsh/Cargo.toml --example nsh`、
`cargo build --manifest-path crates/nsh/Cargo.toml --example nsh`、该示例的 Clippy（deny warnings），
以及 `nsh check crates/nsh/examples/app` 和 `nsh build`。生成的宿主加载
bundle 后持续运行，未出现启动崩溃或 NSH 致命错误；此检查不证明实际交互正确。
`cargo check -p gpui-component --no-default-features` 和 Kit 的 Fast UI
测试目标编译也通过；新增宿主焦点句柄的键盘激活回归用例，尚未运行测试。
Cargo tree 确认 NSH 与 Kit 的 Fast 路径统一为 `gpui-fast 0.1.9`，
默认 Kit 没有 NSH/QuickJS 运行时依赖，适配器没有 gpui-shell 依赖。
NSH 依赖直接指向同级 `../nsh` 源码 checkout（从 crates/nsh manifest 为
`../../../nsh/crates/gpui/nsh-platform-gpui`）。本地必须同时具备同级 `gpui-kit/` 与 `nsh/`；必需的 NSH 修复尚无
可引用的上游 commit，单独克隆本仓库不足以构建适配器。真实 checkout 的 default/dev offline metadata 已确认所有 nsh-* 依赖均来自同一真实
源码仓库；无 vendor、patch 或绝对路径。迁移后的真实 NSH binary 已通过 display、datetime、Dock、menus、layout、窗口 overlays
与 AlertDialog 整套定向验证；此前其他族的证据按其具体构建记录保留，跨族串联另行验证。
GPUI Pre 仍可作为既有非 optional 依赖被额外编译；真正参与此集成的 GPUI 类型
统一走 Fast 0.1.9，不能据此声称整张依赖图只有一个 GPUI 包。
新增 `cargo run --manifest-path crates/nsh/Cargo.toml --features test-support --example verify`，
在 macOS 真实 NSH 引擎及 GPUI 离屏窗口中，已验证事件到 signal 的往返、
Input 同值回写后的光标位置、Checkbox/Switch 共享状态、Select 重排后的
选择身份、Slider change/release、程序设置状态不重复发用户事件，以及卸载。
已查看 `target/nsh-verify/select-open.png`，确认菜单跨出 Native 触发器边界
并正确盖住后面的控件；验证入口还断言菜单中的搜索输入在该边界外可见。
图像验证开启 reduced motion，检查的是完成状态，不覆盖动画过程。
验证入口要求 macOS 或 Linux 的离屏渲染器，本次仅在 macOS 运行。
该验证还检查关闭 Select 搜索后保留选中值并移除搜索输入。
适配器隔离后，已重新通过独立 manifest 的 host 构建、验证器运行和示例 Clippy；
父 workspace 的 lockfile 保持不变，默认 Kit 依赖图不包含 NSH 或 Fast。
NSH 编译/Clippy 与 macOS 离屏 CI 需要真实 NSH 修复 commit。当前修改仅在同级源码
checkout，不能让远端 CI 引用不存在的修复。该构建任务应暂不启用；inventory 检查继续运行。
修复可从远端获取后，CI 应将 gpui-kit 与 NSH checkout 到工作目录中的同级子目录，
以固定 NSH commit 恢复三平台编译与 macOS 离屏任务，不使用 vendor 替代。
Dialog、Sheet、Notification 已有窗口级原生适配，分别进入 Component 的 Root
overlay 和 notification host。Dialog/Sheet 已接入真实 NSH children 与 footer slot；
title/description/message 仍是文本属性，Notification 支持 content/action slot 与 actionLabel/onAction。Dialog/Sheet 的 scoped handle 只关闭自己拥有的实例，避免卸载
时关闭宿主或其他 NSH 实例的浮层；Notification 使用独立 key，并固定为 in-app delivery。
受控关闭不回发用户事件。Dialog/Sheet 的嵌套、替换、scoped unmount 与焦点链已通过
定向运行；Notification 的独立身份、卸载与 hover Close 均已通过；迁移后完整 overlay fixture exit0。
当前各组件族都已有适配源码，catalog 的 `pending` 已归零；所有模块仍为 `partial`。
已实现独立的原生 subtree 桥接、宿主主题同步及标准 NSH CLI 运行器，正进行集成验证。
尚未完成逐组件完整 API 对照、所有 part 的组合覆盖、手动桌面交互及跨平台验证。
本轮 NSH check 与构建示例已再次通过；macOS 的 datetime、Dock 和内容族定向运行已通过，
Collections 的七个主组件交互亦已有通过记录。新增 parts、键盘批次及其余族仍按实际运行逐项确认。
标准 NSH_RUNTIME 运行器目前适用于普通 TSX 应用；nsh.json 中的自定义 rust crate
会令现有 NSH CLI 优先使用生成的 Rust runtime，绕开 NSH_RUNTIME，此路径尚未接入
Kit Root/catalog。自定义宿主可显式调用 host::run，不能据此宣称原生 CLI 生成路径已支持。

## 低维护绑定架构迁移

组件绑定已统一为 Rust-owned native specs，复用真实 NSH 的 Registry、
Props/Event 元数据与模块/类型生成链。152 个 binding 的源码清单已统一走
NSH 的共享 `NativeRegistry`（适配器内部别名为 `BindingRegistry`），JavaScript
组件入口由 NSH 生成；`js/` 仅保留兼容 index 与
通用重导出 bridge，不再有各组件族的手写 TS schema 或属性数组。
普通 parts 共用小型 native_leaf macro，命名类型共用 named_value macro；
复杂组件保留实际 Entity、delegate、订阅及原生生命周期。
通用接受前域校验会丢弃无效候选批次，保留上一份可用 props，防止无关后续更新被污染。
类型解码错误也会拒绝整个批次；原生控件已编辑后遭拒时，由共享桥接重新应用上一份
accepted props，而非要求各控件实现另一套回滚协议。错误仅在批次提交时报告一次。
两种适配器的资源定位、package 加载和严格开发参数解析统一委托真实 NSH 的
`runner` 模块，Kit 层仅保留自己的初始化、Root 开窗和 Component Theme 同步。

该结构减少双份声明与字符串属性分派，不保证 Rust 总行数下降。日期等复杂组件
因准确的 Rust 值类型与事件而增加部分声明，不能夸大为零适配或自动支持任意 builder。
当前 source inventory、全部注册入口编译及生成类型契约已通过；真实交互按
`NSH-COMPONENT-REGRESSION.md` 和 `NSH-BASE-REGRESSION.md` 分别记录。
原有七组件及 CLI、HMR、macOS dev 打包基线已有通过记录，不代替本轮全族检查。
目录清单保持 partial，完整 Rust API 等价与跨平台边界不因生成声明而改变。

## 状态、组合与生命周期

业务值由 NSH signal 持有。Input、Select、Table、Tree、Dock 等所需的
GPUI Entity、delegate、订阅与 focus handle 由原生实例保留。
属性变更更新既有实例；不得在每次 render 时创建新状态。
同值回写不得重复发出用户 change 事件。删除实例、卸载模块及热更新时，
必须释放状态、订阅、回调及未完成任务，不再向已删除节点发事件。

叶子组件使用 nativeComponent；compound component 通过 NSH 源码仓库提供的
ComponentContext::children()/slot(name) 获得原有 NSH 节点的 NativeSubtree，
`js/bridge.ts` 仅重导出 NSH 的 `nsh/native`；children、slot、signal owner、动态子树与清理统一由 NSH 的 Rust renderer 维护。
GPUI 子元素参与真实 GPUI 布局；普通 NSH 子树保留 NSH 布局与渲染。每个挂载节点
每帧只允许一个视觉位置，render callback 每帧解析 slot，不能保存 AnyElement 跨帧复用。
Menu、Popover、Tabs、Table cells、Tree rows、Dock panels 等已使用此边界，
真实运行验证仍须覆盖指针、焦点、键盘/IME、卸载与替换。窗口级 Dialog、Sheet、
Notification 使用 Component 的 Root host，可超出 native 节点的裁剪矩形。

已进一步确认 `nsh-js/src/platform_modules.rs::render_native` 将
`children` 放入 skip 列表，随后只绑定声明的 JSON props 和事件；
`NativeComponent::render` 没有提供 NSH 子树句柄或 slot 参数。
未经扩展时，直接给 nativeComponent 添加 children prop 不会形成有效组合。
当前桥接不通过 JSON prop 传 children，而是保留 NSH 挂载树及原生 subtree handle，
因此无需第二个脚本运行时。

## 实施顺序与验收

1. 建立独立适配 crate 和最小 NSH 宿主，实际编译同一 GPUI Fast 类型图。
   示例必须由 NSH TSX 调用真实 Component Button，事件更新 NSH signal。
2. 接通 Input 持久状态、属性更新、事件、ref command、Tab 焦点、卸载。
   验证编辑时光标、选区与 IME 不因无关 signal 更新而重置。
3. 验证 NSH children/slot 与 GPUI 组合边界、窗口 overlay、主题及尺寸协商。
   以 Select、Menu、Dialog 和带自定义 cell 的 Table 作为跨边界验证。
4. 从 `crates/component/src/lib.rs` 的所有公开组件模块及 re-export
   建立完整 inventory，逐项映射组件、状态、事件、方法、slot、delegate、
   feature 与运行示例。基础设施模块单独说明用途，不能冒充组件覆盖。
5. 依 inventory 完成所有组件族，包括文本/编辑器、数据/虚拟化、Dock、
   图表、日期时间、消息、原生集成及可选功能。平台能力与 Cargo feature
   限制保留，并在生成声明和加载错误中明确；不静默退化为空白组件。
6. 把模块声明生成和绑定注册接入 NSH 的 type-check、run、热更新和 package。
   同步提供英文及中文使用文档。

完成证据必须包括完整 inventory 对照、生成的 TypeScript 声明可检查、
可运行的 NSH gallery、实际窗口中的事件/焦点/overlay/生命周期验证，
以及 macOS、Windows、Linux 的构建与能力核验。叶子示例或注册名称数量
不能单独证明“支持所有组件”。现有 Rust 消费者的构建路径也必须保持。

公共 Render 候选、父组件/slot 对应关系及不可独立构造的例外见
[组件覆盖逐项记录](NSH-COMPONENT-COVERAGE.md)。

## API 覆盖审查记录

以下记录源码覆盖审查及明确标注的定向运行证据；运行通过不等于完整 API 等价。
所有 Component 模块仍为 `partial`；它表示可用绑定尚未完成所有 Rust builder、
callback、状态与组合路径的逐项对等审核，不表示整个模块不可用。
精确的 props/events 以 Rust Props/Event 及 NSH 生成的 `nsh/ui/gpui` 声明为准，源报告可用
`bun script/check-nsh-catalog.ts --report target/nsh-api-audit.json` 重建。

| Family | Retained state / composition | 已知限制与待验收项 |
| --- | --- | --- |
| Display | 稳定 focus handles、原生控件与 live children；Rating 显式受控路径 | 已通过真实 NSH Tab/Shift-Tab、禁用跳过、group 重排、Rating 拒绝/接受、Clipboard、split Dropdown 与打开菜单卸载；仍未证明任意 Rust 样式 API 或跨平台等价 |
| Input / Editor | InputState、TextareaState、EditorState、NumberInputState、OTP entity 与订阅；prefix/suffix 和 token:<id> 真实 slot；异步 language service request/resolveRequest 通道 | InputToken 依赖 InlineTokenContext，通过 Input content/tokens 与原生 renderer 表达；现有 Base 的 resolve_completions 使用 request::Completion 空类型且无调用点，保留原状，不制造无效 JS 接口；showDocument 由 onShowDocument 交还 NSH；InputGroup 通过 mode=input/textarea 调用真实单行/多行主输入，切换 mode 明确重建 session，同 mode 保留编辑状态；真实 checkout 已通过 Textarea、Editor controlled caret/echo/language undo、Number、OTP、token 删除、InputGroup mode 切换及异步 completion 原生菜单接受；IME 和全部 provider 方法仍需专门验证 |
| Collections | List/Combobox/Command/Table/Tree 的 retained Entity、delegate、稳定 ID；原生 Table/List parts；item、cell、header、empty 等真实 slot | SearchableList 由 Combobox/Select delegate 与 SearchableListItemElement 表达；grouped Combobox 已接入原生 SearchableGroup；List 选择/禁用/重排/搜索、DataTable 选择/排序/真实 JSX cell/export、Tree 展开键盘、Command 搜索确认、多选及 Pagination 已通过定向运行；最新运行另通过 List/Tree Enter、compound Table cell 与独立 ListItem 点击；Footer/Caption/Separator/Loading 等其余 parts 尚未逐项运行 |
| Date and time | CalendarState、DatePickerState、TimeFieldState；原地 setters 保留浏览位置和焦点；Calendar 导航/snapshot ref commands | 使用本地 civil 日期时间，不隐式转换时区；range DatePicker 沿用原生日期编辑范围；不提供原生不存在的 multiple 模式；macOS 真实 datetime fixture 已通过选日/disabled、hour/minute/AM-PM 键盘编辑、受控 reset 无反馈、导航 snapshot 不改值、range/preset 和 precision/hourCycle 切换；迁移后另通过 Calendar、DatePicker、TimeField owner veto；日期解析单测通过。任意 Styled/Matcher::custom Rust 闭包不直接序列化 |
| Dock / resize | DockArea、Panel Entity、PanelRegistry、ResizableState 与订阅；panel content/title/titleSuffix/toolbar action slots；move/dockSize/save/restore | 只恢复当前已挂载 NSH Panel；menu 的任意 JSX/submenu 尚未映射；Dock/Resizable/ResizeHandle 真拖动、zoom、save-close-restore 及 Input XAda→XYAda 的编辑状态保留已实测；多个窗口验证仍待完成 |
| Layout / messaging | NSH children/slots 保持原有 owner；Carousel/Questionnaire/MessageScroller 的原生状态 | 迁移后完整 layout fixture 已通过：live children、Tabs 键盘/仅 active input 参与 Tab、Accordion Space、Carousel、Questionnaire 提交、MessageScroller 与 Sidebar 键盘；领域扩展 callback 与全部细分 part 等价仍需复核 |
| Text / Diff | retained TextViewState、DiffState 与订阅；文本追加和选择、diff 导航 commands | TextView blockSlots/inlineSlots 由原生 MarkdownPlugin 将 exact source/offset 映射到真实 NSH 子树；不开放任意 JS AST parser callback。内容族最新 19 个场景已运行通过，包含全部 16 个注册名、原生 Diff 点击、block/inline live slots 与 PlotShapes；Speech 仅验证 idle 渲染，未启动麦克风或真实识别 |
| Chart / Plot | 原生 Chart 与 tooltip/animation；Plot 使用原生 shapes | Plot 已扩展原生 Linear/Point/Band/Ordinal scale、PlotAxis/AxisText、Grid 及 Line/Area/Bar/Pie/Arc primitives；任意 Rust geometry callback 不直接序列化；原生 bar/pie/scales/grid/axis 已通过 PlotShapes 实测，不以 Chart 绑定冒充 Plot 覆盖 |
| Speech | 同窗口共享原生 SpeechState；真实 started/partial/final/cancelled/error；start/stop/cancel/toggle | 取决于系统后端、feature 和权限；不将没有音频时的 Waveform 空白视为接口失败；尚无真实系统识别与多平台验证 |
| Menus | retained PopupMenu、live item slots、AppMenuBar application-global lease、enabled focus handles | 迁移后菜单完整 fixture 已通过 action、disabled 键盘跳过、live slot、rebuild、焦点恢复、owner/host takeover；系统菜单的跨平台原生 UI 仍需人工验证 |
| Window overlays | Root window host、scoped DialogHandle/SheetHandle、每实例 Notification key；Dialog/Sheet live children/footer | Notification 支持 content/action slot、actionLabel/onAction，action 按原生规则禁用 autohide；系统 delivery 未暴露；Dialog/Sheet 嵌套、替换、scoped unmount 和焦点链已运行通过；迁移后完整 overlays/AlertDialog 运行通过，含 Notification hover Close、owner veto/reset |

直接按名称扫描公开 RenderOnce/Render 类型会包括不能独立使用的 state、私有模块的
pub 类型和只在父组件上下文中有意义的 parts。报告将这些标为待审候选；必须记录其
真实父组件/slot 映射，不能机械地创建同名空壳或将没有独立注册视作完整审查结果。
`--require-complete` 会拒绝当前所有 partial 模块；标为 complete 还必须提供
`coverage.publicApiReview` 和 `coverage.verification`。源码存在、类型声明可检查、
实际窗口验证和跨平台构建分别记录，不能互相替代。

## 当前待解决的问题

继续完成逐项 API/part 对照和真实窗口验证，包括所有受控状态、异步 provider、
slot 动态更新、嵌套浮层、焦点恢复及卸载。标准 CLI 的 macOS dev profile 已实证开发模块加载、源修改后的 HMR 模块执行、
.app 生成及从 /tmp 独立启动包内 runner 读取自身资源。日志位于
`target/nsh-runtime-workflow/verification.log`；不据此推断 signal 保留或全部 GUI 交互。
release 优化及 Windows/Linux 运行验证仍未执行。始终保留 NSH 单运行时；不得引入 gpui-shell 规避边界。

迁移后菜单运行：真实 Tab/disabled 跳过、AppMenuBar action、多 owner 接管与恢复、
外部宿主接管、PopupMenu live slot/键盘/items 更新、焦点恢复与受控关闭均已通过。
迁移后布局运行：live children、Tabs 键盘/仅 active input 参与 Tab、Accordion Space、
Carousel、Questionnaire 提交、MessageScroller 与 Sidebar 键盘均通过。
