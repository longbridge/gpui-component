# gpui-base Component Design Review

评审对象：`crates/base`（94 个源文件，约 34k 行）
评审日期：2026-08-12
评审范围：API 表面、样式解析路径、状态所有权模型、可访问性埋点、`crates/ui` 回接情况

## 进度

| 问题 | 状态 |
| --- | --- |
| P0-1 语义态样式优先级自相矛盾 | **已修复并验证**（2026-08-12） |
| P0-2 `Combobox::on_confirm` 错接到 Escape | **已修复并验证**（2026-08-12） |
| P1-2 回调命名与签名不统一 | **已修复并验证**（2026-08-12） |
| P1-3 弹层定位两套实现 | **已合并并验证**（2026-08-12） |
| P1-4 可访问性覆盖不完整 | **可做部分已补齐**（2026-08-12）；无视觉产出，待读屏/accesskit 断言验证；其余上游阻塞 |
| P2-1 `Transition` 同名 | **已修复并验证**（2026-08-12） |
| P2-5 Radio 回接 | **已回接并验证**（2026-08-12） |
| P1-1、P2-2、P2-3、P2-4 | 未开始 |

P1 动手前应先补一份 primitive 契约文档（见 §五），否则回调统一和状态命名又会变成一次性的局部决定。

---

## 一、总体结论

方向是对的，分层契约（"框架拥有行为、应用拥有外观"）在大多数控件上被真正贯彻了。全量扫描 base 源码里的 `bg` / `rounded` / `px_` / `border_1` / `text_color` 等视觉字面量，除测试代码外只剩 `resizable/resize_handle.rs` 一处，而它走的是 `ResizableTheme` 注入。这在一个从有样式库剥离出来的 primitives 层里已经算很干净了。

但它现在还不是一个"设计一致"的 primitives 库，更像一批**按迁移顺序陆续剥离出来、各自局部最优的控件集合**。最大的问题不是缺功能，而是缺一份被强制执行的横切契约：同类问题在不同组件里有 2~4 种解法。

---

## 二、组件盘点：五种抽象形态

按"应用需要怎么用"分类。这个分类本身就是结论——一个 primitives 库出现 5 种形态，说明抽象边界还没收敛。

### A. 无样式受控控件（最成熟，是这套库的样板）

`Button`、`Checkbox` + `CheckboxIndicator`、`Switch` + `SwitchTrack` + `SwitchThumb`、`Radio`、`Toggle`、`Link`、`Tab` / `Tabs`、`Accordion` 系列、`Collapsible`、`Progress` 系列、`Avatar` 系列、`Table` 系列、`Dialog` / `AlertDialog` 系列、`Toast`

统一形态：

```
#[derive(IntoElement)] + Styled + ParentElement + InteractiveElement
  + styles(|s| s.checked(..).disabled(..))   // 语义态样式
  + accessibility_label
  + tab_index / tab_stop
```

这一层做得好。`Button::new(id)` 无 padding 无背景是明确契约而非缺失，README 也讲清楚了。

`Table` 用宏批量生成 7 个语义 part（`table.rs:8-58`），只注入 `Role` 和 `aria_row_index` / `aria_column_index`，是本库里最贴近 Base UI 精神的实现。

### B. 受控根 + 应用自绘内容

`Select`、`Combobox`、`DatePicker`、`Sheet`、`Popover`、`HoverCard`、`Popup`

只拥有 open 状态流转、焦点转移、键盘动作、dismiss，内容完全由应用给。但 `Popover` 通过 `window.use_keyed_state` 内建状态，`Select` 则完全受控——两者在同一族里就不一致。

### C. Entity 状态型（其实已经不是 primitive）

`Slider`(`SliderState`)、`Calendar`(`CalendarState`)、`Tree`(`TreeState`)、`OtpInput`(`OtpState`)、`NumberInput`(`InputState`)、`Resizable`(`ResizableState`)、`Input`(`InputState`)

`Calendar` 尤其越界：`calendar.rs` 947 行，自带月/年视图切换、翻页、六周网格、disabled matcher、`CalendarItemKind`，文档自己写着 "Base owns navigation, view switching, grids... The UI crate only decorates the pre-wired item slot"。这已经是一个**完整组件加了个换肤槽**，不是 primitive。`Tree`（643 行，自带虚拟化 + 选中 + 展开 + 键盘）同理。

对比 Base UI：它不提供 Calendar，那是 shadcn 层的事。

### D. 纯逻辑模型（无元素）

`PaginationState`、`ColorPickerState`、`ToastManager<I, T>`、`History`、`SliderState`

`ColorPickerState` 完全没有配套元素，只导出了一个状态机；`ToastManager` 是泛型纯逻辑 + 定时器。这类 headless model 其实是很好的设计，但它跟 A 类的差异需要在文档里显式命名，现在混在同一个导出列表里。

### E. 基础设施

`FocusTrapElement`、`Scrollbar`、`VirtualList`、`AutoScroll`、`Measure`、`motion` / `animation`、`theme_tokens`、`geometry`、`GlobalState`、`actions`

定位清晰。`SemanticThemeTokens` 严格避开了组件名字段（`theme_tokens.rs` 模块注释有明文约束），是对的。

---

## 三、问题清单

### P0-1　语义态样式的优先级自相矛盾

README 白纸黑字写着：*"Styles applied directly in the main builder chain have the highest priority."*

实际有两套：

| 顺序 | 组件 | 结果 |
| --- | --- | --- |
| 先语义态，后 `.style` | `button.rs:127`、`checkbox.rs:172,298`、`switch.rs`、`toggle.rs`、`radio.rs`、`link.rs` | **实例样式赢**，符合文档 |
| 先 `.style`，后语义态 | `tabs.rs:80`、`input/mod.rs:127` | **语义态赢**，违反文档 |

更糟的是两边都有测试把各自的行为锁死了：

- `button.rs` 的 `selected_disabled_and_instance_styles_have_explicit_priority` 断言实例赢
- `input/mod.rs` 的 `semantic_state_styles_override_the_normal_style` 断言语义态赢

也就是说这不是笔误，是两个时期做的两个决定，各自写了回归测试。

应用侧写 `Tab::new(x).bg(A).styles(|s| s.selected(|s| s.bg(B)))` 时行为跟 `Button` 相反，且没有任何编译期提示。

#### 决策：统一为「语义态优先」——已落地

直觉上"README 已经这么写、多数组件已经这么实现"会导向统一为**实例样式赢**。这个方向是错的，四条证据：

1. **GPUI 自身就是状态叠加在基础样式之上。** `Interactivity` 的 `hover_style` / `active_style` 在 paint 阶段叠加到已计算的 base style 上。`styles(|s| s.disabled(..))` 与 `.hover(..)` 挂在同一个 builder 上却用相反优先级，是本地不一致。
2. **`crates/ui` 的 Input 依赖语义态优先才能工作。** `ui/input/input.rs:505` 把焦点边框放进 `styles(|s| s.focused(..))`，普通边框 `.border_1().border_color(cx.theme().input)` 放在实例链上。只有语义态优先才能让焦点环覆盖普通边框。
3. **反向优先级逼出了两处绕行。** `ui/checkbox.rs` 和 `ui/button/toggle.rs` 都在状态闭包里写了 `.refine_style(&instance_style)`，`ui/toggle` 甚至里外各来一遍。
4. **`CheckboxIndicator` 用 `.when(!checked, ..)` 守卫绕行**，只在非选中时才设基础色。

**最终解析顺序（所有 primitive 一致）：** 实例链 → 值态（`checked` / `pressed` / `selected` / `focused` / `indeterminate`）→ `disabled`（永远最后）。

**实现方式：** 顺序不再在 10 处各写一遍，而是收敛到 `state_style.rs` 的 `resolve_style(instance, active_states)` 单点定义，Button / Checkbox / CheckboxIndicator / Switch / SwitchTrack / SwitchThumb / Toggle / Radio / Link / Tab / Input 全部路由过去——顺序在结构上无法再分叉。该函数自带 4 条契约单测。

**兼容性处理：** ui 侧多数 façade 本就有 `!selected && !disabled` 守卫或已把 `instance_style` 放进状态闭包，行为不变；那两处当初看着像 hack 的写法，在新契约下正是"让调用方样式压过状态"的正规表达，予以保留。唯一真会变行为的是 `ui/button/button.rs`——它在实例链末尾无条件重放 `instance_style`，已在两个状态闭包里补上同样的重放以保持 100% 兼容。

**残留缺口：** `Interactivity::hover_style` / `active_style` 在 GPUI 里是 `pub(crate)`，base 无法在 disabled 时抹掉调用方装的 hover / active 样式。已在 README 写明由调用方 `when(!disabled, ..)` 守卫，并列入上游诉求。

**改动清单：** `state_style.rs`（新增 `resolve_style` + 4 条契约单测）、`button.rs`、`checkbox.rs`、`switch.rs`、`toggle.rs`、`radio.rs`、`link.rs`、`tabs.rs`、`input/mod.rs`、`ui/button/button.rs`、`crates/base/README.md`（优先级描述改写为三层顺序，并补上 hover/active 的调用方守卫说明）。

### P0-2　`Combobox::on_confirm` 实际绑在 Escape 上

`combobox.rs:124` 把 `on_confirm` 传给了 `Select::on_escape`，而 `select.rs:224` 的 `on_escape` 只在 `Cancel`（Escape）action 里触发。测试名字自己承认了：`escape_confirms_then_closes_and_restores_trigger_focus`。

这可能是为了兼容旧 gpui-component 里"关闭时提交输入值"的行为，但在一个新 primitives 层里，`on_confirm` 触发于取消是纯粹的认知陷阱。

#### 决策：拆成两个正交回调——已落地

核查确认 `ui/combobox.rs` 确实依赖"关闭即提交"（它在该回调里 emit `ComboboxEvent::Confirm`）。因此**行为保留，命名改诚实**：

- `Select::on_escape` → `Select::on_dismiss`（语义不变，仍在 `Cancel` 分支、且在 `on_open_change(false)` 之前触发，调用方仍能读到待提交值）。
- 新增 `Select::on_confirm`，接到真正的 `Confirm` action；仅在 open 时触发（关闭态按 Enter 仍是打开弹层，不变）。
- `Combobox` 同时透传 `on_confirm` 与 `on_dismiss`。
- `ui/combobox.rs` 改接 `on_dismiss`，行为逐字节不变。

测试从原来那条自相矛盾的 `escape_confirms_then_closes_...` 拆成 `escape_dismisses_then_closes_and_restores_trigger_focus` 与 `enter_confirms_without_dismissing_while_open`。

**改动清单：** `select.rs`、`combobox.rs`、`ui/combobox.rs`。

### P0 验证记录（2026-08-12）

自动化：

- `cargo test -p gpui-base --lib` — 246 passed / 0 failed（较改前 241 增加 4 条样式顺序契约与 1 条 confirm 路径）。
- `cargo check -p gpui-component --lib` — 0 error。
- `cargo clippy -p gpui-base -p gpui-component --lib --all-targets` — 无新增告警（余下两条为既有的 `input/mode.rs` 参数过多与 showcase example 的 unused import）。
- 仅对改动的 13 个文件执行 rustfmt，避免 `cargo fmt -p` 重排整个 crate。

人工：用户在 story gallery 与 `base_components` showcase 中逐项确认，判据为「任何状态叠加 `disabled` 时 disabled 视觉必须赢」，覆盖 base 的 Button / Checkbox / Switch / Toggle / Radio / Link 六页，ui 的 Button（全 variant × selected × disabled × loading）、Checkbox、Switch、Toggle、ToggleGroup、NumberInput，以及 Combobox 的 Esc 提交与 Enter 确认路径。结果符合预期。

`ui::Radio` 与 `ui::Link` 不在验证范围：二者尚未回接 base 原语（`radio.rs` 仅用 `RadioGroup`，`link.rs` 对 `gpui_base` 零引用），本次改动触及不到，对应待办见 P2-5。

### P1-1　状态所有权模型四种并存

同一个库里：

1. 纯受控 —— `Select`、`Checkbox`
2. `window.use_keyed_state` 内建 —— `Popover`、`HoverCard`、`Button` 的 focus handle
3. `Entity<XxxState>` —— `Slider`、`Calendar`、`Tree`
4. 普通结构体 by-value —— `PaginationState`、`ColorPickerState`

`Popover` 和 `Select` 同属 overlay 族却分属前两种。应用要同时用这两个，就要同时理解两套心智模型。

建议定一条规则：**primitive 默认纯受控，内建状态只作为 `default_open` 这类 uncontrolled 便利路径，且必须能被 `open()` 覆盖**。`Popover` 已经这么做了，把它推广成契约即可。

### P1-2　回调命名与签名不统一

| 组件 | 方法 | 签名 |
| --- | --- | --- |
| `Checkbox` | `on_change` | `Fn(CheckboxState, &mut Window, &mut App)` |
| `Radio` / `Toggle` | `on_change` | `Fn(bool, &ClickEvent, &mut Window, &mut App)` |
| `Switch` | **`on_toggle`** | `Fn(bool, &ClickEvent, &mut Window, &mut App)` |
| `AccordionTrigger` | **`on_toggle`** | `Fn(bool, &mut Window, &mut App)` |
| `Pagination` | `on_change` | `Fn(&usize, ...)` ← 引用传 `usize` |
| `Link` | **`on_activate`** | `Fn(&ClickEvent, ...)` |
| `Tab` / `Button` | `on_click` | `Fn(&ClickEvent, ...)` |

四个语义完全相同的"受控值变更"用了三个名字、三种签名。`Checkbox` 独缺 `&ClickEvent`（拿不到修饰键，做 shift-range-select 就得绕开 primitive）。`Pagination` 的 `&usize` 是纯噪音。

Base UI 的做法是统一 `onCheckedChange` / `onPressedChange` / `onValueChange`，语义清晰且可预测。

#### 决策：按两层收敛——已落地

统一成一种形态在 `Pagination` 上会走不通：`PaginationState::request_page` 是**模型方法**，也能被键盘或应用代码驱动，塞 `&ClickEvent` 会逼调用方伪造事件。因此契约分两层：

- **元素激活回调** —— `on_change(value, &ClickEvent, &mut Window, &mut App)`。适用 `Checkbox`、`Radio`、`Toggle`、`Switch`、`AccordionTrigger`。
- **模型请求方法** —— `on_change(value, &mut Window, &mut App)`，不带指针事件。适用 `PaginationState`。

具体改动：

| 组件 | 改动 |
| --- | --- |
| `Checkbox` | 补 `&ClickEvent`（其 `on_click` 第一参数即是，直接接上） |
| `Switch` | `on_toggle` → `on_change`；签名本就达标 |
| `AccordionTrigger` | `on_toggle` → `on_change`，并补 `&ClickEvent` |
| `Pagination` | `&usize` → `usize`；不加 `&ClickEvent`，理由见上 |
| `Radio` / `Toggle` | 已达标，未动 |
| `Link::on_activate` | 保留——语义不是"受控值变更" |
| `Button` / `Tab` 的 `on_click` | 保留——无受控值 |

**没有保留 `#[deprecated]` 别名**：base 未发布、调用点全在仓库内（ui 5 处 + showcase 6 处 + base 自身测试），而 CI 的 clippy 是 `--deny warnings`，留别名反而会卡住构建。直接改名并同步全部调用点更干净。

`gpui_component` 的对外 API 未变化：ui 侧只是适配了 base 的新签名，自身的 `on_click` / `on_toggle_click` 等公开回调原样保留。

**改动清单：** base 的 `checkbox.rs`、`switch.rs`、`accordion.rs`、`pagination.rs`；ui 的 `switch.rs`、`checkbox.rs`、`accordion.rs`、`pagination.rs`；showcase 的 `checkbox.rs`、`switch.rs`、`accordion.rs`、`pagination.rs`。

**验证（2026-08-12）：** `cargo test -p gpui-base --lib` 246 passed / 0 failed；`gpui-component` 与 `base_components` example 均 0 error；clippy 无新增告警。人工确认 Checkbox / Switch / Toggle / Accordion 的点击回调仍正确改变受控值，Pagination 翻页正常。

### P1-3　弹层定位有四套实现

- `Popup` —— `anchored` + `deferred` + `ElementExt` 测量 + 窗口边缘吸附
- `TooltipPositioner` —— 自己实现了 `Element` trait，做 viewport flip / clamp
- `Popover` / `HoverCard` —— 都包 `Popup`
- `Sheet` / `Dialog` —— 另一套

`Popup` 和 `TooltipPositioner` 是两份独立的"anchor + 翻转 + 夹紧"逻辑。Base UI 只有一个 `Positioner`。这块是最值得合并的技术债——现在改一处 viewport 行为要同步两个地方。

#### 决策：抽出统一 `Positioner`——已落地

深挖时的关键发现：GPUI 的 `Anchored` **本来就有翻转**（`AnchoredFitMode::SwitchAnchor` 是默认值），但 `Popup` 调用 `.snap_to_window_with_margin(px(8.))` 会把 fit_mode 替换掉，等于主动关闭了翻转。而 GPUI 表达不了"翻转 + 留边距"这个组合，这正是 tooltip 当初自己写一套的原因。

新增 `positioner.rs`，一个元素支持两种策略：

- `Positioner::corner(anchor, position)` —— 复刻 GPUI `anchored` 的角点锚定 + 夹紧，**不翻转**。逐行核对过 `Bounds::from_anchor_and_size` 与 snap 的算术，与原实现等价，所以 Popup 迁移零视觉变化。
- `Positioner::side(trigger_bounds)` —— 边放置 + 翻转 + 夹紧，并新增 `align`（Start/Center/End）与 `offset` 维度。tooltip 原有逻辑推广而来。

**元素数量收益：** 弹层内容原先是 `deferred( anchored( div().relative( content ) ) )`，现在是 `deferred( Positioner( content ) )`——每个打开的弹层少一层元素。`TooltipPositioner` 保留为薄包装（`crates/ui/tests/base_compat.rs` 引用了这个名字），其 `IntoElement` 直接返回内部 `Positioner`，同样不增加元素。

**等价性证据：** 合并后，tooltip 原有的 5 条定位测试**一字未改**全部通过；确认等价后才把它们迁到 `positioner.rs`（逻辑所在地），tooltip 只保留 provider 生命周期测试。`positioner.rs` 另有 7 条新测试覆盖 corner/side 两条路径。

**尚未迁移：** Popover 与 HoverCard 目前走 corner 策略，仍无翻转；切到 side 策略会改变弹层位置，应按原计划分步做并逐步目视验证。

**影响面的一处更正：** 本文初稿把 Select / Combobox / DatePicker / Sheet 也算作受影响。核查后：这些组件连同全部菜单都直接使用 GPUI 的 `anchored()`，并不经过 `base::Popup`。实际走改动路径的只有 `ui/popover.rs`、`base::HoverCard` 与 tooltip 三处。

**验证（2026-08-12）：** 人工确认 Popover 与 HoverCard 贴四条窗口边的位置与改动前一致（corner 为等价复刻，判据是"不应有任何变化"），Tooltip 贴边翻转正常且保留 4px 边距。

### P1-4　可访问性覆盖不完整

已做得不错的：

- `Button` / `Checkbox` / `Switch` / `Radio` / `Toggle` —— role + `aria_toggled` + `aria_label`
- `Slider` —— `aria_orientation` + numeric value 三件套
- `Progress`
- `Table` —— row / column index
- `Accordion` —— `Role::Heading` + `aria_level` + `Role::Region` + `aria_expanded`

缺口：

- **完全无 role / aria**：`Collapsible`、`Popover`、`HoverCard`、`Sheet`、`Avatar`、`Calendar`、`Tree`、`VirtualList`
- **无关系属性**：`Tab` 有 `aria_selected` 但没有 `aria_controls`；`AccordionTrigger` 有 `aria_expanded` 但没指向 panel；`Select` 有 `aria_expanded` 但没有 `aria_activedescendant`
- **`Table` 缺 row / column count**（只有 index）
- **`aria_disabled` 缺失** —— 这个是 GPUI 上游的限制，`button.rs` 的测试里已经诚实地记录了这一点（`assert!(!disabled.is_disabled())` + 注释），处理方式很好
- `Tabs` 无键盘导航，doc comment 已明确承认

#### 决策：先补不改构造签名的部分——已落地

已补：`Table::row_count` / `column_count`（为此把 Table 从宏里拆出手写）、`Tab::set_position`、`Radio::set_position`、`Slider` 的 `aria_numeric_value_step`、`Popover` 内容的 `Role::Dialog`。

**一处方案纠错：** 原计划写的"给 Select / Combobox 加 `aria_active_descendant`"是按 Web 的容器侧模型设想的。GPUI 的 `aria_active_descendant()` 是**零参数、设在后代元素上**的，语义相反。而 Select 的选项是应用自有的子元素，base 根节点无从代劳——该条已撤回，改为在 `Select` 的文档注释里说明由应用给高亮项打标记。

**未做，因为需要改构造签名：** `Collapsible` 与 `Avatar` 用的是无 id 的 `Div`，而 GPUI 的 `role()` 只存在于 `StatefulInteractiveElement`（要求元素有 `ElementId`）。给它们加 role 必须把 `new()` 改成 `new(id)` 并同步所有调用点，属于独立一刀，不适合混在 additive 批次里。`Sheet` / `Tree` / `resize_handle` / `Calendar` 同理待查。

### P2-1　`motion::Transition` 与 `animation::Transition` 同名不同物

两个 pub 类型同名：一个是值过渡策略（新，推荐），一个是元素动效组合器（legacy）。`motion.rs` 的文档虽然专门解释了区别，但 `use gpui_base::Transition` 在根导出的是 motion 那个，`animation::Transition` 要走模块路径。同名是持续的困惑源，legacy 那个建议改名 `animation::EffectTransition` 或直接标 deprecated。

### P2-2　`Collapsible` 结构体与 trait 同名

`lib.rs:81` 导出结构体 `Collapsible`，`lib.rs:84` 同文件导出 `component_traits`（里面也有个 `Collapsible` trait，只是没在根重导出）。`crates/ui` 侧把 trait 重导出成 `crate::Collapsible`，两个 crate 的同名符号指向不同东西。

### P2-3　`Disableable` / `Collapsible` trait 零实现者

base 里 `impl Disableable for` 出现 **0 次**、`impl Collapsible for` **0 次**，只有 `impl Selectable for Button` 一处。而每个控件都有自己的 inherent `disabled()`。这两个 trait 目前纯粹是给 `crates/ui` 兼容用的空壳，放在 base 的 `component_traits` 里名不副实。

### P2-4　文档与代码已经漂移

- README 和 `BASE-TODO.md` 都写着"Base Input 是刻意的最小样式例外，拥有 1px 语义边框和语义圆角基线"。**代码里没有** —— `input/mod.rs` 的 `Input` 完全无样式，`render` 只做了 role + children + `refine_style`。这条"例外"要么补回代码，要么从文档删掉（倾向后者，删掉更符合库的整体契约）。
- `crates/base/locales/` 是**空目录**，`Cargo.toml` 里也没有 `rust-i18n` 依赖。
- `Cargo.toml` 是 `publish = false`，但 README 顶部挂着 crates.io / docs.rs 徽章和 `gpui-base = "..."` 安装说明。
- README 引用 `../../specs/BASE-TODO.md`，而该目录已改名为 `docs/`，链接失效。

### P2-5　`crates/ui` 回接覆盖率

已真正走 base 的：Checkbox、Switch、Button、Toggle、Collapsible、Slider、Calendar、Select、Combobox、HoverCard、Sheet、DatePicker、Pagination、Avatar、Progress、OtpInput、NumberInput、Tree、Popover、Accordion、Tab、Tooltip、Toast（notification）、Table。

尚未回接：

- **Link** —— `link.rs` 对 `gpui_base` 零引用
- **Radio** —— 只用了 `RadioGroup`，控件本身还是旧实现。

  **2026-08-12 进展：** base 侧缺口已补齐（`Radio::track_focus` 让 ui 能用同一个 handle 画焦点环，`Radio::set_position` 提供 set 位置）。但委托本身卡在两处需要决策的语义差异，不宜静默改：

  1. **点击已选中项的行为**：`ui::Radio::handle_click` 无条件上报取反值，点击已选中项会发出 `false`；`base::Radio` 在 `checked` 时根本不挂载 `on_click`，是无响应。`RadioGroup` 内部忽略该 bool 所以组内无影响，但**独立使用的 `ui::Radio` 行为会变**。
  2. **a11y 属性**：`ui::Radio` 用 `aria_selected`，`base::Radio` 用 `aria_toggled`，对读屏软件可见。

  **决策与落地：**

  1. **点击已选中项** —— 采纳 base 的"无响应"语义。radio 本就不应自我取消，`ui::Radio` 原先无条件上报取反值更接近缺陷。`RadioGroup` 内部忽略该 bool，故组内零影响；独立使用的 `ui::Radio` 点击已选中项不再回调。
  2. **a11y 属性** —— 在 `base::Radio` 上补 `aria_selected`，与 `aria_toggled` **同时发出**。一个 radio 语义上既"被切换"又"被选中"，不同读屏软件读取其一；这样既保住 `ui::Radio` 原有输出，又不为单一消费者新增开关 API。

  为此在 base 补齐三个能力：`track_focus`（ui 用同一 handle 画焦点环）、`set_position`（set 位置）、`id`（`RadioGroup` 在构造后按序号改写 id，否则组内所有 radio 会共用一个元素 id）。

  `ui::Radio` 的 `base` 字段由 `Div` 换成 `BaseRadio`，与 `ui/tab/tab.rs` 的委托模式一致——`InteractiveElement` 仍路由到该字段，调用方装的交互不会丢失。角色、aria、焦点、激活全部下沉到 base，ui 只保留视觉与布局。

  **验证（2026-08-12）：** 人工确认组内逐项独立响应（`id()` 生效，未退化为共用元素 id）、独立用法点击未选中项正常回调、焦点环、disabled、带 tooltip 的 radio 均正常；覆盖 `radio_story`、`group_box_story`、`tooltip_story`。同时确认 P2-1 改名后 Tooltip 进入/切换动画与 Sidebar 展开/收起动画仍正常。

这跟 `BASE-TODO.md` 的记录一致，属于已知在途项。`menu/`、`dock/`、`text/` 整个子系统对 base 零引用 —— dock 已被明确划为 UI-only，menu 还在 TODO 里。

---

## 四、与 Base UI 的对照

| 维度 | Base UI | gpui-base | 评价 |
| --- | --- | --- | --- |
| Root / Part 拆分 | 严格 | 部分严格（Switch / Checkbox / Progress / Table），部分单体（Calendar / Tree / Select） | 不统一 |
| 受控 / 非受控 | 统一 `value` + `defaultValue` | 四种模型 | **差距最大** |
| 命名权威 | — | 遵循 Base UI（`SliderIndicator` 而非 Radix 的 `SliderRange`），`BASE-TODO.md` 有明文规则 | 做得好 |
| Positioner | 单一 | 两套 | 需合并 |
| 可访问性 | 完整含关系属性 | 单元素属性齐、关系属性缺 | 部分受 GPUI 限制 |
| 无样式 | CSS 层零输出 | base 层零视觉字面量 | **做得好** |

---

## 五、建议的推进顺序

~~1. **定死样式优先级契约**（P0-1）~~ —— 已完成，见上。
~~2. **修 `Combobox::on_confirm`**（P0-2）~~ —— 已完成，见上。

3. **写一份 `PRIMITIVE-CONTRACT.md`** —— 下一步，且必须先于第 4 步。把这几条钉死：状态所有权规则、回调命名、`styles()` 优先级、必须提供的 a11y 属性、part 拆分标准、no-style 边界。当前问题的本质是没有这份文档，每个组件都在重新做决定。P0-1 已经把样式优先级从「重复 10 遍的约定」变成了「单点定义的函数」，其余条款也应尽量做成可被测试断言的形式（配套 `crates/base/tests/contract.rs`），否则契约会重新漂移。
4. **统一回调形态**（P1-2）—— `on_change(value, &ClickEvent, window, cx)`；`Switch::on_toggle`、`AccordionTrigger::on_toggle` 保留为 deprecated alias 一个周期。影响面实测：ui 侧 5 处 + story/examples 约 20 处，均为机械替换。
5. **状态所有权收敛**（P1-1）—— 以 `Popover` 为模板；`XxxState` 专指 `Entity<T>` 持有的可变状态，纯计算模型改用 `XxxModel` 后缀。
6. **补 a11y**（P1-4）—— 先做 GPUI 已支持的部分（`aria_row_count` / `aria_active_descendant` / `aria_position_in_set` 等），关系属性（`aria_controls` / `labelledby` / `disabled` / `modal`）上游缺失，不要在 base 里造轮子。
7. **合并 Positioner**（P1-3）—— 风险最高，`Anchor` 涉及 23 个文件。建议独立 PR，并在 showcase 补一个"贴四条窗口边"的弹层页作为目视门禁。注意这不只是去重：`Popup` 目前只有 `snap_to_window_with_margin` 的滑动、没有真正的翻转，而 `tooltip.rs` 的 `tooltip_position()` 已有完整翻转 + clamp，合并方向是把后者推广。
8. **重新审视 Calendar / Tree 的定位** —— 要么下沉成 headless model（像 `ColorPickerState` 那样，元素交给 ui / registry），要么在契约文档里明确承认"complete primitive"这个第三类，别混在无样式控件里。
9. **清理文档漂移**（P2-4）—— 低成本高信噪比，可随时穿插。
