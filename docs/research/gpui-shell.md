# GPUI Shell RFC

## 基于 GPUI Base 的 JavaScript 原生应用运行时

| 项 | 值 |
| --- | --- |
| 状态 | Draft（提案；M0 骨架已落地于 `crates/shell`） |
| 版本 | 0.4 |
| 文档类型 | 架构提案（`docs/` 内部规范，不发布到站点） |
| 脚本引擎 | **默认 QuickJS**（`quickjs` feature，`rquickjs` 0.12）；`lua` / `luajit` feature 保留一份可用的 mlua 引擎作为退路（§6.3、§6.5） |
| 绑定目标 | **第一期：`gpui-base` + `gpui`**；后续：`import ... from "gpui-component"` |
| 依赖基线 | `gpui-base` 0.5.2 / `gpui` 0.2.2 (zed git) |
| 前置阅读 | [ARCHITECTURE.md](../ARCHITECTURE.md)、[STYLING-AND-MOTION.md](../STYLING-AND-MOTION.md) |

---

## 0. 修订记录

### 0.1 v0.3 → v0.4：脚本语言换成 JavaScript，并留下引擎接缝

v0.3 把 LuaJIT / Lua 5.4 定为唯一的脚本语言。该决定**被修改**，结论是：

> **默认脚本引擎改为 QuickJS（JavaScript）；同时在 `crates/shell/src/engine/` 划出一条引擎接缝，把 Lua 引擎作为可编译、可切换的退路保留下来。**

换语言的理由只有一条，而且它是产品理由不是技术理由：**应用代码用 JavaScript 语法写出来更好读**。第一期呈现权在脚本侧（§4.2），应用层的绝大部分代码是"组合元素 + 写样式 + 处理事件"，这类代码的可读性直接决定了这套运行时值不值得用。

必须同时说清楚这次更换的代价，因为它不是全面升级：

- **变强的是 §2.1 的"生成成本"与使用者 C（AI / 生成式界面）。** JavaScript 是模型训练语料里覆盖最好的语言，没有之一；生成 JS 界面代码的一次通过率高于 Lua，这一条对 §2.2 的使用者 C 是决定性的。
- **变弱的是"小而快的嵌入式 VM"这一条。** QuickJS 比 LuaJIT 大，且**没有 JIT**：它是字节码解释器。热点循环与每次跨语言调用的成本都不会比 LuaJIT 更好（§6.3）。而 §20 的成本模型说明，`C_op` 正是这个方案的一票否决项。
- **因此接缝不是可选项。** 引擎选择是本运行时唯一无法在纸面上判定的决定，M0 的基准测试必须在**两个引擎下各跑一遍**（§26）；若 QuickJS 过不了门槛，切换的动作是改一个 cargo feature，而不是重写（§6.5、R12）。

v0.3 的全部协议设计原样保留，因为它们**与引擎无关**并且已经按这个前提实现：渲染协议（§8）、CallScope（§9）、base-first 的分层立场（§4）、dock 与 panel 集成（§15）、性能模型（§20）在 `crates/shell` 里都位于 `engine/` 之上，两个引擎共用同一份代码。这不是事后辩解：`spec.rs`、`scope.rs`、`style.rs`、`theme.rs`、`capability.rs`、`materialize.rs`、`value.rs`、`error.rs` 里没有一处出现 VM 名字。

### 0.2 v0.2 → v0.3：绑定目标定为 gpui-base

v0.2 曾提出"改绑 `gpui-component`"，理由是 base 无样式、脚本直接用会渲染出看不见的控件。该提议**被否决**，最终决定为：

> **第一期只基于 `gpui-base` 与 `gpui` 本身提供能力；`gpui-component` 后续以 `import ... from "gpui-component"` 的形式追加。**

这不是把问题绕开，而是把"呈现层归谁"这个问题重新定位：在 base-first 的方案里，**呈现层就是脚本应用层本身**，这正是 ARCHITECTURE.md 中 base 的第二类调用者：

> applications that build and own a different visual system directly on top of base behavior.

因此 v0.3 保留 v0.2 的全部协议设计（对象模型、渲染协议、CallScope、异步、沙箱），把分层与呈现归属改回 base，并补齐 base-only 路线必须自己承担的四件事（§4.3）。完整论证见 §4.2。

### 0.3 v0.1 → v0.2/v0.3 仍然成立的四处修正

| # | v0.1 的表述 | 问题 | 结论 |
| --- | --- | --- | --- |
| 1 | `Coroutine → GPUI Scheduler → Tokio Runtime` | 本 workspace **不依赖 tokio** | 异步走 GPUI 的 foreground/background executor（smol），见 §12 |
| 2 | `local button = gpui.Button.new("hello")` 作为长期持有的对象 | GPUI 的元素是**每次渲染重建的值**：`Button` 是 `#[derive(IntoElement)] + RenderOnce`，`render(self, ...)` 消费 `self` | 三类对象模型 + ElementSpec 渲染协议，见 §7、§8 |
| 3 | Rust Extension 类比 "Python → C Extension"，暗示动态加载 | Rust 无稳定 ABI；`dlopen` 的原生代码直接击穿沙箱 | 原生扩展改为**宿主编译期注册**，见 §17.6 |
| 4 | 单一 VM 写死在实现里 | 仓库有 wasm 目标（`crates/base/examples/wasm`、`crates/story-web`），LuaJIT 编不到 wasm；Apple Silicon 还有 W^X 约束 | 引擎是一条显式接缝 + cargo feature，见 §6.3、§6.5 |

---

## 1. 摘要

GPUI Shell 是构建在 `gpui-base` 之上的**动态应用运行时**：Rust 提供渲染、布局、文本编辑、虚拟化、dock、焦点与系统能力，脚本编写**界面组合、视觉呈现与业务逻辑**。脚本语言默认是 **JavaScript（QuickJS）**。

它提供四样东西：

1. 一个可嵌入的脚本应用运行时（VM、调度器、错误恢复、热重载），VM 藏在一条显式接缝之后（§6.5）；
2. 一套覆盖 `gpui-base` 行为层与 `gpui` 元素/样式层的 Native UI 绑定；
3. 一套受能力约束的系统 API（fs / http / store / clipboard / log）；
4. 一个插件与扩展模型（manifest、contribution points、沙箱、分发）。

与 v0.2 提案的关键差别：**脚本不只是"调用别人做好的组件"，脚本就是应用层，拥有完整的呈现权**。`gpui-base` 交出行为，脚本交出样式，二者合起来才是一个应用——这与 Rust 应用在 base 之上自建视觉系统的方式完全同构（`crates/base/examples/showcase` 就是这样一个 Rust 版先例）。

选 JavaScript 的理由同样只有一条：**应用层代码读起来更好**。类、箭头函数、模板字符串、解构、`import` / `export default` 这些结构，恰好落在"组合界面 + 写样式 + 处理事件"这类代码上，而这正是脚本侧代码的绝大部分。附带收益是 §2.1 的第三条成本（生成成本）：JS 是模型语料覆盖最好的语言。附带代价写在 §6.3：QuickJS 没有 JIT，体积也比 LuaJIT 大，`C_op`（§20.2）因此更紧张。

一句话目标：

> 用脚本语言的迭代速度构建原生应用的应用层，同时保持 Rust 与 GPU 渲染的性能。

---

## 2. 动机

### 2.1 本仓库当下的三个具体成本

**编译成本。** 应用层的每一次 UI 微调都要经过 `cargo build`。本 workspace 依赖 zed git 版 `gpui`、tree-sitter、syntect、reqwest，冷构建以分钟计；即使增量构建，改一行也要等待链接。对"调一个间距 / 换一个颜色 / 加一个筛选条件"这类改动，编译时间远大于改动本身的思考时间。

**扩展成本。** `crates/base/src/dock` 已经具备了插件系统需要的一半基础设施：`PanelRegistry` 能按 `panel_name` 字符串重建 panel，`PanelInfo::panel(serde_json::Value)` 能让 panel 持久化私有状态，未注册的 panel 还会被 `DockArea` 用占位视图承接、保证布局往返不丢。缺的另一半是：**panel 的实现必须编进宿主二进制**。第三方无法在不 fork 宿主的前提下贡献一个面板。

**生成成本。** AI 生成 Rust UI 要求类型正确、借用正确、编译通过，反馈回路是编译器；生成脚本界面可以立即执行、立即看到画面、出错时抛出可恢复的异常而保留宿主进程。

**这一条在 v0.4 之后显著变强。** JavaScript 是公开语料里覆盖最好的语言，模型对它的类、模块、闭包、数组方法的掌握程度高于 Lua 一个层级；同时 §14.4 的类型声明从 LuaCATS 换成 `.d.ts` 之后，"喂给模型的 API 契约"这件事有了业界标准格式，也有了 `tsserver` 这一现成的校验器。使用者 C（§2.2）因此从"能用"变成"最合适"。

**必须同时承认它的反面。** JS 的语料优势也意味着模型会带来大量**不适用**的假设：`document`、`window`、`fetch`、`require("fs")`、npm 包、`setTimeout`。这些在本运行时里一个都不存在。缓解手段是 §14.4 的 `.d.ts` 与 §19 的诊断信息——未定义的全局必须报出"这里没有它、请用什么替代"，而不是一句 `ReferenceError`。

### 2.2 三类目标使用者

| 使用者 | 场景 | 对 Shell 的要求 |
| --- | --- | --- |
| A. 宿主应用的插件作者 | 给一个已有的 Rust 应用加面板、命令、侧栏工具 | 稳定的贡献点、沙箱、与 dock 布局持久化打通 |
| B. 内部工具作者 | 仪表盘、运维面板、数据查看器、一次性工具 | 起步成本低、系统 API 齐、能打包分发 |
| C. AI / 生成式界面 | 由模型生成界面与交互 | 语法常见、错误可恢复、能热重载、API 可被类型化描述 |

**这三类都不是"用脚本重写核心产品"。** 这一区分决定了后面几乎所有取舍。

### 2.3 参考项目及其真正可借鉴的部分

| 项目 | 可借鉴 | 不可照搬 |
| --- | --- | --- |
| Neovim | 宿主提供能力 / 脚本负责扩展；`vim.api` 的稳定契约；插件懒加载 | 语言不同（Lua）；Neovim 的 UI 是终端网格，没有元素树与布局引擎 |
| VS Code Extension | **同语言先例**：JS 扩展 + 单一宿主命名空间 + 能力声明、贡献点、分发与信任模型 | 其扩展跑在独立进程、有完整 Node API；本方案是同进程、无 Node、无 npm |
| Figma Plugin | **QuickJS 作为受限 UI 插件 VM 的现网先例**：无 DOM、宿主 API 白名单、单帧内可中断 | 其插件不拥有渲染，只操作文档模型；本方案脚本直接产出元素树 |
| Path of Building | 脚本可承担完整应用体量的业务逻辑**与呈现** | 它自绘 UI（且是 Lua），没有组件行为层可复用 |
| Qt Quick | Runtime + Script + Native Engine 的三层分工 | QML 是声明式 DSL；本方案**不做 DSL**，也不做 JSX（§5.3） |

换语言之后，这张表的重心从 Neovim 移到了 VS Code 与 Figma：前者证明"JS 写插件 + 宿主给能力"这个模型能撑住十万级插件生态，后者证明"QuickJS 当受限 UI 插件 VM"在生产环境跑得住。两者都不能证明的恰恰是本方案最吃紧的一处：**它们的脚本都不在每帧重建元素树的路径上**，而本方案在（§20）。

---

## 3. 范围与非目标

### 3.1 第一期范围

- 绑定 `gpui` 的元素与样式层（`div`、`img`、`svg`、`canvas`、`Styled`、交互修饰符）。
- 绑定 `gpui-base` 的行为层（Button、Checkbox、Input/Textarea、Select、Tree、Table、Dialog、Sheet、Popover、Tabs、Scrollbar、VirtualList、Dock 等）。
- 脚本拥有完整呈现权：样式、颜色、间距、状态样式全部由脚本表达。
- Shell 自带一套**可替换的**默认 token 调色板与脚本预设模块（§4.3）。
- 受能力约束的 fs / http / store / clipboard / log。
- 插件模型：manifest、贡献点、沙箱、dock 面板集成。
- 引擎接缝：QuickJS 为默认，Lua 引擎保持可编译（§6.5）。

### 3.2 后续范围

- `import ... from "gpui-component"`：把 `gpui-component` 的成品视觉组件作为**第二个绑定注册表**接入，与第一期共用同一套渲染协议（§14.6）。

### 3.3 非目标（明确不做）

1. **不替代 Rust 编写产品核心。** 文本编辑引擎、语法高亮、LSP、虚拟化、动画留在 Rust。
2. **不引入 UI DSL / markup，也不引入 JSX。** 界面由普通函数与 builder 调用产生（§5.3）。JSX 需要一个编译步骤，而"改一行、存盘、立刻看到"是本方案存在的理由。
3. **脚本不进入 layout / paint 热路径。** 布局、绘制、命中测试、滚动、IME 全在 Rust（§8.5）。
4. **不做多线程脚本。** VM 与 GPUI 的 `App` 同为主线程独占（§12.4）。不提供 `Worker`。
5. **不做任意 Rust dylib 插件加载。** 原生扩展由宿主编译期注册（§17.6）。
6. **不修改 `gpui-base`。** 全部实现位于 `crates/shell`；若实现中发现 base 缺口，走单独提案，不夹带（§4.4）。
7. **不做 Node.js / 浏览器兼容层。** 没有 `document`、`window`、`fetch`、`require`、`process`、`node:fs`，也不接 npm。做半套兼容层的结果是把整个 npm 生态吸引进来，然后在第一个原生依赖上碎掉；宿主能力一律走 `gpui.*` 这一个命名空间（§6.4、§17）。这条是 v0.4 新增的，因为换成 JS 之后它才成为一个真实的诱惑。

---

## 4. 与既有架构的关系

### 4.1 分层

```text
     JS 应用 / 插件              main.js · panels · commands · 样式与主题
              │  import ... from "gpui"             ← 第一期
              │  import ... from "gpui-component"   ← 后续
              ▼
     crates/shell ── gpui-shell
     ┌──────────────────────────────────────────────┐
     │ engine/ 接缝：QuickJS（默认）| Lua（退路）     │
     ├──────────────────────────────────────────────┤
     │ CallScope · ElementSpec Arena · 样式反射表    │
     │ BindingRegistry · Scheduler · Sandbox        │
     │ ShellRoot · DockSkin 转发 · 默认 token 调色板 │
     │ PluginManager · HostApi (fs/http/store/...)  │
     └──────────────────────────────────────────────┘
              │
              ▼
     gpui-base              行为 · 状态 · 基础设施（无样式）
              │
              ▼
     gpui / gpui_platform   元素 · 样式 · 渲染 · GPU · 平台
```

对照 ARCHITECTURE.md 的依赖图，`crates/shell` + 脚本应用占据的正是 **application-owned UI** 那一支，与 `gpui-component` 平行而非其下游。这是本方案在架构上的定位。注意接缝在图里只有一层薄边：接缝之下是"什么是脚本值"，接缝之上的每一格都与语言无关（§6.5）。

### 4.2 为什么第一期只绑 `gpui-base` 与 `gpui`

五条理由，按重要性排列：

**1. 呈现权完整地交给脚本，这才是"用脚本写应用层"。**
如果第一期就绑 `gpui-component`，脚本能做的只是调用别人已经定好的视觉；改一个按钮的圆角仍要回到 Rust。绑 base 之后，脚本掌握样式、状态样式、间距、颜色的全部决定权——应用层真正落在了脚本里。

**2. 分层中立。** shell 不依赖任何一套产品视觉系统，因此任何宿主都能嵌入它，包括自研设计体系的宿主。反过来，一旦 shell 依赖 `gpui-component`，它就把一套具体视觉强加给所有嵌入者。

**3. 绑定面小一个量级，第一期才可能"绑全"。**

| | `gpui-base` | `gpui-component` |
| --- | --- | --- |
| Button 模块 `pub fn` | 13 | 52 |
| 直接依赖数 | 18 | 31（多出 markdown、html5ever、rust-i18n、assets、rust_decimal…） |

base 的接口因为"不承载视觉"而天然更窄、更稳定。第一期绑 base 可以做到覆盖完整；绑 ui 只能覆盖一部分，而"覆盖了一部分"的绑定层是最难用的形态。

**4. 构建与体积。** 运行时自身的迭代速度、二进制体积、wasm 可行性都直接受益于更小的依赖树。换成 QuickJS 之后这一条更要紧：VM 本身已经比 LuaJIT 大，依赖树上省下来的每一点都用得上（§6.3）。

**5. 已有可行性先例。** `crates/base/examples/showcase` 就是一个 base-only 应用：自己实现 `DockAreaRenderer` / `TabGroupRenderer` / `TilesRenderer`，自己提供 `InputEditorStyle` 与颜色，自己接 syntect 高亮，并且能跑 wasm。第一期的 gpui-shell 本质上是**把 showcase 的组合与样式部分改写成脚本**。

### 4.3 base-only 必须自己承担的四件事

这条路线的代价是真实的，必须写在设计里而不是留给实现者去撞。

**（1）默认 token 是透明的，shell 必须自带调色板。**
`gpui_base::Theme::default()` 里的 `ColorTokens` 是 `#[derive(Default)]`，即全部 `Hsla { h:0, s:0, l:0, a:0 }`——**透明**。`RadiusTokens` / `SpacingTokens` 有真实默认值，颜色没有。`gpui_base::init(cx)` 之后如果不填色，界面就是一片透明。

因此 `crates/shell` 必须提供一份默认语义 token 调色板。好消息是零 schema 成本：`SemanticThemeTokens` 及其子结构已经 `#[derive(Serialize, Deserialize, JsonSchema)]`，直接反序列化一份 `default-tokens.json` 写入 `Theme::global_mut(cx).tokens` 即可，插件主题也走同一份 schema。已实现于 `crates/shell/src/theme.rs`，且位于接缝之上——它与脚本语言无关。

**（2）没有 `Root`，shell 要提供 `ShellRoot`。**
`Root` 在 `crates/ui/src/root.rs`，属于 `gpui-component`。base 提供的是构件而非窗口级宿主：`Dialog` / `Sheet` 自带 viewport 级 host（ARCHITECTURE.md「Modal hosts」），`ToastManager` / `ToastStackState` 提供堆叠几何，`FocusTrapElement` 提供焦点陷阱。缺的是把它们组织成"窗口级覆盖层栈 + 打开/关闭 API"的那一层。

`ShellRoot` 是 `crates/shell` 里的一个 Rust 视图，职责与 `Root` 对等但不复用其代码：承载 dialog 栈、sheet、toast 栈、焦点与 Tab 导航，并把 `cx.open_dialog` / `cx.toast` 暴露给脚本（§16）。

**（3）没有 Icon / TitleBar / Notification 组件。**
`Icon`、`IconName`、`TitleBar`、`window_border` 都在 `crates/ui`。第一期脚本用 `gpui` 的 `svg()` / `img()` 按路径加载图标，shell 提供一个 `gpui.icon(path)` 薄封装与插件级图标目录约定。

**（4）Dock 没有 chrome。**
`DockArea` 不带渲染器时"能拖能停靠能持久化，但不画任何东西"。shell 需实现三个 renderer trait，把 tab bar、工具条、拖放指示器、tiles 画布**转发给脚本**（§15.1）。这既是代价也是能力：dock 的外观第一次可以由脚本决定。

### 4.4 对既有 crate 的改动约束

| crate | 本提案要求的改动 |
| --- | --- |
| `crates/base` | **无。** 遵循 CLAUDE.md"默认不修改 gpui-base"。实现中若发现缺口（例如某个 renderer 上下文读不到需要的状态），单独提案、单独 PR |
| `crates/ui` | **无。** 第一期不依赖 |
| `crates/macros` | 可选新增绑定派生宏（§14.3） |
| 新增 `crates/shell` | 运行时全部实现（`gpui-shell`） |
| 新增 `examples/js_checklist` | 最小示例，符合既有 `examples/*` 约定 |

### 4.5 100% 向后兼容

不引入 `crates/shell` 的既有消费者，构建产物与依赖树完全不变。

---

## 5. 设计原则

### 5.1 宿主提供能力，脚本负责组合与呈现

脚本能做的事等于宿主注册的 API 集合，不多也不少。新增能力必须是宿主的显式动作（这也是 §19 不暴露 quickjs-libc 的 `std` / `os`、不做 Node 兼容层的直接理由）。

### 5.2 元素是值，不是对象

`Button.new("id")` 返回的不是"活的按钮"，而是一段**元素描述**，本次渲染结束即失效。这是 GPUI 元素模型的直接后果（§8.1），不是风格选择。

### 5.3 不做 DSL，也不做 JSX

```js
// 不做这个（声明式 DSL / 属性表）：
{ button: { text: "hello", onClick: save } }

// 也不做这个（JSX，需要编译步骤）：
<Button onClick={save}>Save</Button>

// 做这个（与 Rust 同构的 builder 链）：
Button.new("save").on_click(save).child(text("Save"));
```

理由：其一，builder 链与 Rust API 一一对应，学一次即可双向迁移；其二，DSL 需要独立的解析、诊断、编辑器支持与版本演进，是第二套语言的成本；其三，JS 的函数、箭头函数与数组方法已足以表达条件与循环。

JSX 另有一条独立的否决理由：它必须先编译再执行，而"改一行、存盘、立刻看到画面"是本运行时存在的理由（§2.1）。引入编译步骤等于把这条理由退还回去。

这与 CLAUDE.md 的 GPUI builder style 同源：保持一条流式链，用 `when` 表达条件。

### 5.4 上下文只在调用期间有效

`&mut App`、`&mut Window`、`&mut Context<T>` 都是借用。Shell 用 `CallScope` 把"是否处在一次合法的宿主调用中"变成运行时可检查的事实，越界访问抛脚本异常而非 UB（§9）。

### 5.5 绑定表是数据

绑定必须以数据 + 生成的形式存在，并由 CI 报告与上游的漂移（§14.5）。手写绑定的失败模式不是"写起来累"，而是上游改了签名而绑定没改。无参样式表已经做到了这一点：它来自 GPUI 的反射表，没有一行手写名字（§13.1）。

### 5.6 呈现由脚本拥有，且必须可替换

Shell 自带的默认 token 与预设模块是**便利品，不是契约**：它们以脚本源码形式随包发布，应用可以整体替换。Shell 的 Rust 侧不得内置任何视觉决策（颜色除外，且仅以可被覆盖的 token 形式存在）——否则等于在 base 之上又造了一个不受控的第三套视觉系统。

### 5.7 默认无能力

插件的默认能力集合为空。fs / 网络 / store / clipboard / native module 全部需要 manifest 声明 + 宿主授权（§19.2）。`Capabilities::default()` 就是空集，这一条在代码里是可断言的（`crates/shell/src/capability.rs`）。

### 5.8 失败是可恢复的

脚本侧任何错误都收敛成一次带 stack 的异常：记日志、在 UI 上以错误覆盖层呈现、不影响宿主其余部分、不让 Rust panic 穿过 FFI 边界（§21）。

### 5.9 引擎是可替换的

VM 不是架构的一部分，是架构的一个参数。凡是能写在接缝之上的东西一律写在接缝之上；写在引擎里的每一样都要能说清"为什么它非在这里不可"（§6.5）。

---

## 6. 架构总览

### 6.1 运行时组成

| 模块 | 职责 | 位置 |
| --- | --- | --- |
| `engine/` | **脚本引擎接缝**：VM 生命周期、模块加载、方法派发、回调与异常转换 | 接缝之下 |
| `ShellRuntime` | 引擎为接缝实现的那一个类型；上层只认它的方法表（§6.5） | 接缝之下 |
| `CallScope` | 宿主上下文（`App` / `Window` / 当前 view）的作用域栈与有效性校验 | 之上 |
| `SpecArena` | 单次渲染内的元素描述缓冲区，渲染结束即整体释放 | 之上 |
| `materialize` | 把元素描述重放成真实 GPUI 元素（纯 Rust，不回到脚本） | 之上 |
| `style` | 反射来的无参样式表 + 有参样式绑定 + 拼写建议 | 之上 |
| `BindingRegistry` | 组件、方法、枚举的绑定表；类型声明生成的唯一事实来源 | 之上 |
| `value::Bridged` | 中立的脚本参数值（Nil / Bool / Number / Str）与颜色、长度的强制转换 | 之上 |
| `error::ShellError` | 中立的错误类型，由各引擎在边界转成自己的异常 | 之上 |
| `CallbackArena` | 按渲染代次存活的回调表，句柄类型是类型参数 | 之上 |
| `ShellRoot` | 窗口级覆盖层宿主：dialog 栈、sheet、toast 栈、焦点与 Tab 导航 | 之上 |
| `ScriptDockSkin` | 实现 base 的三个 dock renderer trait，转发给脚本 | 之上（转发调用穿过接缝） |
| `Scheduler` | 异步调度、`Task` 桥接、定时器、生命周期绑定的取消 | 之上 + 一处引擎钩子（§6.5） |
| `HostApi` | fs / http / store / clipboard / log 等能力实现 | 之上 + 各引擎一层薄绑定 |
| `Sandbox` | 语言标准库裁剪、模块解析器、能力检查、中断与内存限额 | 语言相关部分在引擎内（§19） |
| `PluginManager` | manifest 解析、能力授权、插件加载与卸载、热重载 | 之上 |

### 6.2 关键 Rust 类型

```rust,ignore
/// 由脚本驱动渲染的 GPUI 视图。名字里没有语言，因为它不需要知道语言。
pub struct ScriptView {
    runtime: Rc<ShellRuntime>,
    /// 脚本侧的 view 实例句柄，类型由引擎定义（`ViewObject`）。
    object: ViewObject,
}

impl Render for ScriptView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let runtime = self.runtime.clone();
        let object = self.object.clone();
        let entity = cx.entity();
        runtime.render_view(object, entity, window, cx)
    }
}
```

`ScriptView` 是脚本与 GPUI 之间唯一的渲染入口：脚本定义的 view、panel、dialog 内容、dock chrome 全部由它承载。注意它对 `ViewObject` 只做搬运——在 QuickJS 引擎里那是 `Persistent<Object>`，在 Lua 引擎里那是 `mlua::Table`，`ScriptView` 两者都不需要知道。

### 6.3 引擎选择

**默认 QuickJS**，`rquickjs` 0.12（已开启 `macro` / `loader` / `classes` / `properties`）。**Lua 引擎（`mlua` 0.11）保留在 `lua` / `luajit` feature 之后，是一份可编译、可运行的退路，不是文档里的承诺**（§6.5）。

| | QuickJS（默认） | LuaJIT | Lua 5.4 |
| --- | --- | --- | --- |
| 语言 | ES2023：class、模块、`async`/`await`、Proxy、解构、模板字符串 | Lua 5.1 语义 + 部分扩展 | Lua 5.4 |
| 执行 | 字节码解释器，**无 JIT** | 手写汇编解释器 + trace JIT | 字节码解释器 |
| 热点循环 | 三者中最弱 | 最强（数量级差距） | 与 QuickJS 同一量级 |
| 每次跨语言调用（`C_op`） | **待实测**，见 §20.3 与 M0 门槛 | 已知很低 | 中等 |
| wasm | 可（纯 C，emscripten 路径成熟） | **不可** | 可 |
| W^X / 禁止可执行内存的平台 | 无影响（不生成机器码） | 受限（Apple Silicon 等） | 无影响 |
| 体积 | 三者中最大：完整 ES 语义 + 正则 + Unicode | 小 | 最小 |
| GC | 引用计数 + 循环回收 | 增量标记清除 | 分代/增量标记清除 |
| 语料与生态 | 最好（§2.1） | 一般 | 一般 |

三点必须写清楚，因为它们直接改写了后面几章：

**1. 没有 JIT，意味着 §20.4 第 2 条的优化重点变了。** 在 LuaJIT 上可以指望 JIT 把方法派发的一部分开销消掉；QuickJS 上没有这一层，`el.gap_2()` 每次都是一次原型属性查找 + 一次 JS 函数调用 + 一次进 Rust 的 host call。实现据此做了两个具体选择：**元素是共享同一个原型的普通 JS 对象，方法装在原型上**（不是每个元素一个 class 实例，也不是每次访问新建闭包）；而这个原型本身是一个 `Proxy`，用来在拼错方法名时给出"did you mean"。后者每次调用多一次 `Reflect.get`，是一次自觉的取舍，理由与退路见 §13.2。

**2. 引用计数是一个真实的好处，不只是特性差异。** 宿主句柄（`Persistent<Function>`、`Persistent<Object>`）在最后一个引用消失时立即释放，§7.4 的跨 GC 环因此少一层不确定性。代价是每次赋值都有计数开销，且真正的环仍要等循环回收器。

**3. 体积与热点性能这两项，QuickJS 都不占优，而 v0.3 选 LuaJIT 时给它们的权重更高。** 这次不是"发现原来的论证错了"，而是权重变了：应用层代码的可读性与语料覆盖被排到了前面，而接缝（§6.5）把这个选择从单向门变成了可回退的门。若 M0 的实测把 `C_op` 判死，切回 LuaJIT 是改一个 feature（R12）。

**约束**：脚本 API 与文档示例必须落在**两个引擎都能表达**的语义交集内。这不要求两种语言长得一样——`Counter` 在 JS 里是 `class extends View`，在 Lua 里是 `gpui.view("Counter")` 加方法——而要求**行为一致**：同一个用例在两个引擎下必须产出同一棵 spec 树（§22.3）。

### 6.4 JavaScript 模块面

一次 `import`，一个模块；**组件以类型表的形式给出**：

```js
import { View, div, v_flex, h_flex, text, Button, Checkbox, Switch } from "gpui";

export default class Counter extends View {
  init(props = {}) {
    this.count = props.start ?? 0;
  }

  render(cx) {
    return v_flex()
      .gap_2()
      .child(text(`${this.count}`))
      .child(
        Button.new("increment")
          .on_click((_event, cx) => {
            this.count += 1;
            cx.notify();
          })
          .child(text("Increment")),
      );
  }
}
```

`"gpui"` 是唯一的内建模块名，其余 `import` 只能解析到应用目录内（§19.1）。应用的入口是 `main.js`，它必须 `export default` 一个 view class；宿主取这个 class，构造一个实例，挂成窗口根视图。

命名规则直接来自 Rust 侧的形态：

| Rust 里是什么 | JS 里是什么 | 例子 |
| --- | --- | --- |
| 类型 + `::new` | **大写类型表**，构造只有 `.new` | `Button::new(id)` → `Button.new(id)` |
| 自由函数 | 小写函数 | `div()`、`h_flex()`、`v_flex()`、`text(s)` |
| 状态实体 | 大写类型表 | `InputState::new(...)` → `InputState.new({...})` |
| 宿主运行时能力 | `gpui` 命名空间下的小写函数 | `gpui.spawn`、`gpui.timer`、`gpui.open_window`、`gpui.action`、`gpui.keymap`、`gpui.theme`、`gpui.memo` |
| 系统能力 | 小写子对象 | `gpui.fs`、`gpui.http`、`gpui.store`、`gpui.clipboard`、`gpui.log`、`gpui.native` |
| 视图基类 | `class X extends View` | `export default class Counter extends View` |

#### 样式与行为方法保留 Rust 的 snake_case 拼写

`items_center`、`size_full`、`gap_2`、`text_3xl`、`on_click`、`border_color` 在 JS 里就是这个拼写，**不提供 camelCase 别名**。这是一处明知违反 JS 习惯的选择，理由要写清楚：

1. **这些名字不是手写的。** 无参样式方法整张表来自 GPUI 的反射（`gpui_base::styled_ext_reflection_methods` + `gpui::styled_reflection::methods`，§13.1），上游新增一个方法，脚本侧自动就有。加 camelCase 别名等于把这张零维护的表变成一张要维护的表。
2. **机械转换在这批名字上没有良定义。** `items_center` → `itemsCenter` 是清楚的，`gap_2` → `gap2` 还是 `gap_2`？`text_3xl` → `text3xl` 还是 `text3Xl`？`rounded_tl` → `roundedTl` 还是 `roundedTL`？带数字段与双字母缩写的名字上，任何一条规则都会在某几十个名字上给出别扭结果，而这些名字每天都要写。
3. **一件事只有一种写法（§6.4 末尾的约定）。** 同时提供两种拼写，会立刻分裂示例、类型声明、文档与模型生成的代码——而"两套等价写法"正是本文一贯拒绝的东西（§8.2 方案 C、§13.2 的 `class` 字符串）。

代价也如实说：JS 作者第一次看到 `.items_center()` 会觉得不像 JS；同一份文件里于是有两种命名风格——凡是绑定来的名字是 snake_case，凡是作者自己写的（`this.visibleItems()`、`onSubmit`）是 camelCase。`examples/js_checklist` 就是这个样子，读下来并不刺眼，但这是审美判断，不是论证。

**如果将来决定改**：在原型构建处（`crates/shell/src/engine/quickjs.rs` 的 prelude 里那句 `for (const name of __styleNames) define(name)`）加一个名字改写函数即可，一行的事，两种拼写也可以同时装上。真正不可逆的是文档与生态里已经写下的那一批调用点，所以现在就要定。

#### 元素用 `.new(id)` 而不是 `new Button(id)`

JS 的习惯写法是 `new Button(id)`。不用它，是因为返回的**不是**一个对象，而是一段单次渲染内有效的元素描述（§8.3）——`new` 会暗示一个有身份、能保存、能复用的实例，而这正是本运行时最需要作者不要以为的事（重复使用会抛异常）。`Button.new(id)` 与 Rust 侧一字不差，也让"这是一次构造描述"这件事在调用点保持中立。

反过来，**view 用的就是标准 `class extends View`**，因为 view 确实有身份、有跨帧状态、由 GPUI 拥有（§7.3）。同一份文件里两种构造形态，是因为这两类东西的生命周期本来就不同——这一点值得在入门文档里第一段就讲清楚。

#### 两条配套约定

- **一件事只有一种写法。** 不提供 `gpui.button(id)` 这类小写组件工厂，也不给类型表加 `[Symbol.hasInstance]` / 可调用糖。
- **能力子对象可以解构**：`const { fs, http } = gpui;`。这与参考项目里 `local api = vim.api`、`const vscode = require("vscode")` 是同一个习惯：一个宿主命名空间，按需下放到局部，沙箱只需守住一个入口（§19.1 的模块解析器因此只放行 `gpui` 与应用自身的文件）。

第二期的成品视觉组件走另一个模块名，形态对称：

```js
import { Button } from "gpui-component";   // 自带产品视觉的 Button，同样只有 .new
```

两个模块都有 `Button`，但语义不同——`gpui` 的交出呈现权，`gpui-component` 的自带产品视觉。分成两个模块名正是为了让这个差别在 import 行可见，也让"换 import 即换视觉"成立（§14.6）。

### 6.5 引擎接缝

引擎选择是本运行时唯一无法在纸面上判定的决定（§20），所以它被做成一条可以走回头路的接缝，而不是一个假设。

#### 契约

`crates/shell/src/engine/mod.rs` 定义契约。一个引擎模块必须导出一个 `ShellRuntime` 类型，恰好提供这些方法，以及两个对调用方完全不透明的句柄类型 `ViewType` / `ViewObject`：

```text
ShellRuntime::new() -> anyhow::Result<Rc<Self>>
ShellRuntime::set_global(&Rc<Self>, &mut App)
ShellRuntime::global(&App) -> Option<Rc<Self>>
ShellRuntime::arena_mut(&self) -> RefMut<'_, SpecArena>

ShellRuntime::load_app(&Rc<Self>, &Path) -> anyhow::Result<ViewType>
ShellRuntime::load_source(&Rc<Self>, &str, &str) -> anyhow::Result<ViewType>
ShellRuntime::instantiate(&Rc<Self>, &ViewType) -> anyhow::Result<ViewObject>

ShellRuntime::render_view(&Rc<Self>, ViewObject, Entity<ScriptView>, &mut Window, &mut App)
    -> AnyElement
ShellRuntime::render_to_spec(&Rc<Self>, &ViewObject, Option<Entity<ScriptView>>,
    &mut Window, &mut App) -> anyhow::Result<String>

ShellRuntime::dispatch_click(&Rc<Self>, CallbackId, &ClickEvent, &mut Window, &mut App)
ShellRuntime::dispatch_change(&Rc<Self>, CallbackId, bool, &mut Window, &mut App)
```

crate 的其余部分**不调用别的任何东西**。这句话才是接缝的定义：接缝不是一个 trait，而是"上层只用得到这十一个入口"这一事实。用 trait 反而做不到——`ViewType` / `ViewObject` 各带自己的生命周期与 `'js` 标注，硬套一层 trait 只会把复杂度搬到类型系统里。

#### 恰好一个引擎

```rust,ignore
#[cfg(all(feature = "quickjs", any(feature = "lua", feature = "luajit")))]
compile_error!(
    "enable exactly one scripting engine: `quickjs` (default) or `lua`/`luajit`. ..."
);

#[cfg(not(any(feature = "quickjs", feature = "lua", feature = "luajit")))]
compile_error!("enable one scripting engine: `quickjs` (default) or `lua`/`luajit`");
```

两个引擎导出同名类型，同时开启会让 `gpui_shell::ShellRuntime` 有歧义。这里不用"默认引擎兜底"的静默行为：feature 组合错了应当在编译期报出来，并且直接告诉使用者怎么改。构建退路的命令因此是：

```bash
cargo build -p gpui-shell --no-default-features --features luajit
```

#### 接缝的两侧

| 接缝之上（两个引擎共用，源码里不出现 VM 名字） | 接缝之下（每个引擎各写一份） |
| --- | --- |
| `spec.rs`：元素描述 arena、单次使用检查、`debug_tree` | 引擎值 → `Bridged` 的转换（`Value::as_number` vs `mlua::Value`） |
| `materialize.rs`：描述 → 真实元素，纯 Rust | 模块系统形态（ES module + resolver vs `require` + `package.path`） |
| `scope.rs`：CallScope、phase、代次校验、唯一的 `unsafe` | 方法派发（共享原型上的函数 vs `__index` 元方法 + 方法缓存） |
| `style.rs`：反射表、有参样式、拼写建议 | 回调句柄类型（`Persistent<Function>` vs `mlua::Function`） |
| `theme.rs`：默认 token 调色板与 token 名解析 | 异常类型转换（`ShellError` → `Exception` / `LuaError`） |
| `capability.rs`：能力集合与路径解析 | view 定义形态（`class extends View` vs 元表 + `gpui.view(name)`） |
| `value.rs`：`Bridged` 与颜色、长度的强制转换 | 沙箱中与语言绑定的部分（intrinsics 取舍 vs 标准库裁剪，§19） |
| `error.rs`：中立的 `ShellError` | |
| `runtime.rs`：`CallbackArena<T>`、错误覆盖层 | |
| `view.rs`：`ScriptView` | |

比例本身就是结论：接缝之上是元素模型、样式、主题、能力、上下文安全这些**真正的设计**，接缝之下是"脚本值长什么样"。

#### 新增能力的规则

**任何新能力都加在接缝之上，除非它在语言层面确实无法表达。** 三条判据依次问：

1. 它需要知道脚本值长什么样吗？不需要 → 接缝之上。
2. 它能用 `Bridged` + `SpecOp` + `ShellError` 表达吗？能 → 接缝之上，引擎里只留一层参数搬运。
3. 若确实必须落在引擎里：**两个引擎都要实现**，或者在缺失的一侧抛出明确异常。一个引擎有、另一个静默无行为，是这条接缝最容易腐烂的方式，也是 CI 行为套件（§22.3）要抓的东西。

`host.rs` / `scheduler.rs` / `sandbox.rs` 目前还是占位，它们是这条规则的第一批考题：`gpui.fs` 的路径解析与能力检查全部属于第 1 类（已经在 `capability.rs` 里），引擎里应当只剩"把参数取出来、把结果放回去"。

#### 已知的一处缺口：异步

契约里目前没有异步。Lua 的 coroutine 与 JS 的 Promise 不是同一个东西，QuickJS 还额外要求宿主主动把 job queue（microtask）跑完，否则 `await` 之后的代码永远不执行。因此 §12 的调度器**不能整体落在接缝之上**，它至少需要引擎再提供两个操作：

- 把一个宿主 `Task<T>` 变成脚本侧可等待的值（JS 里是 Promise，Lua 里是可被 resume 的 coroutine 挂起点）；
- 把待执行的 job 跑完（QuickJS 有，Lua 没有对应物，实现为空操作）。

这两条必须在 M3 之前补进上面的契约，而**不是**在两个引擎里各写一套调度器——否则接缝就名存实亡。这是当前契约唯一已知的缺口，写在这里而不是留给实现者去发现。

---

## 7. 对象模型

脚本与 Rust 之间的每个对象都属于且只属于以下三类之一。

| 类别 | Rust 侧 | 脚本侧表示 | 生命周期 | 例子 |
| --- | --- | --- | --- | --- |
| **值 (Value)** | `Copy`/`Clone` 的小数据 | number / string / boolean / 普通对象 | 传递即复制 | `Pixels`、`Hsla`、`ElementId`、枚举、`Point`、`Edges` |
| **元素描述 (Spec)** | arena 中的节点 id | 共享原型的轻量对象（只带一个 `__id`） | **单次渲染** | `div()`、`Button.new(...)` |
| **实体 (Entity)** | `Entity<T>` / `WeakEntity<T>` | 宿主类实例 + 弱句柄 | 跨帧，由 GPUI 拥有 | `InputState`、`TreeState`、`DockArea`、`Window`、`ScriptView` |

### 7.1 值

转换由 `value.rs` 的 `Bridged` 负责，规则一处定义、全局一致（这一层在接缝之上，两个引擎共用同一套规则与同一批错误信息）：

| 脚本输入 | 目标类型 | 规则 |
| --- | --- | --- |
| `12` | `Pixels` | `px(12.)` |
| `"50%"` | `DefiniteLength` | `relative(0.5)` |
| `"#1e88e5"` / `"#1e88e5cc"` | `Hsla` | 十六进制解析 |
| `"accent"` | `Hsla` | 语义 token 查表（§13.3） |
| `[8, 12]` | `Edges<Pixels>` | 二元组 → 垂直/水平 |
| `{ top: 8 }` | `Edges<Pixels>` | 具名字段 |
| `"sm"` | 枚举 | 名字匹配；失败时报错并列出全部合法值 |

**枚举与 token 的错误必须列出合法值。** 已实现的形态是：

```text
unknown color token `surfacee`; expected one of: background, foreground, surface, … — or a #rrggbb literal
```

比 `invalid argument #1` 有用一个数量级。`null` 与 `undefined` 都归一成 `Bridged::Nil`，因为在调用点它们是同一个意思：这个参数没给。

### 7.2 元素描述

见 §8。核心约束：**离开本次渲染即失效，重复使用报错。**

### 7.3 实体

脚本侧持有的是宿主对象，内部是 `WeakEntity<T>` 加宿主代次号：

```js
const state = InputState.new({ placeholder: "Search" });
state.set_value("hello");
console.log(state.value());
```

规则：

1. 脚本持有**弱句柄**；真实所有权在 GPUI（通常是某个 view 或 `ScriptView`）。
2. 访问已释放实体抛异常（`attempt to use a released InputState`），不返回 `undefined`，避免错误静默传播——`undefined` 在 JS 里会一路飘到很远的地方才炸。
3. 实体方法只能在 `CallScope` 内调用（§9）。
4. 脚本侧的回收**不**释放 Rust 实体，只释放弱句柄本身。

### 7.4 跨 GC 的循环引用

嵌入式脚本最经典的泄漏来源：脚本闭包被 Rust 持有（`Persistent<Function>` / `RegistryKey`），闭包又捕获了指向 Rust 实体的宿主对象，两个 GC 各自看不到对方的边。

- **每帧回调**（`on_click` 等）存在 `CallbackArena`，下一次渲染整体替换、整体释放，不构成长期环。已实现：`runtime.rs` 的 `CallbackArena<T>`，代次编在 `CallbackId` 里。
- **长期回调**（实体事件订阅、定时器、命令处理器）必须绑定 **owner**（view 或 plugin）。owner 销毁时其注册的长期回调一并释放。
- 提供 `gpui.gc_stats()`：存活 view 数、持久句柄数、arena 峰值。M1 起纳入调试面板。
- QuickJS 的引用计数让"最后一个引用消失即释放"成立，环仍需回收器；这减轻但没有消除这一类问题，所以上面三条一条都不能省。

---

## 8. 渲染协议

**本章与脚本语言无关**，两个引擎共用同一份实现（`spec.rs` + `materialize.rs`）。

### 8.1 约束：GPUI 的元素是被消费的值

```rust,ignore
#[derive(IntoElement)]
pub struct Button { /* ... */ }

impl RenderOnce for Button {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement { /* ... */ }
}
```

三个事实决定一切：

1. `render(self, ...)` **按值消费**元素，一个元素值只能被使用一次；
2. `.child(impl IntoElement)` 同样按值取走子元素；
3. 视图的 `Render::render` 在每次需要重绘时**从零重建**整棵元素树。

因此 v0.1 中"脚本 Button 对象 ↔ Rust Button Entity"的映射不成立：Button 从来不是 entity。

### 8.2 三个候选方案

| 方案 | 做法 | 结论 |
| --- | --- | --- |
| **A. 保留式脚本控件树 + Rust 镜像** | 脚本长期持有控件对象，Rust 维护镜像树并 diff | **否决。** 要在 GPUI 之上再造一层 VDOM/reconciler，而 GPUI 本身就是每次重建 |
| **B. 渲染函数 + 一次性元素描述** | 脚本实现 `render(cx)` 返回描述树；Rust 在 `Render::render` 内物化为真实元素，随后整体丢弃 | **采纳。** 与 GPUI 模型同构，无 reconciler，无跨帧元素生命周期 |
| **C. 纯数据对象描述** | 脚本返回嵌套对象字面量，Rust 递归解释 | **否决。** 它与 B 完全等价却是第二套写法：示例、文档、类型声明与模型生成的代码会立刻分裂成两派，而它想省的每 op 一次 host call，用 `gpui.memo` 与虚拟化同样能拿到（§20.4） |

方案 C 在 JS 下比在 Lua 下更有诱惑力——对象字面量是 JS 作者最顺手的东西，React 生态又让"UI 是数据"成了肌肉记忆。否决理由不变，而且更硬：一旦两种写法并存，`.d.ts` 要描述两套形状，模型会随机产出其中一套。

### 8.3 ElementSpec Arena

```rust,ignore
/// 单次渲染内的元素描述缓冲区。渲染结束后整体清空。
pub struct SpecArena {
    nodes: Vec<SpecNode>,
    /// 已被作为子元素挂载的节点，重复挂载即报错。
    parented: Vec<bool>,
}

struct SpecNode {
    component: Option<Component>,            // div / h_flex / v_flex / text / Button / …
    ops: SmallVec<[SpecOp; 8]>,              // 有序记录的 builder 调用
    children: SmallVec<[SpecId; 4]>,
}

pub enum SpecOp {
    NullaryStyle(u16),                       // 反射表下标（§13.1）
    ParamStyle(&'static str, SmallVec<[Bridged; 2]>),
    Method(&'static str, SmallVec<[Bridged; 2]>),
    Callback(&'static str, CallbackId),
}
```

脚本侧的元素只包一个 `SpecId`（JS 里是对象上的 `__id` 属性，Lua 里是 userdata 字段）。每次方法调用把一条 `SpecOp` 推进 arena 并返回自身，形成链式写法。

**"消费语义"在脚本侧的还原**：节点被 `.child(other)` 挂载时置位 `parented`；再次使用（挂到第二个父节点、或跨帧复用）抛出：

```text
element `Button` was already added to a parent; elements are single-use values
```

把 Rust 的移动语义翻译成一条**明确的运行时错误**，而不是诡异画面或 panic。

### 8.4 渲染流程

```text
cx.notify() / 事件 / 状态变化
        │
        ▼
ScriptView::render(window, cx)
        ├─ CallScope::enter(window, cx, view)            §9
        ├─ SpecArena::reset() · CallbackArena::swap()
        ├─ 调用脚本 render(cx)  →  返回 root SpecId
        ├─ materialize(root) → AnyElement                （纯 Rust，深度优先）
        ├─ 上一帧的回调随 swap 释放
        └─ CallScope::exit()
        │
        ▼
GPUI layout / paint（全程不回到脚本，虚拟化 item renderer 除外）
```

`materialize` 是纯 Rust 的深度优先遍历：按 `component` 取构造器，顺序重放 `ops`，递归物化 `children`，产出 `AnyElement`。这一步**不触及脚本、也不知道脚本是什么语言**，因此可以单独 benchmark，也可以单独快照测试（§22.1）。

### 8.5 重入规则

base 的若干组件在 GPUI 的 layout / prepaint 阶段回调应用代码渲染单项：`VirtualList`、`Tree` 的 `TreeEntry`、`Calendar` 的 `CalendarItem`、`Table` 的单元格，以及本方案新增的 dock renderer（§15.1）。这些回调发生在 `ScriptView::render` 之外。

1. 允许在其中调用脚本，但**必须打开嵌套 CallScope**，标记 `ScopePhase::Layout`。
2. `Layout` 下**禁止** `cx.notify()`、禁止创建/销毁实体、禁止 `spawn`——布局期改状态会导致本帧不一致或递归失效。违反抛异常。
3. 这是性能敏感路径：为其提供预分配的参数对象复用，避免每行一次对象分配。

### 8.6 记忆化

元素不可跨帧缓存（它们是被消费的值），但**描述可以**：

```js
const rows = gpui.memo(this.dataVersion, () => buildRows(this.data)); // 返回 SpecId 子树
```

key 未变时跳过脚本构建，复用上一帧的 arena 子树片段；物化仍每帧进行——那是纯 Rust 的廉价部分。价值在 §20 量化。QuickJS 没有 JIT，这条优化的相对收益因此比在 LuaJIT 上更大。

---

## 9. 上下文安全模型：CallScope

**本章与脚本语言无关**，实现是 `scope.rs`，也是整个 crate 唯一的 `unsafe` 模块。

### 9.1 问题

GPUI 的核心上下文都是借用：

```rust,ignore
fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement
fn on_click(&mut self, event: &ClickEvent, window: &mut Window, cx: &mut App)
```

脚本对象的生命周期由脚本 GC 决定，无法承载 Rust 借用。一个被存进模块作用域、事后从定时器调用的 `cx`，指向的是早已失效的栈帧。JS 的闭包让这件事更容易发生，因为箭头函数捕获外层变量不需要任何显式动作。

### 9.2 设计

```rust,ignore
/// 宿主调用期间有效的上下文。脚本侧的 `cx` 只是它的一个代次令牌。
pub fn enter(window: &mut Window, app: &mut App, phase: ScopePhase,
             view: Option<Entity<ScriptView>>) -> (CallScopeGuard, u64);
pub fn with_context<R>(generation: u64,
             f: impl FnOnce(&mut Window, &mut App) -> R) -> Result<R, StaleContext>;
```

- 每一次 Rust → 脚本的入口（渲染、事件、定时器、Task 完成、命令执行、dock renderer）都压入一个 scope，返回时由 `CallScopeGuard` 弹出。
- 脚本侧的 `cx` 只是一个宿主对象，唯一携带的是 `generation`；每次方法调用先与栈顶比对，不匹配即抛：

  ```text
  cx is no longer valid: it was captured during an earlier call and used later.
  Use gpui.spawn or take cx from the callback arguments instead.
  ```

- `unsafe` 全部收敛在这一个模块，前提条件写进模块文档：VM 与 `App` 同为主线程独占；scope 严格后进先出；指针在 guard 存活期间必然有效。
- 脚本无法伪造 generation：`cx` 上没有暴露任何字段，代次只存在于 Rust 闭包捕获里。JS 侧即使 `Object.keys(cx)` 也只看得到 `notify` 与 `phase` 两个函数。

### 9.3 `cx` 的能力按 phase 分级

| Phase | 允许 | 禁止 |
| --- | --- | --- |
| `Render` | 读状态、读主题、构建元素、注册回调 | `notify`（渲染中通知自身是死循环）、阻塞 |
| `Event` | 全部：改状态、`notify`、`spawn`、开窗、开 dialog | 阻塞 |
| `Task` | 同 `Event` | 阻塞 |
| `Layout` | 只读 + 构建元素（§8.5） | `notify`、创建/销毁实体、`spawn` |

**每条禁止都对应一条明确的错误信息**，而不是未定义行为。已实现的一条：

```text
cx.notify() is not allowed during the `render` phase;
request a re-render from an event handler instead
```

---

## 10. 事件与回调

### 10.1 注册与存活期

```js
Button.new("save")
  .child(text("Save"))
  .on_click((event, cx) => {
    this.saved = true;
    cx.notify();
  });
```

箭头函数在这里不是风格选择：它不绑定自己的 `this`，因此 `this` 仍是 view 实例，回调里可以直接改状态。**用 `function () {}` 写事件处理器会拿到错误的 `this`**——这是 JS 作者最常踩、模型也最常写错的一处，文档与 `.d.ts` 的示例必须一律用箭头函数。

`on_click` 把函数存入本帧 `CallbackArena`，`SpecOp::Callback` 只记 `CallbackId`（高 16 位是渲染代次，低 16 位是下标）。物化时 Rust 装配的闭包持有 `Weak<ShellRuntime>` + `CallbackId`。

回调**属于产生它的那次渲染**，下一次渲染整体替换（上一代多留一帧，因为事件可能在 render 与 paint 之间派发）。若事件在两代之外才派发，代次不匹配则静默丢弃并记 `debug` 日志——不报错，因为这不是作者的错误。

### 10.2 事件对象

事件以普通对象传入，字段名与 Rust 结构一致：

```js
.on_click((event, cx) => {
  // event.click_count === 1
  // event.modifiers === { shift: false, control: false, alt: false, platform: true }
});
```

原则：**只暴露 base 已经规范化过的语义**，不暴露平台原始事件。base 已把"回车激活按钮"与"点击按钮"归一成同一个回调，脚本侧不应看到这个差别。字段名保留 snake_case，理由同 §6.4：它们是 Rust 结构的镜像，不是本地发明的名字。

### 10.3 受控值回调

base 的受控组件不自行改状态，只报告意图（ARCHITECTURE.md「Controlled values」）。绑定必须保持这条语义：

```js
Checkbox.new("agree")
  .checked(this.agreed)                    // 值来自脚本状态
  .on_change((checked, cx) => {            // 只是"请求"
    this.agreed = checked;
    cx.notify();
  });
```

绝不允许 shell 悄悄替脚本维护勾选状态：那会让脚本作者与 Rust 作者拥有不同的心智模型，而两者共存于同一个应用里。

### 10.4 状态样式

base 的 `state_style::resolve_style` 提供 checked / selected / disabled 等语义状态的样式优先级（STYLING-AND-MOTION.md）。脚本需要能表达它们，否则"呈现权在脚本"就是空话：

```js
Checkbox.new("agree")
  .checked(this.agreed)
  .style_checked({ bg: "primary", border_color: "primary" })
  .style_disabled({ opacity: 0.5 })
  .hover({ bg: "accent" });                // GPUI 原生运行时修饰符
```

三种机制（普通 `Styled`、语义状态样式、GPUI 原生 hover/active/focus-visible）在脚本侧保持与 Rust 侧相同的分工与优先级，不做简化合并。M0 尚未实现这一组方法，条件样式暂时写成 `.when(cond, (el) => ...)`。

### 10.5 Actions 与快捷键

GPUI 的 action 是类型，`actions!` 在编译期生成，脚本无法生成 Rust 类型。因此：

- Shell 定义统一的 `ShellAction { id: SharedString }`，所有脚本 action 都是它的实例；
- `gpui.action("myplugin.reload")` 注册 id，`gpui.keymap({ "cmd-shift-r": "myplugin.reload" })` 绑定按键；
- 派发时按 id 查表调用脚本处理器。

**已知约束**：action 名参与 keymap 匹配且是 `&'static str` 语义，脚本注册的 id 需 intern 成 `&'static str`（一次性 `Box::leak`，数量由已加载插件界定，卸载不回收）。写进文档，不留给未来的读者去发现。

### 10.6 实体事件订阅

```js
const state = InputState.new({});
const sub = state.on_event("change", (value, cx) => { /* ... */ });
sub.unsubscribe();          // 显式；或随 owner view 销毁自动释放
```

订阅是**长期回调**，必须绑定 owner（§7.4）。事件名取自 Rust 侧的 `InputEvent` 等枚举变体，由绑定表提供合法值列表，拼错时报出全部合法值。

---

## 11. 状态与响应式

### 11.1 三层状态

| 层 | 存放位置 | 适用 | 变更后 |
| --- | --- | --- | --- |
| view 局部状态 | view 实例的字段（`this.count`） | 展开/折叠、筛选条件、临时输入 | `cx.notify()` |
| 宿主实体状态 | `Entity<T>`（`InputState` / `TreeState` / `DockArea`…） | 文本、树、表格、dock 布局 | 实体自行 notify |
| 应用/插件全局 | `store`（§17.3）或模块作用域 | 设置、缓存、会话 | 订阅者显式 notify |

### 11.2 不做自动依赖追踪

不引入 signal / observable / 自动 `notify`：

1. GPUI 本身是显式 `cx.notify()` 模型，两套心智模型共存会互相干扰；
2. 自动追踪要把每个 view 实例包进 `Proxy`，在渲染路径上是持续开销，而 QuickJS 没有 JIT 来摊薄它；
3. 显式 notify 的漏写有确定症状（界面不更新），排查成本远低于自动追踪的过度触发。

这一条在 JS 下要说得更明确，因为整个前端生态的默认假设正相反：**这里没有 signal，没有 `useState`，没有依赖数组。改完状态自己调 `cx.notify()`。**

补偿：一次事件回调内的多次 notify 合并为一次重绘。

### 11.3 view 定义

```js
import { View } from "gpui";

export default class Counter extends View {
  init(props = {}) {            // 构造时调用一次，CallScope = Event
    this.count = props.start ?? 0;
  }

  render(cx) {                  // CallScope = Render，必须返回恰好一个元素
    return v_flex().gap_2().child(text(`${this.count}`));
  }

  dispose() {                   // 实体销毁前调用，释放长期回调/任务
  }
}
```

`View` 的构造函数只做一件事：如果子类定义了 `init`，就调用它。之所以不让作者直接写 `constructor`，是因为 `constructor` 里必须先 `super(props)` 才能碰 `this`，而这条规则每被忘记一次就是一个红字异常；`init` 没有这个陷阱，也让"创建时跑一次"的语义与 Lua 引擎一致。

类名不是标识符：view 的名字取自 class 名（`Counter.name`），用于错误信息、DevTools 与 panel 注册。匿名默认导出（`export default class extends View {}`）会拿到一个空名字，加载时报警告。

---

## 12. 异步模型

### 12.1 执行器

本 workspace 没有 tokio。GPUI 提供：

- **foreground executor**：主线程、与 UI 同线程、可访问 `App`；
- **background executor**：线程池、只跑 `Send` 的纯计算或 IO。

脚本只在 foreground 上运行，永不进入 background。

### 12.2 Promise 与 `async` / `await`

v0.3 用 Lua coroutine 手工实现了一套 await；JS 里这是语言自带的，宿主要做的是把 Promise 与 GPUI 的 `Task` 接起来：

```js
gpui.spawn(async () => {
  const response = await gpui.http.get("https://api.example.com/items");
  if (response.status !== 200) return;

  gpui.with_cx((cx) => {              // 重新取一个有效的 cx
    this.items = JSON.parse(response.body);
    cx.notify();
  });
}, { owner: this });
```

实现要点：

1. 宿主能力（`http.get`、`fs.read_text`、`gpui.native.*`）在 Rust 侧返回一个 **Promise**，由背后的 `Task<T>` 完成时 resolve 或 reject。
2. `gpui.spawn(fn, opts)` 调用 `fn` 拿到 Promise，在 foreground executor 上跟踪它；每一次 resolve 之后宿主必须**把 job queue 跑完**（QuickJS 的 microtask 不会自己执行），并且这一段要跑在 phase = `Task` 的新 `CallScope` 里——continuation 里随时可能调用宿主 API。
3. **不得跨 `await` 缓存 `cx`。** `await` 之后代次已变，使用会得到 §9.2 的错误。规范写法是 `gpui.with_cx(...)`，或从回调参数重新取。这一条比 Lua 版更容易违反：`await` 前后是同一个词法作用域，`cx` 就在手边。`.d.ts` 把 `cx` 标成 `CallContext` 而非可存值，并在文档里点名这个陷阱。
4. **未处理的 rejection 必须可见。** JS 里一个没有 `catch` 的失败 Promise 默认是静默的；宿主注册 rejection 钩子，把它按 §21.1 的事件期错误处理（日志 + toast），绝不吞掉。
5. **第一期不支持模块顶层 `await`。** 加载路径（`load_source`）要求模块求值同步完成；顶层 `await` 会让它挂起，此时报出明确错误而不是超时。需要启动时异步的，在 `init` 里 `gpui.spawn`。

### 12.3 取消与所有权

```js
const task = gpui.spawn(fn, { owner: this });
task.cancel();
```

owner 销毁时其未完成任务全部取消。这条规则消除了"面板已关闭，回调还在改它的状态"——在脚本里这类问题不会 panic，只会静默写入一个再也不会被渲染的对象，因此更难发现，必须由运行时兜住。

取消一个已经进入 `await` 的 JS 函数没有语言级手段（没有可中断的 coroutine），所以取消的语义是：**不再 resume**——宿主丢弃 continuation 并释放 owner 的句柄，后续的 `gpui.with_cx` 因代次失效而抛错。这与 Lua 引擎"不再 resume 协程"的效果一致，但机制不同，值得在两个引擎的行为套件里各测一次。

### 12.4 后台工作

**明确不提供**在 background 上跑脚本。允许脚本派发到后台的只有宿主实现的 Rust 任务（`http.get`、`fs.read`、`gpui.native.<module>.<fn>`），入参与返回值必须是可跨线程传递的纯数据。不提供 `Worker`。

### 12.5 定时器

```js
const t = gpui.timer.after(300, (cx) => { /* ... */ });
const i = gpui.timer.every(1000, (cx) => { /* ... */ });
i.cancel();
```

同样绑定 owner。`every` 在窗口不可见时的行为由宿主策略决定（默认继续，文档说明可被降频）。

**不提供全局 `setTimeout` / `setInterval`。** 它们不是 JS 语言的一部分（宿主给的），而且没有 owner：一个 `setInterval` 会在面板关闭之后继续跑，这正是 §12.3 要根除的东西。但模型生成的代码一定会用它们，所以全局位置上放一个**只抛错的桩**：

```text
setTimeout is not available; use gpui.timer.after(ms, fn) — it is cancelled with its owner
```

有名字而报错，好过没名字而 `ReferenceError`（§2.1）。

---

## 13. 样式与主题

第一期呈现权在脚本，本章因此是**核心章节**而非配套章节。整章位于接缝之上：`style.rs` 与 `theme.rs` 两个引擎共用。

### 13.1 无参样式方法：来自反射，零维护

`crates/ui/src/inspector.rs` 已经在做一件对本提案极其有用的事：

```rust,ignore
let table: Vec<_> = [
    gpui_base::styled_ext_reflection_methods::<StyleRefinement>(),
    gpui::styled_reflection::methods::<StyleRefinement>(),
]
.into_iter()
.flatten()
.map(|method| (Box::new(method.invoke(StyleRefinement::default())), method))
.collect();
```

运行时即可得到一张 `名字 → 样式方法` 的表。Shell 复用同一对 API（注意：两者分别来自 `gpui-base` 与 `gpui`，**都不需要依赖 `gpui-component`**），把全部无参样式方法（`flex`、`flex_col`、`items_center`、`gap_2`、`rounded_md`、`text_sm`、`size_full`…）一次性暴露给脚本，上游新增自动可用。

三个必须写明的约束：

1. `FunctionReflection::invoke` 只接收 receiver，**因此反射只覆盖无参方法**。有参方法需显式绑定，当前实现是 57 个（`crates/shell/src/style.rs` 的 `PARAM_STYLES`）：尺寸 9、内边距 7、外边距 7、定位 5、flex 6、绘制 6、边框与圆角 17。刻意未绑的几个（`shadow`、`cursor`、`text_align`、`font_weight`、`scrollbar_width`）连同理由写在该文件头部。
2. `styled_ext_reflection_methods` 在 base 中是 `#[cfg(any(feature = "inspector", debug_assertions))]`。**`crates/shell` 必须开启 `gpui-base/inspector`**，否则 release 构建下样式表为空。CI 需要一条 release 断言：表条目数 > 0。
3. 反射作用在 `StyleRefinement` 上；组件的**行为方法**（`disabled`、`selected`、`checked`、`on_click`、`on_change`）不在其中，是手写的一小张表。

QuickJS 引擎多做一步：启动时把这张名字表（`style::known_names()`）交给一段 JS prelude，由它循环生成原型上的方法，每个方法体只是 `__apply(this.__id, name, args); return this;`。**不是三千个 Rust 闭包**——那既占内存又要每个都跨语言注册。上游加一个样式方法，这里一行都不用改。

### 13.2 有参样式

样式只有一种表达：**与 Rust 同构的 fluent 链**。无参方法来自 §13.1 的反射表，有参方法显式绑定，二者在调用点无差别：

```js
v_flex()
  .size_full().items_center()               // 反射来的无参方法
  .bg("surface").p(12).rounded(8).gap_2();  // 显式绑定的有参方法
```

不提供 `.class("flex gap_2")` 这类字符串批量写法。它与链式调用完全等价，却会立刻变成第二套样式语法：示例、类型声明、编辑器补全与模型生成的代码都要同时支持两种，而字符串形式恰恰是补全与静态检查最弱的一种。

#### 拼错的样式名：诊断如何保住

原型派发（§6.3）本身给不出好诊断：原型上不存在的名字根本到不了 `__apply`，QuickJS 只会抛一句 `TypeError: not a function`——它连属性名都不带。

实现的取法是**把原型本身做成 `Proxy`**：命中就原样返回原型上的函数，未命中且名字不以 `__` 开头，就返回一个"调用即报错"的函数，错误信息由 `style::suggest` 生成。

| | Lua 引擎 | QuickJS 引擎（当前实现） |
| --- | --- | --- |
| 派发方式 | userdata 的 `__index` 元方法 + 方法缓存 | 原型 `Proxy` 的 `get` 陷阱 → 原型上的普通函数 |
| 拼错 `items_centre` 时 | `unknown element method 'items_centre' (did you mean 'items_center'?)` | 同左，逐字一致 |
| 每次调用的额外成本 | 一次元方法查找（首次）+ 缓存命中 | 一次 `Reflect.get` |

这是一次**明确的取舍**：为了诊断，每次方法调用多付一次属性读取。之所以现在就这么选，是因为拒绝 `class("...")` 字符串写法的全部理由就是"调用点必须能立刻看见拼写错误"（本节开头），如果换成 QuickJS 就把这个能力丢掉，那条论证就不成立了。

**退路写在代码注释里**：若 §20.3 的 M0 基准显示这次 `Reflect.get` 吃掉了预算，就把原型换回普通对象、只在 `--dev` 下包 `Proxy`，或者让失败的一次 render 重跑一遍带 `Proxy` 的版本专门用来生成错误信息。三条都不需要改 API。

另外两条与引擎无关的保障仍然成立：

1. **`.d.ts` + `// @ts-check`**（§14.4）。这是换成 JS 之后**新拿到**的能力，比运行时提示更早：编辑器里就是红波浪线。
2. **绝不静默**：任何到达 `__apply` 却不认识的名字一律抛错，不做无声 no-op。

### 13.3 语义 token 与默认调色板

**关键事实**：`gpui_base::Theme` 的 `ColorTokens` 是 `#[derive(Default)]`，默认值是全零 `Hsla`，即**完全透明**；`RadiusTokens` / `SpacingTokens` 才有真实默认值。只调 `gpui_base::init(cx)` 而不填色，界面一片透明。

因此 shell 承担两件事：

1. 随包发布 `theme/default-tokens.json`（`include_str!` 内嵌，保证二进制自足），启动时反序列化写入 `Theme::global_mut(cx).tokens`。零 schema 成本：`SemanticThemeTokens` 及其子结构已 `#[derive(Serialize, Deserialize, JsonSchema)]`。
2. 向脚本暴露只读 token 与主题切换：

```js
const theme = gpui.theme();
theme.colors.background;   // background / foreground / surface / primary / muted /
                           // accent / destructive / border / input / ring 及其 *_foreground
theme.spacing.md;          // xxs xs sm md lg xl xxl
theme.radius.lg;           // none sm md lg xl full
theme.is_dark;

gpui.set_theme("dark");    // 切换到已注册的 token 集
```

规则：

- 颜色参数**首选 token 名字符串**（`.bg("surface")`）；接受十六进制（`#rgb` / `#rrggbb` / `#rrggbbaa`）是为一次性工具，文档标注它绕过主题。
- 插件用 `gpui.register_theme(id, path)` 注册 token JSON 文件，而不是在脚本里拼颜色。
- 这与 CLAUDE.md 一致：主题 API 暴露语义 token，不暴露不断增长的组件专属字段。

### 13.4 预设样式模块（脚本源码，可替换）

只有 token 还不够：作者不应该每次都从零写按钮样式。Shell 随包发布一个**用 JavaScript 写的**预设模块：

```js
import { button } from "gpui/preset";

button(Button.new("save"), { variant: "primary", size: "sm" }).child(text("Save"));
```

三条纪律：

1. **它是脚本源码**，随包发布在 `crates/shell/js/`，应用可以整体替换或 fork；`examples/js_checklist/counter.js` 里那个十几行的 `button` 辅助函数就是它的雏形；
2. **Rust 侧不得内置任何视觉决策**（§5.6），否则等于在 base 之上又造了一套不受控的视觉系统；
3. 它**不是** `gpui-component` 的复刻，也不承诺与之视觉一致——需要产品视觉的应用等 §14.6。

预设模块要在两个引擎下各有一份（JS 一份、Lua 一份）。这是接缝的真实成本之一：**接缝之上的 Rust 代码只有一份，随包发布的脚本代码有两份**。控制办法只有一个——把预设做薄（§27 第 1 条）。

### 13.5 动画

base 的 `motion`（`Transition`、`Spring`、`Interpolate`）与 `animation` 直接绑定，脚本只描述目标与时序，插值全在 Rust：

```js
div().transition("opacity", { duration: 150, easing: "ease_out" });
```

---

## 14. 绑定注册表与代码生成

### 14.1 规模现实

| 度量 | `gpui-base`（第一期） | `gpui-component`（后续） |
| --- | --- | --- |
| Button 模块 `pub fn` | 13 | 52 |
| 直接依赖数 | 18 | 31 |
| 无参样式方法 | 数百，反射零维护（两层共用） | 同左 |

绑定 base 的表面小一个量级，第一期因此**有可能覆盖完整**——这是 §4.2 第 3 条的量化依据。

### 14.2 分层策略

| Tier | 内容 | 绑定方式 | 稳定性承诺 |
| --- | --- | --- | --- |
| **Tier 1** 核心 | `gpui` 的 `div`/`img`/`svg`/`canvas` + `Styled`；base 的 Button、Checkbox、Radio、Switch、Toggle、Link、Label 语义、Input/Textarea、Select、Combobox、Tabs、Dialog、Sheet、Popover、Tooltip、Scrollbar、Tree、Table、VirtualList、Dock | 手工调优 + 覆盖测试 | 随 API 版本 semver 承诺 |
| **Tier 2** 生成 | base 其余模块：Accordion、Avatar、Calendar、DatePicker、ColorPicker、Slider、Progress、Pagination、OtpInput、NumberInput、ToggleGroup、Resizable、Toast、HoverCard、AlertDialog | 由绑定表生成 | 尽力而为，破坏性变更记 CHANGELOG |
| **Tier 3** 暂不绑定 | `input::Editor` 的 LSP / 折叠 / 诊断 / 高亮接口 | 不绑定；Editor 以"给文本 + 给配置"的窄接口暴露 | 无 |

Tier 3 的理由：这些接口以 Rust trait 与泛型为主（`InputHighlighter`、`CompletionProvider`、`HighlightStyleResolver`），跨语言映射成本与失真都高，而它们恰恰是"该留在 Rust"的部分（§3.3 第 1 条）。

M0 实际绑定的只有：`div` / `h_flex` / `v_flex` / `text` 与 `Button` / `Checkbox` / `Switch`，加 `child` / `children` / `when` / `on_click` / `on_change` / `disabled` / `selected` / `checked`，以及全部无参样式与 57 个有参样式。这是 Tier 1 的一个很小的前缀，足以回答 §20 的问题，不足以写应用。

### 14.3 绑定表形态

绑定以**数据**形式声明在 `crates/shell/src/bindings/`，由宏展开为注册代码：

```rust,ignore
binding! {
    component gpui_base::Button as "Button" {
        new(id: ElementId);
        method disabled(flag: bool);
        method selected(flag: bool);
        state_style checked | selected | disabled;
        event on_click(ClickEvent);
        styled;                                  // 接入反射样式表
        children;                                // 实现 ParentElement
    }
}
```

它同时是三样东西的唯一事实来源：运行时注册表、类型声明、文档表格。**绑定表在接缝之上**：它描述的是"哪个组件有哪些方法"，与脚本值长什么样无关；每个引擎从同一张表生成自己的注册代码（QuickJS 是原型上的函数名列表，Lua 是 `is_known_method` 的判定）。

### 14.4 类型声明生成：`.d.ts`

从绑定表与样式反射表生成 TypeScript 声明：

```ts
declare module "gpui" {
  export class View {
    init(props?: Record<string, unknown>): void;
    render(cx: CallContext): Element;
  }

  export interface Element {
    child(element: Element): this;
    children(list: Element[]): this;
    when(condition: boolean, branch: (el: this) => Element): Element;
    /** 无参样式，来自 GPUI 反射表 */
    items_center(): this;
    size_full(): this;
    gap_2(): this;
    /** 有参样式 */
    bg(color: string): this;
    p(pixels: number): this;
  }

  export const Button: { new(id: string): ButtonElement };
  export interface ButtonElement extends Element {
    disabled(flag: boolean): this;
    selected(flag: boolean): this;
    on_click(handler: (event: ClickEvent, cx: CallContext) => void): this;
  }
}
```

**这是换成 JavaScript 之后收益最直接的一处。** 相对 v0.3 的 LuaCATS：

- 应用**仍然是纯 JS，没有编译步骤**（§3.3 第 2 条）；`.d.ts` 只是给编辑器与 `// @ts-check` 看的旁注。
- `tsserver` 是现成的、每个编辑器都有的检查器，能抓住 §13.2 那类拼写错误、参数类型错误与"把 cx 存起来"这类误用（把 `CallContext` 标成不可存的品牌类型即可）。
- 对使用者 C（AI 生成）同样关键：`.d.ts` 是模型最熟悉的 API 契约格式，可以整份喂进上下文。

Lua 引擎那一侧继续生成 LuaCATS，同源同表。

### 14.5 覆盖率与漂移检测

CI 增加一步：以 `cargo rustdoc --output-format json` 取 `crates/base` 的公共 API，与绑定表比对：

- 未绑定的公共方法清单（信息，不失败）；
- **绑定表引用了已不存在的方法**（失败）；
- **签名不匹配**（失败）。

把"绑定与上游漂移"从运行时问题变成构建期问题。

### 14.6 后续：`import ... from "gpui-component"`

第二期把 `gpui-component` 作为**第二个绑定注册表**接入，与第一期共用同一套渲染协议、CallScope、事件模型与 arena：

```js
import { v_flex, text } from "gpui";                 // base + gpui：呈现权在脚本
import { Button } from "gpui-component";             // 成品视觉：呈现权在组件库

Button.new("save").primary().label("Save");          // 一行即成品外观
```

设计要点：

1. **同一套协议，两个注册表。** `Component` 命名空间隔离，`SpecArena` / `materialize` 不变。这正是第一期把渲染协议与组件绑定彻底分离的回报。
2. **crate 依赖可选。** `gpui-component` 绑定放在 `crates/shell` 的 `component` feature 之后（或独立 crate `gpui-shell-component`），不开就完全不进依赖树。
3. **命名现在就要定。** 第一期 `gpui` 的 `Button` 与第二期 `gpui-component` 的 `Button` 方法名会有重叠但语义不同（后者带 variant/size/图标等成品参数）。两个模块名从一开始就分开，避免同名不同义——在 JS 里这一点尤其重要，因为两者可以在同一个文件里同时 import，必须靠模块名而不是靠约定区分。
4. **迁移是替换 import，不是重写。** 一个应用从"自绘视觉"切到"产品视觉"，改动集中在构建界面的那几个函数，业务逻辑与状态不动。
5. **`Root` 与 `ShellRoot` 的关系。** 接入 `gpui-component` 后，`ShellRoot` 可以选择委托给 `Root`（复用其 dialog / sheet / notification 栈），也可以继续自持。这个决定留到第二期，届时已有实际使用数据。

---

## 15. Dock 与 Panel 集成

这是让插件真正"长在宿主里"的一章，也是 base 现有基础设施最能直接复用的一处。**整章与脚本语言无关**：三个 renderer trait 与 `Panel` 适配器都在接缝之上，穿过接缝的只有"调用脚本的这个函数"。

### 15.1 Dock 外观由脚本决定

base 的 `DockArea` 不带渲染器时"能拖能停靠能持久化，但不画任何东西"。shell 实现三个 renderer trait，把外观全部转发给脚本：

```rust,ignore
pub struct ScriptDockSkin { runtime: Rc<ShellRuntime>, handlers: DockHandlers }

impl TabGroupRenderer for ScriptDockSkin {
    fn render_tab_bar(&self, context: &TabGroupContext, window: &mut Window, cx: &mut App) -> AnyElement {
        // 嵌套 CallScope，phase = Layout（§8.5）
        self.runtime.call_render(self.handlers.tab_bar, context, window, cx)
    }
}
// DockAreaRenderer / TilesRenderer 同理
```

```js
dock.on_render_tab_bar((context, cx) =>
  h_flex()
    .h(32)
    .bg("surface")
    .children(
      context.tabs.map((tab) =>
        div()
          .px(10)
          .when(tab.active, (el) => el.bg("background"))
          .child(text(tab.title)),
      ),
    ),
);
```

`context.tabs.map(...)` 是 JS 侧写起来最自然的形态，也说明了为什么 `children` 接一个数组而不是接"列表 + 回调"：数组方法是 JS 作者的默认工具，多一层回调只是噪音。Lua 引擎那边保留 `children(list, fn)` 的形态，因为 Lua 没有等价的数组方法——这是允许两个引擎"行为一致、写法不同"的一个例子（§6.3）。

base 负责拖拽源、放置目标命中测试、键盘动作与焦点；脚本只拿到解析后的状态（`TabGroupContext` / `DockContext` / `TileContext`），永远看不到拖拽事件本身。

### 15.2 ScriptPanel 适配器

```rust,ignore
pub struct ScriptPanel {
    view: Entity<ScriptView>,
    name: &'static str,          // intern 后的 "script:myplugin/inbox"
}

impl gpui_base::dock::Panel for ScriptPanel {
    fn panel_name(&self) -> &'static str { self.name }
    fn visible(&self, cx: &App) -> bool { /* 脚本可选钩子，默认 true */ }
    fn dump(&self, cx: &App) -> PanelState { /* 脚本 serialize() → PanelInfo::panel(json) */ }
    // closable / zoomable / set_active / set_zoomed / on_added_to / on_removed → 脚本可选钩子
}
```

第一期只实现 `gpui_base::dock::Panel`（行为）。标题、工具条、下拉菜单等呈现不走 `gpui_component::dock::Panel`，而是通过 §15.1 的 renderer 由脚本直接画——这与"呈现权在脚本"一致，也避免第一期引入 ui 依赖。

### 15.3 持久化往返

`PanelRegistry::register_panel("script:myplugin/inbox", builder)` 在插件激活时注册。`DockArea::load` 恢复布局时按名字回调 builder，`PanelBuildContext::info()` 里的 `serde_json::Value` 转成脚本对象交给 `deserialize(data)`。

JSON ↔ 脚本对象这一步在 JS 侧几乎没有阻抗：`serde_json::Value` 与 JS 对象是同一套形状（Lua 侧则要处理"空表既是数组又是字典"的歧义）。这是换语言之后少数几处实现变简单的地方，值得记一笔。

关键的既有保障：**插件未安装时布局不会被破坏**。registry 找不到名字时 `DockArea` 会放一个 draw-nothing 占位并保留原始 `PanelState`，下次保存仍写回去。用户卸载再装回一个插件，它的面板会回到原位。

### 15.4 `&'static str` 约束

`Panel::panel_name(&self) -> &'static str` 要求静态字符串，而插件名运行时才知道。方案：进程内 intern 表，首次注册 `Box::leak` 一次。

- 上界：已加载插件数 × 每插件面板数，量级在百级，可接受；
- 卸载不回收该字符串（几十字节），文档写明；
- 强制前缀 `script:<plugin_id>/<panel_id>`，避免与宿主内建 panel 冲突。前缀用 `script:` 而不是语言名，因为同一份布局文件在切换引擎后仍应能恢复。

### 15.5 脚本侧的 panel

```js
import { Panel } from "gpui";

export default class Inbox extends Panel {
  static id = "inbox";
  static options = { title: "Inbox", closable: true, zoomable: true };

  init() {
    this.filter = "all";
  }

  render(cx) { /* ... */ }

  serialize() {
    return { filter: this.filter };
  }

  deserialize(data) {
    this.filter = data.filter;
  }
}
```

`Panel` 与 `View` 同源：一个有额外的静态元数据与序列化钩子，其余完全一样。

---

## 16. ShellRoot：窗口与覆盖层

### 16.1 base 没有 Root

`Root` 属于 `gpui-component`。base 提供的是构件：`Dialog` / `Sheet` 自带 viewport 级 host，`ToastManager` / `ToastStackState` 提供堆叠几何，`FocusTrapElement` 提供焦点陷阱，`Popup` / `Positioner` 提供定位与碰撞。缺的是把它们组织成窗口级覆盖层栈的那一层。

`ShellRoot` 是 `crates/shell` 的一个 Rust 视图，职责与 `Root` 对等但不复用其代码：

- dialog 栈（打开顺序、关闭顺序、Escape 处理）；
- sheet 宿主；
- toast 栈；
- 焦点与 Tab / Shift-Tab 导航；
- 顶层键盘动作分发。

窗口的第一级视图始终是 `ShellRoot`，脚本不能绕过——与 `gpui-component` 中 `Root` 不可绕过的约定同构。

### 16.2 API

```js
gpui.open_window(
  {
    title: "Inbox",
    width: 1024,
    height: 720,
    decorations: "client",     // 无内建 TitleBar，标题栏由脚本自绘
  },
  () => Inbox.new({}),
);

cx.open_dialog((cx) => ConfirmView.new({ /* ... */ }));
cx.close_dialog();
cx.open_sheet("right", (cx) => { /* ... */ });
cx.toast({ title: "Saved", level: "success" });
```

这些方法在 Rust 侧都需要 `&mut Window`，因此只在 phase = `Event` / `Task` 的 `CallScope` 中可用（§9.3）。

M0 里窗口由宿主打开：运行时加载 `main.js`，取它 `export default` 的 view class，构造一个实例挂成根视图；`gpui.open_window` 属于 M2（§26）。

### 16.3 标题栏与窗口装饰

第一期没有 `TitleBar` 与 `window_border` 组件。脚本用 GPUI 的窗口选项 + 自绘标题栏；shell 提供拖动区域、双击最大化、窗口按钮等**行为**绑定，外观由脚本决定。

---

## 17. 系统能力 API

所有能力默认关闭，需 manifest 声明并获授权（§19.2）。能力的判定与路径解析在接缝之上（`capability.rs`），引擎里只有一层参数搬运（§6.5 的规则 1）。

```js
const { fs, http, store, clipboard, log } = gpui;
```

### 17.1 文件系统

```js
const text = await fs.read_text("data/items.json");
await fs.write_text("data/items.json", body);
const entries = await fs.read_dir("data");
```

- 路径相对插件根目录解析；越界（`..`、绝对路径、符号链接指向授权范围外）直接报错。已实现：`Capabilities::resolve` 做词法归一化并要求结果落在授权根内，符号链接在系统调用处再查一次。
- 额外根目录须由用户通过系统文件选择器授予，授权结果持久化在宿主而非插件目录。
- 全部异步：同步 IO 会卡住渲染线程，因此**不提供**同步版本。JS 的 `await` 让这一条几乎没有代价——v0.3 里它需要 coroutine 才成立。

### 17.2 网络

```js
const a = await http.get(url, { headers: { accept: "application/json" } });
const b = await http.post(url, { json: payload, timeout: 5000 });
```

- 域名 allowlist 来自 manifest；默认超时、大小上限、重定向上限均可由宿主策略收紧；
- HTTP 客户端由**宿主注入**：`crates/shell` 不硬绑某个实现。桌面宿主可传入 zed 的 `reqwest_client`（`crates/story` 已在用），wasm 宿主传入 fetch 适配器。这保持了 §4.2 第 4 条的依赖克制。
- **不叫 `fetch`，也不模仿它的签名。** 名字一样但语义只有八成像，是比换个名字更糟的结果：模型会按 `fetch` 的完整语义生成代码（`Response.json()`、流式 body、`AbortController`），然后在第三行失败。

### 17.3 存储

```js
store.set("theme", "dark");
const v = store.get("theme");
await store.flush();
store.on_change((key, value) => { /* ... */ });
```

- 每插件独立命名空间，落盘 JSON，原子写（临时文件 + rename）；
- 只适合配置与小状态。大数据（索引、缓存、全文检索）应由宿主 native module 提供。

### 17.4 剪贴板与日志

```js
clipboard.write_text(s);
const s = clipboard.read_text();
log.info("loaded %d items", n);        // 进 tracing，带插件 id 字段
```

不提供 `console.log`？**提供**，但它就是 `log.info` 的别名，输出进 tracing 并带上插件 id。理由与 §12.5 的 `setTimeout` 相反：`console.log` 的语义足够简单，一比一映射没有失真风险，而它是 JS 作者与模型的第一反应——让它可用比让它报错更划算。`console.error` / `console.warn` 同理，`console` 的其余成员（`table`、`time`、`group`…）不提供。

第一期不绑 `rust-i18n`（它是 `crates/ui` 的依赖）。插件的多语言用普通对象 + `gpui.locale()` 自行实现；若后续接入 `gpui-component`，再复用其 i18n。

### 17.5 进程与外部命令

v0.3 在这里的决定是：**文件与进程操作保留可用，但必须先声明 `fs` 能力**。这条决定在 v0.4 原样保留，只是落到了不同的 API 表面上。

变化的是机制而不是政策。Lua 版把 `os.execute` / `os.remove` / `os.rename` / `os.exit` **保留原名**再加能力检查，理由是"让既有 Lua 代码与模型生成的代码能直接跑"。JavaScript 里没有对应的语言内建 API——文件与进程在 JS 里从来都是宿主给的（Node 的 `fs`、`child_process`），而本运行时明确不做 Node 兼容层（§3.3 第 7 条）。那条理由因此消失了，入口收敛成一个：

```js
const { status, stdout, stderr } = await gpui.process.run("git", ["status", "--short"], {
  cwd: "repo",
  timeout: 5000,
});

await gpui.fs.remove("cache/index.json");
await gpui.fs.rename("a.json", "b.json");
gpui.process.exit();
```

对应的 manifest 声明不变：

```json
"capabilities": {
  "fs": {
    "read": ["${pluginDir}", "${dataDir}"],
    "write": ["${dataDir}"],
    "execute": ["git", "rg"]
  }
}
```

- `execute` 取值是**命令名白名单**；`true` 表示不限命令，安装时的能力清单会把它显示为最高等级的一条，与"完全的文件读写"同级。
- 未声明 `execute` 而调用 `gpui.process.run` 立即报错，错误信息直接给出需要在 manifest 里加什么。已实现的措辞：`running \`curl\` is not granted; add it to capabilities.fs.execute in the manifest`。
- `gpui.fs.remove` / `rename` 需要 `fs.write`，路径经过与其他 `fs` 方法**完全相同**的解析与越界检查——同一套路径规则，一个入口，不存在后门。
- **不提供流式子进程**（Lua 版删掉 `io.popen` 的同一个理由）：管道语义与 §12 的异步模型冲突。需要流式输出的场景走 native module（§17.6），那里可以给出结构化结果与超时。
- `gpui.process.exit()` 在声明 `fs` 后可用，但实现是**向宿主发出退出请求**，而不是直接 `exit(2)`：宿主可以先落盘、可以拒绝（嵌入式宿主里一个插件不该有权杀掉用户正在编辑的应用），也可以按策略直接退出。这是本节唯一一处对原语义的收窄。

### 17.6 原生扩展（v0.1 §13 的修正）

不做 `dlopen`。原生扩展的形态是**宿主在编译期注册 Rust 模块**：

```rust,ignore
shell.register_native_module("html", |registry| {
    registry.function("parse", |input: String| -> Result<HtmlTree> { ... });
});
```

```js
const html = gpui.native("html");
const tree = html.parse(content);
```

理由：Rust 无稳定 ABI；`dlopen` 的原生代码在同进程内拥有全部权限，沙箱无从谈起；而这条路径覆盖了 v0.1 举的全部例子（HTML 解析、压缩、加密、数据库），只是把"谁能加载"从插件作者移回宿主作者。第三方确需原生能力时走 fork 宿主或提 PR ——这个成本是**有意**保留的。

native module 的注册表在接缝之上；函数的参数与返回值必须能表达成 `Bridged` 或 JSON，这样两个引擎共用同一份注册代码。

---

## 18. 插件模型

### 18.1 manifest

manifest 只回答两个问题：**这是谁**，**它要什么权限**。

```json
{
  "id": "com.example.inbox",
  "name": "Inbox",
  "version": "1.2.0",
  "entry": "main.js",
  "capabilities": {
    "fs": { "read": ["${pluginDir}", "${dataDir}"], "write": ["${dataDir}"], "execute": ["git"] },
    "network": { "hosts": ["api.example.com"] },
    "store": true,
    "clipboard": { "write": true }
  }
}
```

五个字段，没有第六个。命令、面板、快捷键、设置项、主题**一律在脚本里注册**，不在 manifest 里再声明一遍：

```js
import { gpui } from "gpui";

gpui.require_api("1.0");                 // API 版本要求，不匹配立即报错退出

gpui.command("inbox.open", "Open Inbox", async (cx) => {
  const { open } = await import("./inbox/window.js");   // 真正的实现在这里才被加载
  open(cx);
});

gpui.keymap({ "cmd-shift-i": "inbox.open" });
gpui.register_panel(Inbox);
gpui.register_theme("inbox-dark", "themes/dark.json");
```

这样定的三个理由：

1. **不做双份声明。** manifest 里的 `contributes` 与脚本里的注册代码必然要对齐，一旦不一致就是一类专属 bug，而对齐本身没有产生任何信息。
2. **能力是权限，贡献是行为。** 前者需要在跑任何代码之前被用户看见并批准，所以必须留在 manifest；后者是代码的一部分，放在代码里。
3. **schema 小到可以手写。** 用 `schemars` 生成 JSON Schema 供编辑器校验仍然值得（`crates/ui/src/theme/schema.rs` 是现成先例），但一个五字段的 schema 谁都能读。

`id` 同时是命名空间前缀：面板名是 `script:<id>/<panel>`（§15.4），store 命名空间、日志字段、能力授权记录都用它。

注意上面用的是**动态 `import()`** 而不是同步加载：这正是 §18.2 的模块级懒加载在 JS 里的自然形态，也是 §19.1 必须把动态 `import()` 纳入模块解析器管辖的原因——它是懒加载的手段，不是要封死的洞。

### 18.2 生命周期

```text
discover → parse manifest（5 个字段）→ 能力检查与授权
   → 加载 main.js（进程启动时；只允许注册，不允许干活）
   → 命令/面板/快捷键被触发 → 处理器内动态 import 真正的实现模块
   → ...
   → deactivate() → 释放长期回调/任务/面板 → 丢弃模块
```

去掉 `activation` 之后，**懒加载从"插件级"下沉到"模块级"**：`main.js` 在启动时执行，但它的唯一职责是注册；真正的实现放在别的模块，由处理器在被触发时 `import()`。这正是参考项目 Neovim 的做法，也与 VS Code 扩展"顶层只 `activate`、重活按需 require"的实践一致。

代价与约束写清楚：

- 启动成本从"≈ 0"变成"每个插件一次 `main.js`"。§20.5 因此给出预算：单个 `main.js` **< 3 ms**，超出的插件在 DevTools 里被点名。
- `main.js` 里做重活（读文件、建窗口、拉网络）是**约定违规**，运行时会警告；这是文档与 lint 能管住的事，不值得为它引入一套 activation 事件的 DSL。
- 顶层 `await` 不可用（§12.2 第 5 条），这恰好也让"main.js 里做重活"更难写出来。
- 宿主仍可整体延迟：`ShellOptions` 提供"首次进入某个窗口后再加载插件"的开关，粒度是全体插件，不是逐个事件。

### 18.3 宿主嵌入 API

```rust,ignore
let shell = ShellRuntime::new(
    ShellOptions::default()
        .with_plugin_dirs(vec![user_plugins_dir()])
        .with_tokens(my_design_tokens())     // 宿主可整体替换默认调色板
        .with_http_client(reqwest_client.clone())
        .with_capability_policy(policy)
        .with_native_modules(modules),
    cx,
);
shell.load_installed(cx)?;
```

`ShellOptions` / `CapabilityPolicy` 等跨 seam 的公共类型遵循 CLAUDE.md 第 7 条：私有字段 + 读方法；构造从 `default()` 起手，再用链式设置逐项覆盖，因此新增一个选项不会破坏任何既有调用点。

**引擎不是 `ShellOptions` 的一个字段，而是编译期 feature。** 两个引擎的句柄类型不同（`Persistent<Object>` vs `mlua::Table`），做成运行时选项就要把它们塞进一个 trait object，接缝的复杂度会从 cfg 搬进类型系统而不会消失。宿主要换引擎，改的是 `Cargo.toml`（§6.5）。

---

## 19. 沙箱与安全

本章按 QuickJS 写。Lua 引擎那一侧的标准库裁剪（`ffi`、`io.*`、`package.loadlib`、`debug.*`）与本章不是同一张表——**语言不同，攻击面就不同**，所以这部分按 §6.5 的规则 3 落在接缝之下，各写一份。但**能力判定与路径解析只有一份**（`capability.rs`），两个引擎共用，因此不存在"某个引擎的沙箱松一点"的余地。

### 19.1 语言面的裁剪

JS 的好处是它的标准库里没有 IO：`eval` 之外，语言本身碰不到文件、进程或网络。危险面因此集中在四处——宿主注入了什么、`eval` 一类的动态代码、模块解析、以及共享的内建原型。

| 处理 | 目标 | 说明 |
| --- | --- | --- |
| **从不添加** | quickjs-libc 的 `std` / `os` 模块 | **最关键的一条**：它们提供 `open`、`exec`、`getenv`、`popen`，一旦注册就是全权限。`rquickjs` 不会自动注入，shell 也绝不注册。这是"从不添加"而不是"移除"，比事后删除可靠一个量级 |
| **从不添加** | `Eval` intrinsic（`eval`、`new Function`） | QuickJS 把 eval 做成**可选 intrinsic**：用 `Context::custom` 组装上下文时不加 `intrinsic::Eval`，`eval` 与 `Function` 构造器就根本不存在。这比"删掉 `globalThis.eval`"强得多——后者绕不过 `(function(){}).constructor`。**M0 目前用的是 `Context::full`（全部 intrinsic），这是已知待办**，必须在 M4 之前改成显式组装 |
| **替换** | 模块解析器（静态 `import` 与动态 `import()` 走同一个） | 只解析两类名字：内建模块（`gpui`，后续 `gpui/preset`、`gpui-component`），以及应用/插件目录内的相对路径。解析结果落在根目录之外一律拒绝，符号链接在打开文件时再查一次。动态 `import()` **不封**——它是 §18.2 模块级懒加载的手段，封了等于没有懒加载 |
| **冻结** | 内建原型（`Object.prototype`、`Array.prototype`、`Function.prototype`、`String.prototype`…） | 单 VM 多插件（§27 第 4 条）意味着内建原型是共享可变状态：一个插件给 `Array.prototype` 挂个属性，其余插件与 shell 自己的 prelude 全部受影响，`for...in` 的行为都会变。prelude 装完后统一 `Object.freeze` 一遍，并且 shell 的 prelude 自身不依赖任何可被改写的内建方法 |
| **需能力** | `gpui.fs.*` | 声明 `capabilities.fs.read` / `fs.write` 后可用；路径全部经 `Capabilities::resolve`（§17.1） |
| **需能力** | `gpui.process.run` | 声明 `capabilities.fs.execute` 的命令白名单后可用（§17.5） |
| **需能力** | `gpui.process.exit` | 声明 `fs` 后可用；实现为向宿主发出退出请求而非直接 `exit(2)`（§17.5） |
| **报错桩** | `setTimeout`、`setInterval`、`fetch`、`require`、`process`、`document`、`window`、`localStorage` | 存在但一调用就抛，错误信息指向替代品（`gpui.timer` / `gpui.http` / `import` / `gpui.process` / 无对应物）。理由见 §2.1：模型一定会写它们，有名字而报错好过 `ReferenceError` |

`console.log` / `warn` / `error` 是例外，映射到 `gpui.log`（§17.4）：语义足够简单，一比一映射没有失真。

**实现落地后的三条实测结论**（都由 `engine/quickjs/sandbox.rs` 的逃逸测试给出，不是推断）：

1. **中断不可被脚本吞掉。** `try { while (true) {} } catch (e) {}` 依然会被中断终止。这回答了 §19.3 原本标记为"待验证"的问题：中断是真实防线，策略不必升级到"丢弃整个 context"。
2. **quickjs-libc 的 `std` / `os` 根本没有被编译进来**（`rquickjs-sys` 的 `build.rs` 只编译 `libregexp.c`、`libunicode.c`、`quickjs.c`、`dtoa.c`），因此不需要"移除"，只需要一条回归测试断言它们不存在。
3. **不装 `Eval` intrinsic 不是即插即用的。** `Ctx::eval` 本身就是 `JS_Eval`，受同一个 intrinsic 管辖——去掉它，宿主自己的 `ctx.eval` 也一起没了，而 `mod.rs` 的 JS prelude 正是用它装载的。所以当前在起作用的是"删掉 `globalThis.eval`、把四个函数构造器（`Function`、`AsyncFunction`、`GeneratorFunction`、`AsyncGeneratorFunction`）换成抛错桩"这一层；要走到 intrinsic 级别，得先把 prelude 改成 `Module::evaluate` 或预编译字节码。

模块解析器这一项**已经落地**：`engine/quickjs/mod.rs` 里的 `AppModules` 同时实现 `Resolver` 与 `Loader`，把静态与动态 `import` 都限制在 `canonicalize` 之后的应用根目录内（rquickjs 自带的 `FileResolver` 不可用——它按进程工作目录判断候选文件是否存在，绝对路径永远匹配不上）。其余三项仍待实现。

裁剪与替换必须在**创建 context 之后、加载任何应用代码之前**完成，并有针对性的逃逸测试（§22.2）。`--dev` 模式可以把 `Eval` intrinsic 加回来（DevTools 的 REPL 需要），此时 UI 上持续显示开发模式标记（§19.4）。

### 19.2 能力授权

三态：`granted` / `denied` / `prompt`。首次触发时由宿主弹出授权 UI，结果持久化在宿主配置（不在插件目录，插件不能自己改）。宿主可通过 `CapabilityPolicy` 强制策略（例如企业部署下全部 `denied`）。

`Capabilities` 的默认值是空集，且每个字段私有、只能通过 `with_*` 构造（`crates/shell/src/capability.rs`）——"默认无能力"（§5.7）因此是类型层面的事实，不是文档里的承诺。

### 19.3 资源限额

| 限额 | 手段 |
| --- | --- |
| 死循环 | `rquickjs::Runtime::set_interrupt_handler`：按时间片或计数中断；渲染路径阈值更严（一次 render 超预算即中断并报错） |
| 内存 | `Runtime::set_memory_limit` + `set_max_stack_size`，超限抛异常而非 OOM |
| 回调风暴 | 单帧 `notify` 合并；单帧脚本调用次数上限（超限记警告） |
| 未完成的 microtask | 单次泵 job queue 的轮数上限，防止 `Promise.resolve().then(loop)` 这类自我复制的 microtask 饿死主线程（§12.2） |
| 磁盘 | store 单插件配额 |

**一处必须实测的假设**：QuickJS 的中断以抛异常的形式呈现，而 JS 的 `try { } catch { }` 能捕获它。若脚本能把中断异常吞掉继续跑，那么"中断"就不是一道防线，届时的处理必须升级为**丢弃该插件的整个执行上下文**而不是抛一个可捕获的异常。这一条列进 §22.2 的逃逸测试，M4 前必须有结论——不要按"应该没问题"实现。

中断一个失控插件的表现应当是：该插件的面板显示错误覆盖层，宿主其余部分正常工作。

### 19.4 信任模型

- 本地开发模式（`--dev`）跳过签名校验并可开启 `Eval` intrinsic，但 UI 上持续显示"开发模式"标记；
- 分发包（§23）签名校验；
- 首次安装展示能力清单，与 manifest 一致，用户确认；
- 插件更新若**新增**能力，重新征求授权。

---

## 20. 性能模型

**本章与脚本语言无关，但对引擎极其敏感**——它正是 §6.5 那条接缝存在的理由。

### 20.1 渲染何时发生

关键事实：GPUI **不是**每帧调用 `Render::render`。视图在被 `notify`、依赖实体变化或窗口失效时才重建元素树。因此脚本的成本与"帧率"无关，与"交互频率"有关。

需要特别关注**连续交互**：拖拽、滚动、输入、动画会以接近帧率的速度触发 notify。

### 20.2 成本模型

```text
T_render ≈ N_nodes × (C_new + K_ops × C_op) + N_nodes × C_materialize + C_scope
```

`C_op` 是一次脚本 → Rust 的方法调用（含参数转换），`C_materialize` 是纯 Rust 的元素构造。

QuickJS 引擎下，一次 `C_op` 的实际组成是：

1. 原型上的属性查找（普通查找，不是 proxy trap——§6.3 就是为这一条选的原型派发）；
2. 一次 JS 函数调用（prelude 生成的三行小函数）；
3. **一次 rest 参数数组分配**（`function (...args)`）；
4. 一次进 Rust 的 host call（`__apply`）；
5. `Value` → `Bridged` 的转换与一次 `SmallVec` 推入。

第 3 项是 JS 特有的、v0.3 的成本模型里没有的一笔：`...args` 每次调用都要新建一个数组。无参样式方法（最常见的一类）付了这笔钱却什么都没用上。**最直接的优化因此是给零参与单参两种情形装特化函数**（`function () { __apply0(this.__id, name); return this; }`），M0 的基准要把这两种形态分别测出来。

**base-first 的一个额外成本必须计入**：呈现权在脚本意味着每个节点的 op 数高于绑 gpui-component 的方案（后者一个 `.primary()` 顶前者五六个样式调用）。§20.3 的预算已按此放大。由于样式只有 fluent 一种表达（§13.2），不存在"一次调用套用多个样式"的批量口子，对冲手段只剩三条：压低 `C_op` 本身、`gpui.memo`、虚拟化（§20.4）。

### 20.3 预算（待 M0 实测验证）

| 指标 | 目标 | 说明 |
| --- | --- | --- |
| 120Hz 帧预算 | 8.3 ms | 全部工作，含布局与绘制 |
| 连续交互下脚本渲染预算 | **< 1.5 ms** | 拖拽/滚动/输入时的重建 |
| 典型面板节点数 | 200 – 800 | 虚拟化后可见项 |
| 每节点 op 数（base-first） | 6 – 12 | 样式 + 状态样式 + 事件 |
| 由此推出的 `C_op` 上限 | **≈ 150 ns** | 800 × 12 × 150ns ≈ 1.44 ms |

`C_op` 能否落在这个量级，是 M0 必须回答的问题（§26）。**v0.4 的变化是：这个数要在两个引擎下各测一遍。** 两份数据本身就是接缝的价值——它把"换引擎"从一次赌博变成一个有测量依据的决定。

若 QuickJS 不达标，退路依次是：压低 `C_op`（特化调用形态、参数转换）→ `gpui.memo`（§8.6）→ 更细粒度的 view 划分与局部 notify → **切换到 Lua 引擎**（R12）→ 该界面留在 Rust。倒数第二条是 v0.3 没有的。

### 20.4 优化手段清单

1. **虚拟化留在 Rust**：`VirtualList` / `Tree` / `Table` 只对可见项回调脚本，1 万行列表的脚本成本与 100 行相同。
2. **压低 `C_op` 本身**：无参样式方法在 Rust 侧只是往 `SmallVec` 推一个 `u16`，真正开销在 JS 侧的调用形态与参数转换——特化零参/单参调用、避免 rest 数组、`__apply` 的入口零分配，是这里最值钱的优化。**QuickJS 没有 JIT，这一条完全靠人做**：在 LuaJIT 上还可以指望 trace 把一部分派发消掉，这里不能。
3. **`gpui.memo`**：数据未变的子树跳过脚本构建。没有 JIT 也让这条的相对收益更高。
4. **静态子树提升**：与状态无关的子树用 `gpui.memo` 固定住，避免每帧重建。
5. **参数对象复用**：item renderer 与 dock renderer 的上下文对象预分配复用。
6. **绝不让脚本参与**：布局计算、文本 shaping、滚动偏移、动画插值、命中测试。

### 20.5 启动成本

| 项 | 预算 |
| --- | --- |
| VM 创建 + intrinsic 组装 + 沙箱裁剪 | < 2 ms |
| 全局 API 注册（含反射样式表构建，`OnceLock` 缓存） | < 5 ms（一次性） |
| **prelude 执行：按样式名表生成原型上的数百个函数** | 计入上一行，M0 单独测量 |
| 默认 token 加载 | < 1 ms |
| 每个已安装插件的 `main.js`（只注册，不干活） | < 3 ms |
| 一个插件激活（加载 + activate） | < 20 ms |

prelude 那一行是 v0.4 新增的：数百个 JS 闭包的创建是一次性成本，但它发生在启动路径上，必须有数字而不是估计。如果它超过预算，替代形态是把原型缓存成 QuickJS 字节码，或改为按需 define（首次调用某个样式名时才生成）——后者会把成本挪回 `C_op`，因此只有在实测支持时才做。

---

## 21. 错误、调试与热重载

### 21.1 错误必须可恢复

- 所有 Rust → 脚本的调用都在边界处捕获异常，并带上脚本自己的 stack。已实现：QuickJS 引擎的 `describe()` 把 `Exception` 展平成 `message + stack` 的字符串，Lua 引擎做同样的事（`traceback`）；两者最终都变成一个普通的 `anyhow::Error`，因为上层不该认识任何一种 VM 的错误类型。
- Rust panic 不允许穿过 FFI 边界（绑定层安全封装 + 边界处 `catch_unwind`）。
- 渲染期出错：该视图渲染为**错误覆盖层**（错误信息 + stack + "重载"按钮），宿主其余部分不受影响。这一段在接缝之上（`runtime.rs::error_overlay`），两个引擎共用同一个失败界面。
- 事件期出错：toast 通知 + 日志，状态保持原样。
- **未处理的 Promise rejection 按事件期错误处理**（§12.2 第 4 条）。这是 JS 特有的一条：不接钩子的话它是完全静默的。
- 错误信息必须包含插件 id、源文件与行号。**因为没有编译步骤（§3.3 第 2 条），行号就是源码行号**——不需要 source map，这是拒绝 JSX/TS 编译换来的一个实在好处。

### 21.2 热重载

```text
文件监听（宿主注入 watcher，shell 不直接依赖 notify crate）
  → 变更去抖 200ms
  → deactivate 插件（释放回调/任务/面板）
  → 丢弃模块，重新 activate
  → 恢复面板与（可选的）序列化状态
```

状态保留默认走 `serialize()` / `deserialize()` 往返，与布局持久化复用同一条路径（§15.3）。既省一套机制，也顺带持续测试序列化的正确性。

一处 JS 特有的实现约束：ES 模块一旦被求值就进了模块注册表，**没有语言级的卸载**。热重载因此不是"丢弃某个模块"，而是**丢弃整个 context 并重建**（原型、prelude、全局一起重来），再按序列化状态恢复。这比 Lua 的 `package.loaded[name] = nil` 粗一档，好处是不会留下半新半旧的模块图。

### 21.3 DevTools

M2 起提供一个用脚本自身写的调试面板（自身即最好的 dogfood）：VM 内存、存活 view、持久句柄数、上一帧节点数与耗时、样式表命中、错误历史、REPL（REPL 需要 `--dev` 下的 `Eval` intrinsic，§19.1）。

---

## 22. 测试策略

### 22.1 Spec 快照测试（无 GPU）

脚本产出的是**纯数据的 SpecArena**，可以在没有窗口、没有 GPU 的情况下断言界面结构：

```rust,ignore
let tree = runtime.render_to_spec(&object, None, window, cx)?;
assert_snapshot!(tree);
```

`debug_tree` 的输出形如：

```text
v_flex .size_full .items_center .gap_2 .p[Number(16.0)] .bg[Str("background")]
  text "Count: 0" .text_color[Str("foreground")]
  Button "increment" .px[Number(12.0)] :on_click(fn)
    text "Increment"
```

这是 §8 选择方案 B 的额外收益，也是脚本层最主要的回归防线，同时是 §22.3 跨引擎比对的载体。

### 22.2 沙箱逃逸测试

一组必须失败的脚本：

- `import { open } from "std"` / `import * as os from "os"`（模块不存在）；
- `eval("1+1")`、`new Function("return 1")`（intrinsic 未装）；
- `import("/etc/passwd")`、`import("../../secret.js")`（解析器越界）；
- `Array.prototype.push = ...`（原型已冻结）；
- 未声明 `fs.execute` 时调用 `gpui.process.run`；未声明 `fs.write` 时调用 `gpui.fs.remove`；声明了 `fs.write` 但 `gpui.fs.remove("../../secret")` 越界；`gpui.process.run("curl", ...)` 不在白名单内；
- 越权域名的 `gpui.http.get`；
- **`try { while (true) {} } catch {}` 能否吞掉中断异常**（§19.3 的待验证项）。

这类测试是安全断言，不适用"避免琐碎测试"的豁免。

### 22.3 共享行为套件（跨引擎）

v0.3 这一条是"同一批脚本在 LuaJIT 与 Lua 5.4 下各跑一遍"。有了接缝之后它的形态必须变：两个引擎的脚本**不是同一份文件**，语言都不同。要保持的不是脚本相同，而是**行为相同**。

一个用例由三部分组成：

```text
tests/suite/counter/
  app.js          # QuickJS fixture
  app.lua         # Lua fixture
  expected.txt    # 期望的 spec 树（debug_tree 输出）
```

规则：

1. 断言的载体是 `render_to_spec` 的文本树——它在接缝之上、纯数据、与语言无关，因此"两个引擎产出同一棵树"是一个可执行的判据。
2. CI 跑两遍：默认 feature 一遍，`--no-default-features --features luajit` 一遍；三平台各一次。
3. **一个用例只有一份 fixture 时 CI 失败。** 这是"退路是真的"这句话唯一可执行的定义。没有这条，Lua 引擎会在三个月内静静地编不过。
4. 覆盖范围限定在接缝之上的行为：元素与样式、单次使用报错、回调代次、`cx` 越界、能力拒绝，以及**错误信息文本**——两个引擎给同一个错误说同一句话，是接缝的一部分，不是锦上添花。
5. 不覆盖语言本身的特性。`Array.prototype.map` 是否正确不是本项目要测的东西。

代价照说：每个行为要写两份 fixture。控制办法只有一个——套件保持小而关键（几十个用例的量级），并且只测上面第 4 条列出的那几类。

### 22.4 交互测试

用 GPUI 的 `TestAppContext` / `VisualTestContext` 驱动点击、输入、键盘，断言脚本状态与重绘次数。

### 22.5 与仓库测试原则的关系

遵循 `.claude/COMPONENT_TEST_RULES.md`：不为纯呈现尺寸写测试，重点覆盖复杂逻辑——本提案中即 CallScope 有效性、arena 复用报错、回调存活期、序列化往返、沙箱边界、值转换表、样式反射表非空、跨引擎行为一致。

---

## 23. 分发与版本

### 23.1 API 版本独立于 crate 版本

脚本 API 有自己的 semver，由插件在 `main.js` 里用 `gpui.require_api("1.0")` 声明。新增 API = minor；改变既有行为/移除 = major。弃用先经过一个 minor 的警告期（调用时 `log.warn` 一次）。

**API 版本与引擎无关**：同一个版本号在两个引擎下必须提供同一组能力与同一组行为（§22.3）。引擎不进版本号，也不进 manifest。

`"gpui"` 与后续的 `"gpui-component"` 各自独立计版本。

### 23.2 分发格式

先做最简单可用的：`.tar.zst` 包 + `index.json` 索引，托管在任意静态文件服务或 git 仓库；宿主按 URL 安装、校验签名与校验和。包里是**纯脚本源码**，没有编译产物、没有 `node_modules`。**先不建市场服务**——在插件数量证明需求之前，注册表服务是纯负债。

### 23.3 兼容性检查

`gpui.require_api` 在加载时就失败，错误信息给出插件要求的版本与宿主实际提供的版本；分发索引（§23.2）额外记录一个 `api` 字段供安装器预检，但它是索引的元数据，不是 manifest 的字段。

---

## 24. 备选方案对比

这张表按 v0.4 的决定重写。**它不宣称 v0.3 的论证有错**：LuaJIT 更小、更快、每次调用更便宜，这些事实一条都没变（§6.3）。变的是权重——应用层代码的可读性与语料覆盖被排到了前面——以及接缝让这次选择不再是单向门。

| 方案 | 优点 | 结论 |
| --- | --- | --- |
| **QuickJS / JavaScript（本方案）** | 应用层代码最好读（类、箭头函数、模板字符串、模块）；语料覆盖最好，对使用者 C 是决定性的；`.d.ts` 是现成的类型契约与检查器；能编到 wasm；无 JIT 因而无 W^X 问题；引用计数让宿主句柄的释放时机确定 | **采纳。** 代价照单认下：体积比 LuaJIT 与 Lua 5.4 都大；**没有 JIT**，热点循环与 `C_op` 都不占优；rest 参数分配是新增的一笔（§20.2）；GC 有环就要等回收器 |
| **LuaJIT / Lua（v0.3 的选择）** | 嵌入最成熟、体积最小、启动最快、`C_op` 最低；Neovim 已证明该模型 | **保留为退路，不是弃用。** `lua` / `luajit` feature 下可编译可运行，CI 双跑（§22.3）。真正的损失是语料与类型工具链，以及 LuaJIT 编不到 wasm、在 W^X 平台受限 |
| **WASM 组件模型** | 强隔离、多语言、无 GC 交叉 | 否决。每次调用要跨序列化边界，UI 这种高频细粒度调用最不适合；工具链重；调试差 |
| **Node.js / Deno 嵌入** | 完整 JS 生态、npm | 否决。进程模型与体积都不匹配（本方案是同进程、主线程、嵌入式）；引入 npm 等于引入原生依赖与供应链面；VS Code 那条路要有独立扩展进程才成立 |
| **Rhai / Steel / Koto（纯 Rust 脚本）** | 无 C 依赖、Rust 互操作最顺 | 否决。生态几乎为零；作者要学新语言；语料稀薄——对使用者 C 是硬伤 |
| **Rust dylib 插件** | 全速、类型安全 | 否决。无稳定 ABI；无沙箱；编译成本仍在，等于没解决动机中的任何一条 |
| **Rust 热重载** | 保留 Rust 全部能力 | 否决。只解决"编译慢"，不解决插件分发与第三方扩展；状态保持脆弱 |

一句话总结这次更换：**用运行时性能的一部分余量，换应用层代码的可读性与语料覆盖，并且用一条接缝把这笔交易做成可撤销的。** 能不能换成，M0 的数字说了算（§26）。

---

## 25. 风险台账

| # | 风险 | 影响 | 缓解 | 判定时点 |
| --- | --- | --- | --- | --- |
| R1 | 跨语言调用成本超预算；base-first 的高 op 数放大了这一风险，且样式无批量口子 | 致命 | 压低 `C_op`（特化调用形态）、memo、虚拟化、更细的 view 划分 | **M0 基准测试（准入门槛）** |
| R2 | 呈现权在脚本 ⇒ 上手成本高、界面质量参差 | 高 | 默认 token 调色板 + 可替换预设模块 + 示例；后续 `gpui-component` 提供成品视觉 | M1 / M6 |
| R3 | 绑定表面与上游漂移 | 高 | 绑定表 + rustdoc JSON 比对 CI；Tier 分层限制承诺范围 | M1 |
| R4 | 跨 GC 循环引用泄漏 | 中 | 回调按帧/按 owner 释放；`gc_stats` 观测；长跑压力测试 | M2 |
| R5 | 沙箱逃逸（以 `Eval` intrinsic、quickjs-libc 的 `std`/`os`、原型污染为最） | 高 | 显式组装 intrinsic + 从不注册 libc 模块 + 冻结内建原型 + 逃逸测试套件 + 默认无能力 | M4 |
| R6 | 引擎在某个目标平台不可用 | 中 | 两个引擎覆盖面互补：QuickJS 可编 wasm、无 W^X 问题；LuaJIT 在这两处受限而性能更好 | M0 |
| R7 | shell 的预设样式事实上变成"第三套视觉系统" | 中 | §5.6 纪律：预设只以脚本源码存在、可整体替换；Rust 侧零视觉决策 | 持续 |
| R8 | 两个模块名（`gpui` / `gpui-component`）造成生态分裂 | 中 | 第一期即定命名与职责边界（§14.6）；共用同一渲染协议与文档骨架 | M6 |
| R9 | 维护成本超出团队承受 | 高 | 生成优先、Tier 3 明确不做、每阶段有退出标准 | 每个 M 结束 |
| R10 | 调试体验差导致无人使用 | 中 | stack + 错误覆盖层 + 热重载 + DevTools 提前到 M1/M2；`.d.ts` 把一部分错误提前到编辑期 | M2 |
| R11 | 脚本 action 名 `&'static str` 泄漏累积 | 低 | intern 表 + 上界分析 + 文档写明 | M4 |
| **R12** | **QuickJS 的 `C_op` 过不了 M0 门槛（无 JIT，且比 LuaJIT 多一笔 rest 参数分配）** | **致命，但可逆** | **切引擎：`--no-default-features --features luajit`。这正是接缝存在的理由（§6.5）。切换前先走完 §20.4 的优化清单——接缝是退路，不是免测的借口** | **M0 基准测试** |
| **R13** | **双引擎腐烂：Lua 引擎在几个月内静静编不过，"退路"变成一句文档** | **高** | **§22.3 的双 fixture 硬性要求 + CI 双 feature 构建；§6.5 的"新能力一律加在接缝之上"；接缝之下的代码量必须持续小于接缝之上** | 每次 CI |
| **R14** | **模型按 Node / 浏览器假设生成代码（`fetch`、`require`、`setTimeout`、npm 包）** | **中** | **报错桩给出替代品（§19.1）、`.d.ts` 让编辑期就报错（§14.4）、文档首页明确列出"这里没有什么"** | M1 |

**R1 是唯一的一票否决项**，R12 是它在 v0.4 下的具体形态；区别在于 R12 有一条不改设计就能走的退路。其余风险都有可接受的降级路径。

---

## 26. 路线图

每个阶段都有**退出标准**；不达标则调整范围或终止，而不是顺延。

### M0 — 可行性基准（2–3 周）

引擎接缝、`CallScope`、一个窗口、`div` / `text` / `h_flex` / `v_flex` / base `Button` / `Checkbox` / `Switch`、`on_click` / `on_change`、`cx.notify()`、默认 token 调色板、反射样式表 + 57 个有参样式、spec 文本树。**两个引擎都要能跑。**

退出标准：

- 800 节点 × 12 op 的脚本渲染 **< 1.5 ms**（三平台各测）；
- **同一份基准在 QuickJS 与 Lua 引擎下各跑一遍，两组数字都记录在案**——这是 R12 的判定依据，也是"接缝是真的"的第一次验证；
- 两个引擎产出同一棵 spec 树（§22.3 的第一个用例）；
- 越界使用 `cx` 得到明确异常，无 UB、无 panic；
- release 构建下样式反射表非空。

**不达标即停止或重新设计**——但"QuickJS 不达标而 Lua 达标"不算不达标，那是接缝在做它该做的事。

### M1 — 元素模型与呈现（3–4 周）

有参样式补全、未知方法名诊断、状态样式与 hover/active/focus-visible、预设模块（JS 与 Lua 各一份）、错误覆盖层、热重载、spec 快照测试、`.d.ts` 首版。

退出标准：用纯脚本复刻 `crates/base/examples/showcase` 中三个中等复杂度的组件页，视觉与 Rust 版一致。

### M2 — 状态、事件与实体（3–4 周）

view 生命周期、`InputState` / `TextareaState`、Checkbox/Switch/Radio/Select/Combobox、Tabs、List/Table/Tree（含 item renderer 的嵌套 CallScope）、实体事件订阅、`ShellRoot`（dialog / sheet / toast 栈）、`gpui.open_window`、DevTools。

退出标准：一个带表单、列表、筛选、详情与对话框的完整脚本应用可用；`gc_stats` 在 30 分钟压力操作后无单调增长。

### M3 — 异步与系统能力（2–3 周）

**先把异步补进引擎契约（§6.5 的已知缺口）**：Promise ↔ `Task` 桥接、job queue 泵、取消语义在两个引擎下的对齐。然后是定时器、任务 owner 绑定、fs / http（宿主注入客户端）/ store / clipboard / log。

退出标准：脚本应用能拉取远程数据、落盘、重启后恢复；关闭面板后未完成任务确实被取消；未处理的 rejection 一定可见。

### M4 — Dock 与插件模型（4–5 周）

`ScriptDockSkin` 三个 renderer、`ScriptPanel` + `PanelRegistry` + 布局往返、manifest + schema、脚本侧贡献注册 API、能力授权 UI、显式 intrinsic 组装、模块解析器、原型冻结、限额。

退出标准：脚本面板与 Rust 面板在同一个 `DockArea` 中行为一致；卸载再安装后面板回到原位；沙箱逃逸测试全绿，含 §19.3 那条中断可捕获性的结论。

### M5 — 分发与独立运行时（3 周）

`gpui-shell` 可执行文件（`run` / `pack` / `install`）、签名与校验、`.d.ts`（与 LuaCATS）生成、文档站点章节（en + zh-CN 双语）。

退出标准：第三方能在不接触本仓库源码的前提下，从零写出并分发一个插件。

### M6 — `gpui-component` 绑定（3–4 周）

第二个绑定注册表接入成品视觉组件；`ShellRoot` 与 `Root` 的关系定案；两个模块名的文档与迁移指南。

退出标准：同一个脚本应用切换 import 即可在"自绘视觉"与"产品视觉"之间迁移，业务逻辑不改。

### M7 — 可选扩展

wasm 目标（QuickJS 与 Lua 5.4 都可编，届时比一次体积与启动时间）、更多 Tier 2 组件、性能再优化。

---

## 27. 开放问题

1. **预设模块该做多厚？** 太薄则作者每次从零写样式，太厚则事实上成为第三套视觉系统（R7）；而且它现在要写两份（§13.4）。建议 M1 只做"按钮 / 输入 / 卡片 / 列表行"四类，再按真实反馈扩展。
2. **`ShellRoot` 与 `Root` 是否最终合并？** M6 定案，届时有实际使用数据。
3. **脚本能否定义可被其他插件引用的可复用模块？** 跨插件依赖带来版本解析、加载顺序、循环依赖。建议 v1 只允许插件内复用。
4. **多窗口下 VM 的粒度**：单 VM 多窗口（共享状态，简单）还是每窗口一 VM（隔离，但状态同步复杂）？倾向单 VM，M2 用真实场景验证。注意单 VM 是 §19.1"冻结内建原型"那一条的前提。
5. **Editor 的窄接口长什么样？** Tier 3 明确不绑完整接口，但"给文本 + 给语言 + 给只读标志"的最小形态值得在 M2 探一次。
6. **设置系统归属**：插件设置项走宿主 settings UI（一致性好）还是插件自绘（灵活）？倾向前者，由 `gpui.register_settings(schema)` 在脚本里声明、宿主渲染。
7. **异步怎么进接缝契约？**（§6.5 的已知缺口）Promise 与 coroutine 要归到同一组操作上，候选是"把 `Task<T>` 变成可等待值" + "把待执行 job 跑完"两条。M3 前必须定，否则调度器会在两个引擎里各长一套。
8. **兼容桩的边界画在哪？** `console.log` 映射到 `gpui.log`（做了），`setTimeout` 报错指向 `gpui.timer`（做了），那 `structuredClone`、`TextEncoder`、`URL`、`crypto.randomUUID` 呢？判据草案：**语义能一比一映射的可以给，语义只有八成像的一律不给**（§17.2 拒绝 `fetch` 用的就是这一条）。需要一份清单，M1 给出。
9. **退路的保质期。** 什么时候可以承认 Lua 引擎已经没人用、可以删掉？建议判据是"连续两个里程碑没有任何一次 CI 失败是由 Lua fixture 抓出来的，且没有平台强制需要它"——在那之前，双 fixture 的成本照付（R13）。

---

## 28. 附录

### 附录 A：完整示例应用（第一期，呈现权在脚本）

`examples/js_checklist/main.js` 的完整形态（M2 之后，窗口由脚本打开）：

```js
import { gpui, View, div, h_flex, v_flex, text, Button, Checkbox, Input, InputState, VirtualList }
  from "gpui";
import { button as presetButton, input as presetInput, checkbox as presetCheckbox }
  from "gpui/preset";

const FILTERS = [
  ["all", "All"],
  ["open", "Open"],
  ["done", "Done"],
];

class TodoApp extends View {
  init() {
    this.input = InputState.new({ placeholder: "What needs doing?" });
    this.items = [];
    this.filter = "all";

    this.input.on_event("submit", (value, cx) => {
      if (value === "") return;
      this.items.push({ text: value, done: false });
      this.input.set_value("");
      cx.notify();
    });
  }

  visibleItems() {
    if (this.filter === "all") return this.items;
    return this.items.filter((item) => (this.filter === "done") === item.done);
  }

  filterButton(id, label) {
    return presetButton(Button.new(id), {
      variant: this.filter === id ? "primary" : "ghost",
      size: "sm",
    })
      .child(text(label))
      .on_click((_event, cx) => {
        this.filter = id;
        cx.notify();
      });
  }

  renderItem(item, index, cx) {          // CallScope phase = Layout
    return h_flex()
      .items_center()
      .gap_2()
      .py(6)
      .child(
        presetCheckbox(Checkbox.new(`done-${index}`))
          .checked(item.done)
          .on_change((checked, cx) => {
            item.done = checked;
            cx.notify();
          }),
      )
      .child(
        text(item.text).when(item.done, (el) =>
          el.line_through().text_color("muted_foreground"),
        ),
      );
  }

  render(cx) {
    return v_flex()
      .size_full()
      .p(16)
      .gap_3()
      .bg("background")
      .child(presetInput(Input.new(this.input)))
      .child(h_flex().gap_2().children(FILTERS.map(([id, label]) => this.filterButton(id, label))))
      .child(
        VirtualList.new("todos")
          .items(this.visibleItems())
          .render_item((item, index, cx) => this.renderItem(item, index, cx)),
      );
  }
}

gpui.run(() => {
  gpui.open_window({ title: "Todo", width: 520, height: 640 }, () => TodoApp.new({}));
});

export default TodoApp;
```

几处值得指出的写法：

- **事件处理器一律是箭头函数**，因为它们要在回调里用 `this` 访问 view 状态（§10.1）。
- **`children(...)` 接一个数组**，配合 `map` 就是 JS 最自然的列表写法（§15.1）。
- **`.when(cond, fn)`** —— 与 CLAUDE.md 要求的 GPUI builder style 同构：保持一条链，用 `when` 表达条件，而不是把链拆成一个临时变量再逐条 if。
- **样式方法是 snake_case，自己写的方法是 camelCase**（`visibleItems`、`renderItem`）。这是 §6.4 那次取舍在真实代码里的样子。

同一个应用在 M6 之后切到产品视觉，只改构建界面的两个函数：

```js
import { Button } from "gpui-component";

filterButton(id, label) {
  return Button.new(id)
    .label(label)
    .size("sm")
    .when(this.filter === id, (el) => el.primary())
    .on_click((_event, cx) => {
      this.filter = id;
      cx.notify();
    });
}
```

### 附录 B：crate 布局

```text
crates/shell/                 # gpui-shell（第一期只依赖 gpui-base + gpui）
  Cargo.toml                  # features: quickjs（默认）/ lua / luajit
  src/
    lib.rs                    # init / 公共导出
    engine/                   # ← 接缝（§6.5）
      mod.rs                  #   契约、compile_error! 守卫、cfg 转发
      quickjs.rs              #   默认引擎（rquickjs）
      lua.rs                  #   退路引擎（mlua），可编译可运行
    runtime.rs                # CallbackArena<T> / 错误覆盖层（引擎无关）
    scope.rs                  # CallScope（唯一的 unsafe 模块）
    spec.rs                   # SpecArena / SpecNode / SpecOp
    materialize.rs            # 描述 → 真实元素（纯 Rust）
    style.rs                  # 反射样式表 + 57 个有参样式 + 拼写建议
    theme.rs                  # 默认语义 token 调色板与 token 名解析
    value.rs                  # Bridged 与颜色/长度转换
    error.rs                  # ShellError（中立错误类型）
    capability.rs             # Capabilities / 路径解析 / 授权判定
    view.rs                   # ScriptView
    host.rs                   # fs http store clipboard log（待实现）
    scheduler.rs              # Promise/Task 桥接 · timer（待实现，见 §6.5 缺口）
    sandbox.rs                # intrinsic 组装 · 模块解析器 · 限额（待实现）
    bindings/                 # 绑定表（数据 + 宏展开）           （规划）
    root.rs                   # ShellRoot（dialog / sheet / toast / 焦点栈）（规划）
    dock.rs                   # ScriptDockSkin + ScriptPanel      （规划）
    plugin/                   # manifest · 能力授权 · 贡献注册     （规划）
    bin/gpui-shell.rs         # run / pack / install
  theme/default-tokens.json   # 默认语义 token 调色板
  js/                         # 随包发布的 JS 模块（含 preset，可替换）
  lua/                        # 同上，Lua 引擎那一份
  types/                      # 生成的 .d.ts（与 LuaCATS）
  tests/                      # render 端到端 + suite/（跨引擎行为套件，§22.3）
examples/js_checklist/            # 最小示例（JS）
examples/lua_hello/           # 同一个示例的 Lua 版，用于验证退路
```

`crates/shell` 已在根 `Cargo.toml` 的 members 中，路径与目录名一致。

### 附录 C：命名约定（遵循 CLAUDE.md）

- 不使用 `Kind` 后缀：`ScopePhase` 而非 `ScopeKind`，`ExecuteGrant` 而非 `CapabilityKind`，`SpecOp` 而非 `SpecOpKind`。
- 跨 seam 的公共数据类型（`ShellOptions`、`Capabilities`、`PluginManifest`）一律私有字段 + 读方法，选项类型由 `default()` 起手再链式设置；全布尔类型的读方法用 `is_*` / `has_*`，含非布尔字段的类型 setter 用 `with_*`。`Capabilities` 已经是这个形态。
- `Context` 拼全：`PanelBuildContext`、`TabGroupContext`，不用 `Ctx`；`cx` 只留给 GPUI 的 `App` / `Context<T>` / `AsyncApp`，以及脚本侧那个同名的上下文对象。
- **Rust 侧的类型名不带语言**：`ScriptView` / `ScriptPanel` / `ScriptDockSkin`，不叫 `JsView` / `LuaView`——它们在接缝之上，不知道语言是什么（§6.5）。引擎内部的类型可以带，因为那里确实只有一种语言。
- **脚本侧从绑定来的方法名与 Rust 一致（snake_case），不做 camelCase 重命名**（§6.4）；作者自己写的方法与变量按各语言习惯（JS 用 camelCase）。

### 附录 D：文档产出计划

本 RFC 属 `docs/`（内部规范，单语言）。若提案被接受并进入实现，面向使用者的文档进 `website/docs/` 与 `website/zh-CN/docs/`，双语同步——按 CLAUDE.md 要求，中文文档中的框架、组件与 API 名保留英文原形。

面向使用者的文档以 JavaScript 为准，Lua 引擎只在"如何切换引擎"一节出现：**退路是给维护者的，不是给应用作者的**。同时随文档发布 `.d.ts`（§14.4），它既是编辑器契约，也是喂给模型的 API 描述。
