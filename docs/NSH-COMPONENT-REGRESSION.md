# NSH Component 绑定回归记录

本报告记录保留并继续开发的 Component 路线，与并行的 Base 路线分开。验证对象是
真实 NSH checkout 的 typed native 注册、自动生成的 `nsh/ui/gpui` 模块及原有
GPUI Component 渲染器；不使用 NSH vendor，也不经过 `Component::view` 重绘控件。

以下运行以本地 macOS 的 `crates/nsh/target/debug/examples/verify` 为准。首轮全部
组件族迁移后的 binary mtime 为 `1792341246`；menus/surfaces 已在包含最新共享 NSH
修复的 `1792342020` binary 再次完整通过；overlays 在 `1792342339` binary 完整通过。
各 fixture 直接导入生成模块，
Probe 和内部 Slot 仍通过原生测试传输注册。表中通过只表示明确列出的路径通过，
不表示每个 part、Rust builder、provider、系统服务及各平台行为均已等价。

最终默认全族串联已在 `1792344204` binary 中 exit 0，同一 AppView 依次执行
基础路径和全部 family，包括最终 Button/Checkbox/Radio/Switch logical tabIndex 修复。
已核对 `target/nsh-component-all-families-4204.log`；专项亦 exit 0，日志
`target/nsh-component-focus.log`。这不是将各自独立运行结果拼接成串联结论。
此前 `1792343668` 是该修复前已通过的历史版本，不作为最终代码证据。

## 本轮结果

| 范围 | 当前结果 | 实际验证与证据 |
| --- | --- | --- |
| 菜单 | 通过 | `NSH_VERIFY_FAMILY=menus` exit 0；AppMenuBar Tab/键盘 action、禁用顶层、双 owner 接管/恢复、外部宿主接管，PopupMenu 禁用跳过、live slot、items 更新、焦点恢复及受控关闭；`/tmp/nsh-typed-menus-verify.log` |
| Surfaces | 通过 | 最新 `1792342020` binary 整套 exit 0；Popover 指针/Space/Escape/受控关闭、Dropdown/ContextMenu/禁用子菜单/键盘选择、自定义 trigger Tab、HoverCard、Tooltip 出现/消失、NativeMenu label 与重复 action id 拒绝均通过；`/tmp/nsh-typed-surfaces-verify.log` |
| Display | 通过 | 全 typed fixture exit 0，涵盖 Tab/Shift-Tab、禁用跳过、Radio/ToggleGroup/ButtonGroup、重排后焦点、Rating owner 拒绝/接受、Clipboard、split Dropdown 及打开菜单时卸载；`/tmp/nsh-component-typed-display.log` |
| Collections | 通过 | 全 typed fixture exit 0；List/Tree/Command 搜索、键盘与确认、DataTable JSX cell、grouped Combobox 多选、Pagination、Table 组合 parts、ListItem；包含 Tree Tab 进入及 Command 搜索框关闭后的 Shift-Tab；`/tmp/nsh-component-typed-collections.log` |
| Layout / messaging | 通过 | 全 typed fixture exit 0；live children、Tabs 键盘和仅 active Input 参与 Tab、Accordion、Carousel、Questionnaire 提交、MessageScroller、Sidebar；`/tmp/nsh-component-typed-layout.log` |
| Dock / resize | 通过 | 全 typed fixture exit 0；live slots、panel actions、zoom、save/restore、原生 tab/splitter 拖动、ResizeHandle，以及 Input 编辑与光标状态保留；`/tmp/nsh-component-typed-dock.log` |
| Inputs | 通过 | 全 typed fixture exit 0；Textarea/Editor 编辑、controlled caret/echo、语言切换与 undo、Number、OTP、rich token 删除与双次 echo、InputGroup mode 切换及异步 completion 原生菜单接受；`target/nsh-generated-inputs.log`。未逐项运行所有 InputGroup part/provider/IME 场景 |
| Select / Slider 基础路径 | 已通过对应断言 | 生成模块入口的选择、重排、search 切换、指针 change/release、owner reset 无回声及 unmount 通过；默认 `1792343668` 串联实际通过，含这些基础路径 |
| Date / time | 通过 | 最新 `1792343153` binary 的 datetime fixture exit 0，包含受控日期/时间替换、disabled、分段键盘编辑、range/preset 与 owner veto；`target/nsh-generated-datetime.log` |
| Stepper | 通过原有对应断言 | 最新 `1792343153` binary 的 extra_controls 整族 exit 0，原有 live slot、指针与禁用断言全部保留；`target/nsh-generated-extra-controls.log` |
| Color | 通过 | 同一 extra_controls fixture 完整通过真实 palette/anchor、HSLA/alpha、commitHex 关闭且无回声、真实 hex Input Enter，以及 Picker/Select owner veto 后恢复；owner 已查看 `target/nsh-verify/extra-controls.png` |
| Content / chart / Plot / speech | 通过对应场景 | 最新 `1792342020` binary 的 content fixture exit 0，19 场景覆盖全部 16 注册名、Diff 点击、Markdown block/inline live slots、原生 Plot shapes/scales/axes；`/tmp/nsh-component-typed-content.log`。Speech 仅 idle/unsupported 展示，不证明麦克风或真实识别 |
| Overlays | 通过 | `1792342339` binary 的 overlays family 完整通过；fixture 使用真实 dispatch 卸载并同步退出动画与 fake clock，不以删掉焦点断言获得通过 |
| AlertDialog / overlay stack / Dialog parts | 通过 | 默认 `1792343668` 串联通过 native header/footer、confirmation veto、close reset、sheet/dialog opener 与 stale handle，以及 DialogAction/DialogClose Enter/Space、单次事件和 scoped host protection |
| 四控件 logical tabIndex | 专项通过 | `1792344204` binary exit 0：Button2→Checkbox3→Radio0→Switch0→wrap2 的五次真实 Tab；四个 negative/tabStop=false 控件各一次 pointer、ref focus 和 Space 激活。日志 `target/nsh-component-focus.log`；亦包含于最终 4204 全族串联 |

## 同业务对比示例

Component compare 已用 `1792343153` binary 加载真实 CLI 构建的原 bundle，
完整通过指针创建、Input/Checkbox reset、Button disabled/refocus、Enter 创建及
80 字符 veto。日志 `/tmp/nsh-component-compare-verify.log`；owner 已逐张查看
`target/nsh-component-compare/screenshots/` 的 initial/before/after/keyboard/limit，
并与 Base 示例实图对照相同业务与文案：Created 1 task / Review NSH controls · Priority，
Created 2 tasks / Check keyboard submission · Standard。Component 保留原控件皮肤；
Base 的应用外观由 NSH 组合。该示例通过不代替所有独立控件和平台行为验收。

## 菜单与 Tooltip 图像核对

本轮已查看 `target/nsh-verify/menus-popup.png`：两个禁用项显示为禁用，真实 NSH
自定义菜单行和普通 action 均完整显示。已查看 `target/nsh-verify/tooltip.png`：
原生 Tooltip 中同时显示文本及真实 NSH live content；随后 fixture 断言该内容消失。
Tooltip 验证使用其真实子树的可见性，没有把未注册的测试 Role 查询当作唯一依据。

NativeMenu 的 OS menu tracking 需要真实平台窗口。本离屏 fixture 的末路径验证
针对非法重复 action id 的拒绝，不声称完成 OS 原生菜单弹出、点击或跨平台人工验证。

## 类型与覆盖边界

本轮 CLI 类型契约已通过：消费 152 个生成组件值、326 个 schema 声明及兼容入口
的 64 个旧类型名，并检查 readonly/as-const、日期、颜色与 Stepper 类型。
产物目录为 `target/nsh-typed-typecontract`。这是类型生成与消费证据，不能代替 GUI。

已核对 `target/nsh-shared-final-tests.log`：Core 283 passed / 3 ignored，JS 99 passed / 1 ignored，
typed native 集成 3 passed。backend runner 3 项与 tab priority stable sort 1 项亦实际通过，
日志分别为 `target/nsh-shared-runner-tests.log` 和 `target/nsh-shared-tab-priority-test.log`。
父 Kit 默认 Pre 后端的专项焦点回归亦 1 项通过：`target/nsh-parent-kit-focus-test.log`。
这些测试覆盖共享协议与生命周期，不代替 Component GUI。
最终 fmt check 与严格 Clippy（all targets，cli/test-support/dev，no-deps，`-D warnings`）
已通过，日志 `target/nsh-component-final-clippy.log`；最终 4204 binary 的专项和串联 GUI 亦已实际通过。

catalog 继续保持 `partial`：已有可用绑定，但完整 Rust API 逐项等价仍未证明。
MenuItem 的递归值、禁用、深度与 id 唯一性由单一 Rust schema 和 domain 校验负责；
生成对象类型尚未完全恢复旧手写 separator/action/submenu union 的所有静态互斥约束。
PopupMenu items、AppMenuBar menus 及 ApplicationMenu items 保留 readonly 输入兼容。

所有本轮 domain 校验通过通用 `validate_props` 在 candidate 接受前执行；非法 batch
保留上次 accepted props，避免后续无关更新被已拒绝值污染。默认跨族串联及新修复专项已实际通过；release 优化及 Windows/Linux GUI 仍需
分别记录，不能由这一 macOS 串联推断。
