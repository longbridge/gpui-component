# NSH Base 代表绑定回归记录

当前 Base 路线仅包含 Button、Checkbox、Input 三个代表绑定，以及组合和任务创建示例。
这不是全部 gpui-base 的覆盖结果，也不表示三个控件的全部行为已经通过。
本报告与 [Component 回归记录](NSH-COMPONENT-REGRESSION.md) 分开记录。

## 实现边界

`crates/nsh-base` 注册 `nsh/ui/base`。Props/Event 的 Rust 声明通过同一 NSH
Registry 生成 JavaScript 导出和 TypeScript 声明，没有另维护一份 TS props schema
或控件运行 shim。普通全局 `nsh` 不会自动发现自定义 UI set；宿主和对应的薄 CLI
必须注册它。NSH 使用同级真实源码 checkout，不使用 vendor 快照。

渲染调用真正的 `gpui_base::Button`、`Checkbox` 和 Input。NSH 保留 signal、
动态 JSX 子树与外观布局；Base 提供原生交互、焦点及文本编辑行为。依赖树产物
`target/nsh-base-dependency-tree.txt` 中没有 gpui-component 或 gpui-shell。
这不表示整张依赖图只有一个 GPUI 包，也不改变 Component 作为独立路线保留的事实。

单源 schema 不等于没有 Rust 适配成本：Input 仍需保留 InputState Entity、
订阅、光标/选区与受控同步；按钮和 Checkbox 仍需真实 FocusHandle、焦点事件与
生命周期处理。通用桥接承担 codec、注册、生成、受控 ack 和 live subtree，
不能靠自动生成声明推断这些原生状态语义。

## 已测与待测

运行环境为本地 macOS 离屏 GPUI、真实 NSH 引擎。以下记录具体 binary 和路径，
不将旧 slice 的成功扩大为后续新增场景或整套通过。

| 范围 | 当前结果 | 实际证据与未完成项 |
| --- | --- | --- |
| Button 基础交互 | 已通过对应 slice | 指针恰好一次 press、蓝色背景像素、白色 NSH Text、240×44 布局；Enter/Space、Tab 跳过禁用项、Shift-Tab、禁用当前 focused 控件后不响应及恢复、精确 Focus/Blur 已通过。图片 `target/nsh-base-verify/button.png`、`button-before.png` 已由 owner 目视核对 |
| Button 扩展焦点顺序 | 已通过对应 slice | 最新 `1792342635` binary，button family exit 0；正 tabIndex 2→3→0 及 Shift-Tab 通过，tabStop=false/负 index 被 Tab 跳过、仍允许指针聚焦后 Space/Enter 激活。原基础断言保持通过；最新 `1792343039` binary 还通过 ref focus、非法 disabled 候选恰好一次 onError/保留 last good、focused Enter 自卸载后无旧回调及重挂恰好一次 press。owner 已查看 `button-recovery.png` |
| Checkbox | 当前扩展 suite 通过 | 最新 `1792343039` binary，`NSH_BASE_VERIFY_FAMILY=checkbox` exit 0；保留原八次 Change、pointer/Enter/Space、controlled accept/veto/mixed、Focus/Blur、动态子背景像素，并通过 disabled Enter、disabled Tab wrap、focused self-unmount、旧位置 pointer/Space/Enter 无 stale event、remount 恰好一次触发，合计精确十次 Change。日志 `target/nsh-base-checkbox.log`。Checkbox 未单独测试正 tabIndex，Button 的结果不移作该项通过 |
| Input | 当前 suite 通过 | 最新 `1792343695` binary，input family exit 0；echo/veto 与 caret、readonly/disabled/recover、ref focus/selectAll/Enter payload、tabStop=false/负 index 的输入入口、正 index 2/3/0 正反 Tab 和原生 exclusive focus、NSH 祖先 font/color/padding 的响应式 glyph 像素均通过。日志 `target/nsh-base-input-final.log`，owner 已目视 input-echo/navigation/style-before/style-after 图像。系统 IME 兼容性仍不由这些输入入口模拟断言推断 |
| Input reconciliation | 当前 suite 通过 | 最新 `1792343471` binary，input_reconciliation family exit 0；external reset 中间 caret veto 后接受 Q 得到 wxQyz；Unicode 甲乙丙丁选中乙丙、veto Z、接受 K 后得到甲K丁；同 batch 两个非法属性加合法 disabled 请求，恰好一次 error、原生值/焦点/启用状态保留，恢复后 R 精确插入为甲KR丁。日志 `/tmp/nsh-base-input-reconciliation-3471.log`，截图 `target/nsh-base-verify/input-reconciliation.png` |
| Input lifecycle | 当前 suite 通过 | 最新 `1792343695` binary，input_lifecycle family exit 0；接受 X 后修改无关 NSH style/placeholder，真实 Cmd-Z 回到 abc；聚焦输入 Q 后自卸载，原位置 pointer/IME/backspace、旧 ref focus/selectAll 均无 stale change，remount R 恰好第四次 Change，error 为空；`target/nsh-base-input-lifecycle.log`、`target/nsh-base-verify/input-lifecycle.png` |
| 嵌套原生组合 | 当前 suite 通过 | 最新 `1792343039` binary，composition family exit 0；outer 320×120 内原生 Base Button 160×56、背景像素/containment/精确一次 press 保留；新增交互 NSH 子 View 仅 child 一次、outer 零次，以及测试 native host 的 named header/default body labels 0→1→2、卸载 query 消失且旧坐标无回调、remount 后 2→3 恰好一次。owner 已查看 `composition.png`、`composition-live.png`、`composition-unmounted.png`。测试 host 不计作新增公开 Base 控件 |
| Create task 示例 | 通过 | 最新 `1792342635` binary，`NSH_BASE_VERIFY_FAMILY=task_app` exit 0；加载真实原 bundle，无 probe/测试 hooks，以公有 native snapshots 验证鼠标创建、Input 清空、Checkbox 复位、Button 再次禁用、焦点恢复、Enter 提交及 80+1 字符 veto。日志 `target/nsh-base-task-app-verify.log`；owner 已逐张查看 `task-before.png`、`task-after.png`、`task-keyboard.png`、`task-limit.png`，实际显示 Created 1 task / Created 2 tasks 及正确内容 |

## 生成类型契约

fresh CLI `1792343353` 已实际 check `target/nsh-base-typecontract/App.tsx`：
三个控件的全部已声明 props/events、host style、readonly children、slots 与
ref.command 消费通过，并保留显式 JSX.ArrayElement.push 的兼容性。
16 个 `@ts-expect-error` 均实际消费；移除这些 directive 后 check 失败且恰有
16 个 TypeScript 错误，恢复后再次 check 成功。日志目录
`target/nsh-base-typecontract`，负控为 `negative-proof.log`。

声明由该 CLI 从 Rust schema 重新生成，没有手写类型补丁或 cast。
这里的“全部”仅指这三个代表绑定已经声明的 API，不表示全部 gpui-base API。
类型契约不能代替后续实际 GUI、生命周期和受控同步验证。

## CLI、HMR 与包资源

fresh `1792343039` runner/薄 CLI 的实际工作流已通过：生成并 check 原示例的
`nsh/ui/base` 声明、标准 run 加载 build 1、源编辑后 build 2 获得真实引擎
hot-update ack、标准 package，以及包内 runner 从 `/tmp` 启动。移走包自身的
bundle 后，同一 runner 启动失败，恢复资源后保留原包，验证了包资源定位。
日志为 `target/nsh-base-runtime-workflow/verification.log`，可复验入口为
`python3 -B crates/nsh-base/examples/verify_workflow.py`。

这里的标准命令由注册 Base UI set 的薄 CLI 复用 NSH CLI 执行，不声称普通全局
`nsh` 自动发现该 set。验证使用独立 runtime/sample 副本，没有改全局 NSH_RUNTIME。
这是 macOS 的 CLI/HMR/资源证据；HMR ack 不证明 signal 状态跨更新保留，GUI
任务结果另由 task_app 验证，也不推断 Windows/Linux 或 release 构建通过。

可复验入口为 `script/test-nsh-base-ui`：构建同一批 Base host/CLI/verify，
用对应 CLI 构建真实 TSX 示例 bundle，再运行默认七个 family：button、checkbox、input、input_reconciliation、input_lifecycle、
task_app、composition。
`1792343695` binary 的默认 all 已在同一 AppView 依次热载并通过全部七个 family，
不是仅将独立进程结果拼接。`script/test-nsh-base-ui` 已实际 exit 0，包含最终 bins rebuild、原 TSX bundle 编译
与全部七 family UI；证据为 `target/nsh-base-final-script.log`。runner 单测另有 3 passed。
最终 fmt check 与严格 Clippy（all targets，dev/cli/test-support，no-deps，`-D warnings`）亦已实际通过，日志 `target/nsh-base-final-clippy.log`。父 Kit 集成检查通过，日志 `target/nsh-parent-kit-integration-check.log`；父 workspace lockfile 未变化。

Base 默认七 family 串联已通过。Component 的 152 值/326 schema 类型契约不能挪作
Base 的验收；上述结果仍限定当前三个代表绑定与实际断言。release 优化、
Windows/Linux GUI、系统 IME 与其他平台行为未由这些 macOS 运行证明。

## 中文输入法确认回归

Base Input 的 `unmark_text` 现在在确认已有 marked composition 时发布一次
`InputEvent::Change`。此前 `n → ni → 你 → unmark_text` 只改变原生文本，NSH
订阅者收不到最终值。新增回归在修复前得到 `[]`，修复后得到 `["你"]`；同时检查
重复 unmark、insertText 后 unmark、取消预编辑、只读和禁用输入。

```sh
cargo test -p gpui-base --features test-support composition --lib --locked
```

本轮 10 项 composition 测试和 Base 七 family UI 套件均通过。这里覆盖输入法协议
与事件订阅，不宣称已经自动操作 macOS 系统输入法候选框。
