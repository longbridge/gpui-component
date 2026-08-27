---
title: API 参考
description: 脚本能 import 或触及的每个名字——三个内置模块、cx 与 window 全局对象，以及那些不是样式的元素方法。
order: 13
---

# API Reference

脚本接口的一份清单：有什么，以及它来自哪个模块。其余页面解释每样东西为什么是这个样子——这一页是用来查名字的。

权威不在这一页。`gpui-shell` 每次运行都会在你的源码旁边重写一份 `gpui.d.ts`，它由即将执行这段脚本的那个运行时生成，所以提交进仓库的副本只可能是过期的那一份。在脚本顶部写上 `// @ts-check`，编辑器就会照着它检查。

## 模块

提供能力的每个 crate 对应一个模块，所以一条 import 就说明了脚本依赖哪一层。一个名字只属于其中一个模块：这里不为了方便做任何 re-export，因为一个能从两个 specifier 取到的名字，就不再说明它来自哪里。

| 模块 | 提供 |
| --- | --- |
| `"gpui"` | GPUI 自己的元素，加上这个运行时补上的部分：视图、样式接口、存储、调度、native 模块 |
| `"gpui-base"` | `gpui-base` 的布局辅助函数、组件与主题 |
| `"gpui-fps"` | `gpui-fps` 的性能 HUD |

有两个名字是 ambient 的，完全不需要 import：每次宿主调用都会交给你的 `cx`，以及 `window`。标准运行时模块——`fs/promises`、`path`、`crypto`、`process`、`net`、`websocket` 等等——受宿主授权门控，记录在 [Capabilities](./capabilities.md)。

下面每张表里的名字，就是你实际写下的那个。小写函数在 Rust 侧同样是自由函数——`div()` 就是 `gpui::div()`。首字母大写的名字是一个只有工厂方法的对象，镜像它同名的关联函数：`Button.new(id)` 就是 `Button::new(id)`，而 `TableRow.new(id, index)` 带的是屏幕阅读器要念出的、从 1 开始的位置。

## `gpui` 模块

### 元素

| 名称 | 说明 |
| --- | --- |
| `div()` | 自身不带布局的元素 |
| `svg(path)` | 来自应用根目录的矢量图，按周围的文字颜色着色 |
| `image(path)` | 来自应用根目录的全彩图片，保留原色 |
| `PathBuilder` | `fill()` 与 `stroke(width)`，各自开启一条正在构建的路径 |
| `Background` | `solid`、`stop`、`linear_gradient`、`pattern_slash`、`checkerboard` |

`PathBuilder.fill()` 与 `.stroke(width)` 返回一个句柄，可链式调用 `move_to`、`line_to`、`curve_to`、`cubic_bezier_to`、`arc_to`、`add_polygon`、`close` 与 `dash_array`，最后以 `build()` 收尾。用 `window.paint_path(path, background)` 把结果画出来——它是唯一一个通过对象取到的元素构造器，因为它镜像的东西在 Rust 侧就是窗口上的一个方法。

字符串本身也是元素，和 GPUI 里 `&str` 实现 `IntoElement` 完全一样：`.child("hello")` 就是写文本的方式，样式来自持有它的那个元素。

### 视图

| 名称 | 说明 |
| --- | --- |
| `View` | 每个视图的基类；继承它，并把子类作为 default export |
| `ViewClass` | 一个具体的 `View` 子类，也就是 `cx.new` 接受的东西 |
| `Entity` | 对一个嵌套视图的留存所有权：`set_props(props)`、`release()` |
| `Props` | 交给 `init` 与 `cx.new` 的属性包 |

子类定义只执行一次的 `init?(props, cx)`，以及返回恰好一个元素、在视图被置为失效时执行的 `render(cx)`。可选的 `update(props)` 在父视图改变嵌套视图的 props 时执行。

### 存储

| 名称 | 说明 |
| --- | --- |
| `store` | 能挺过重启的键值存储，每次写入都会持久化 |
| `Store` | `get(key)`、`set(key, value)`、`remove(key)`、`keys()`、`flush()` |
| `Json` | store 能持久化的全部内容，仅此而已 |

未设置的键 `store.get` 返回 `null`；`flush()` 在当前值被可靠写入之后完成。

### Native 模块

| 名称 | 说明 |
| --- | --- |
| `native(module)` | 宿主在 Rust 侧注册的模块；一个都没找到时抛异常，并列出存在的那些 |
| `NativeModules` | 这里是空的——应用把自己的模块声明进去，`native("…")` 就有了类型 |

### 调度

| 名称 | 说明 |
| --- | --- |
| `Task` | 一个正在运行的任务：`cancel()`、`is_done()` |
| `TaskOptions` | `owner`——任务随之取消的那个视图，或 `null` 表示比任何视图都活得久 |
| `Timer` | `after(ms, handler, opts?)` 与 `every(ms, handler, opts?)` |

### 焦点与组件形态

| 名称 | 说明 |
| --- | --- |
| `FocusHandleHandle` | 脚本自己持有的焦点目标：`focus()`、`is_focused()`、`release()` |
| `ComponentType` | `new(id)`——跨渲染有身份的组件 |
| `PartType` | `new()`——自身没有身份的子部件 |
| `IndexedComponentType` | `new(id, index)`——会报读自身从 1 开始位置的组件 |

### 共享类型

| 名称 | 说明 |
| --- | --- |
| `Length` | 数字（像素）、`"12px"`、`"1.5rem"`、`"50%"` 或 `"auto"` |
| `DefiniteLength` | 同上，但不含 `"auto"` |
| `AbsoluteLength` | 只有像素或 rem |
| `LengthString` | 长度的字符串形式 |
| `Color` | 一个 `ColorToken` 名字，或 `#rgb` / `#rrggbb` / `#rrggbbaa` 字面量 |
| `ColorToken` | 已安装调色板定义的十七个语义 token |
| `Role` | 一个无障碍 role，镜像 `gpui::Role` 的 snake_case 拼写 |
| `Anchor` | 锚定浮层的哪个角固定在它的触发元素上 |
| `MouseButton` | `"left"`、`"right"` 或 `"middle"` |
| `Phase` | `"render"`、`"event"`、`"task"`、`"layout"` 或 `"none"` |
| `SheetSide` | sheet 贴靠哪一边 |
| `DialogOptions` | `escape_dismissable`、`backdrop_dismissable` |
| `ToastOptions` | `title`、`description`、`level`、`timeout`、`id` |
| `ClickEvent` | `click_count`、`modifiers` |
| `MouseMoveEvent` | `position`、`local_position`、`bounds`、`modifiers` |
| `Modifiers` | `shift`、`control`、`alt`、`platform` |
| `Point` | `x`、`y` |
| `ElementBounds` | 带 `width` 与 `height` 的 `Point` |
| `MotionProperty` | `"opacity"`、`"width"`、`"height"`、`"left"`、`"top"` |
| `MotionEasing` | `"linear"`、`"ease-in"`、`"ease-out"`、`"ease-in-out"` |
| `TransitionPolicy` | `duration`、`delay`、`easing` |
| `SpringPolicy` | `response`、`damping`、`epsilon` |
| `Path` | 由 `PathBuilder.build()` 产出的不可变原生几何 |
| `PathCoordinate` | 像素，或所绘元素边界的百分比 |
| `BackgroundValue` | 可复用的原生背景：`opacity(factor)`、`color_space(space)` |
| `BackgroundStop` | 一个渐变色标，来自 `Background.stop(color, percentage)` |

## `cx` 上下文

`cx` 是某一次宿主调用的脚本侧 context，并且只在那次调用中有效。`await` 会把控制权交回宿主，它指名的那一帧随之消失，所以跨越 `await` 留住的 `cx` 会报出 stale-context 错误。

| 成员 | 说明 |
| --- | --- |
| `notify()` | 请求重新渲染；在 `render` 期间抛异常，因为渲染中通知自己是一个死循环 |
| `phase()` | 这次调用处于哪个 `Phase` |
| `theme()` | 当前 `gpui_base::Theme` 的语义 token 投影 |
| `open_url(url)` | 把一个绝对的 `http`/`https` URL 交给系统处理器 |
| `read_from_clipboard()` | 剪贴板里的文本，没有文本时是 `undefined` |
| `write_to_clipboard(text)` | 替换剪贴板里的文本 |
| `focus_handle()` | 一个新的 `FocusHandleHandle`；属于 `init` 或事件处理器，绝不属于 `render` |
| `new(Class, props?)` | 创建一个留存的嵌套视图，并返回拥有它的 `Entity` |
| `spawn(body, opts?)` | 执行 `body(cx)` 并接管它返回的 promise，让 rejection 得到上报 |
| `sleep(ms?)` | 在 GPUI 的 foreground executor 上，`ms` 之后 resolve |
| `timer` | `Timer`：`after` 与 `every` |

其中好几个都指名了它所镜像的 GPUI 方法：`open_url` 是 `App::open_url`，`read_from_clipboard` 与 `write_to_clipboard` 是 `App::read_from_clipboard` 与 `App::write_to_clipboard`，`focus_handle` 是 `App::focus_handle`（GPUI 没有 `FocusHandle::new`，这里同样没有），`new` 是 `AppContext::new`，`spawn` 是 `App::spawn`。

### `AsyncContext`

`AsyncContext` 继承 `Context`，不增加任何成员。区别在生命周期，不在接口：普通的 `Context` 只为一次宿主调用发言，一旦那次调用返回就明确报错；而 `AsyncContext` 不指名任何一次调用——用到它时才解析当时正在执行的那一次，只有在一次都没有时才拒绝。它对应 GPUI 的 `AsyncApp`。

有三处会交出一个：`init`、`cx.spawn` 的 body，以及 `cx.timer` 的回调。这三处的职责正是「安排或延续比启动它的那次调用活得更久的工作」。

## `window` 全局对象

和 `cx` 一样是全局的，不需要 import。每次调用都读取当前正在跑的那次宿主调用，不在任何调用中时抛异常，所以没有句柄要持有，也没有东西会过期。浮层属于窗口，而不属于打开它的那个视图——这就是这些方法在这里、而不在 `Context` 上的原因。

| 成员 | 说明 |
| --- | --- |
| `open_dialog(content, options?)` | 打开一个 dialog，并返回栈的新深度 |
| `close_dialog()` | 关闭最上层的 dialog，并回答有没有找到 |
| `close_all_dialogs()` | 关闭所有 dialog，并回答关掉了几个 |
| `has_active_dialog()` | 是否有 dialog 打开；与其余方法不同，它在 `render` 中合法 |
| `open_sheet(content)` | 在右侧打开 sheet，替换掉原本在那里的内容 |
| `open_sheet_at(side, content)` | 同上，贴靠你指定的那一边 |
| `close_sheet()` | 关闭 sheet，并回答原本有没有打开 |
| `has_active_sheet()` | sheet 是否打开；在 `render` 中合法 |
| `push_toast(options)` | 弹出一个 toast，并返回它的 id |
| `remove_toast(id)` | 撤回一个 toast，并回答它当时是否还在显示 |
| `clear_toasts()` | 撤回所有 toast，并回答撤回了几个 |
| `paint_path(path, background)` | 用原生背景绘制不可变几何；对应 `Window::paint_path` |

`open_dialog`、`open_sheet` 与 `open_sheet_at` 接受的是**一个返回元素的函数**，而不是元素：dialog 活得比打开它的那次调用久，每次重绘时这个函数都会再执行一次。除了两个 `has_active_*` 查询与 `paint_path`，这里的一切在 `render` 中都不合法。见 [Overlays](./overlays.md)。

## `gpui-base` 模块

这里的组件拥有行为、焦点，以及屏幕阅读器听到的内容，而自身几乎什么都不画。画面归脚本所有，用[样式接口](./styling.md)写出来。

### 布局

| 名称 | 说明 |
| --- | --- |
| `h_flex()` | 一行 |
| `v_flex()` | 一列 |
| `h_resizable(id)` | 一行带可拖拽分隔条的窗格；尺寸按这个 id 存在窗口里 |
| `v_resizable(id)` | 同上，纵向堆叠 |
| `resizable_panel()` | 可调整组里的一个窗格，用在别处都不合法 |

### 控件

| 名称 | 说明 |
| --- | --- |
| `Button.new(id)` | 激活、焦点、disabled 与 selected 状态 |
| `Link.new(id)` | 通过系统浏览器打开的外部 HTTP(S) 资源 |
| `Checkbox.new(id)` | 受控的勾选；勾选标记自己画 |
| `Switch.new(id)` | 受控的 switch |
| `Radio.new(id)` | 一组中的一个选项；只报告 `true`，从不报告取消选中 |
| `Toggle.new(id)` | 一个会保持按下的按钮 |
| `RadioGroup.new(id)` | 被报读为一组的一批 radio；自身不持有选中项 |
| `ToggleGroup.new(id)` | 被报读为 toolbar 的一批 toggle |
| `Tabs.new(id)` | 自身不持有选中项的 tab 列表 |
| `Tab.new(id)` | 一个 tab：`selected(...)` 进，`on_click(...)` 出 |
| `Progress.new(id)` | 只有报读，没有进度条；单独的 `Progress.new(...)` 什么都不画 |
| `ProgressTrack.new()` | 凹槽：一个由你设定尺寸与颜色的普通元素 |
| `ProgressIndicator.new()` | 已填充的部分；按你报读的百分比设置它的宽度 |
| `SliderState.new(options?)` | 留存的 slider 状态，也是一次拖拽写入的地方 |
| `Slider.new(state)` | 根：报读数值，并拥有 release |
| `SliderTrack.new(state)` | 按下与拖拽的表面 |
| `SliderIndicator.new(state)` | 凹槽，也是每个指针位置据以测量的那个盒子 |
| `SliderThumb.new(state)` | 滑块；shell 给它位置，你给它外观 |

slider 的四个部件接受同一个 `SliderStateHandle`，而且四个都不能少——没有 `SliderIndicator` 的 slider 根本拖不动。

### 文本编辑

| 名称 | 说明 |
| --- | --- |
| `InputState.new(options?)` | 留存的文本状态：`InputState.new({ placeholder, value })` |
| `Input.new(state)` | 包住留存文本状态的框 |
| `NumberInput.new(state)` | 建立在同一个 `InputState` 上的 spinbutton，三个插槽都有分量 |
| `TextareaState.new(options?)` | 留存的多行文本状态；`rows` 是一个选项 |
| `Textarea.new(state)` | 包住留存多行状态的框 |
| `OtpState.new(length, options?)` | 留存的一次性验证码状态；长度在创建时固定 |
| `OtpInput.new(state)` | 定长验证码，格子由 shell 画、由脚本设定样式 |

没有专门的数字状态类型：给 `InputState` 设上 `set_step`、`set_min` 与 `set_max`，它就成了数字状态。

### 容器与浮层

| 名称 | 说明 |
| --- | --- |
| `Collapsible.new()` | 仅在 `open` 时渲染它的 `content` 插槽；不带 role、箭头或触发器 |
| `Popover.new(id)` | 锚定在触发元素上、由按下打开的浮层 |
| `HoverCard.new(id)` | 同上，但由指针停留打开，并有自己的打开状态 |
| `Popup.new(id, trigger)` | 光秃秃的锚定浮层：`Popup.new(id, trigger)`，填入 `content` 即打开 |
| `Select.new(id)` | combobox 的根：role、报读的打开状态、键盘——但不含任何画面 |
| `Combobox.new(id)` | 同一个根，被报读为一个触发器是可编辑输入框的 combobox |
| `DatePicker.new(id, focus_handle)` | 日期选择器的根：`DatePicker.new(id, focus_handle)`；它不持有日期 |

在这些之上动手之前，有两处缺口值得先知道：打开的 `Select` 或 `Combobox` 列表还没有方向键导航，而 Enter 与 Escape 到不了 `DatePicker`。两者都写在各自类型的声明里，也就是它们真正咬人的地方。

### 表格与列表

| 名称 | 说明 |
| --- | --- |
| `Table.new(id)` | 语义表格的根，组合方式与 HTML 组合表格一致 |
| `TableHeader.new(id)` | 表头行组 |
| `TableBody.new(id)` | 表体行组 |
| `TableRow.new(id, index)` | 一行：`TableRow.new(id, row_index)`，从 1 开始 |
| `TableHead.new(id, index)` | 一个列头，从 1 开始 |
| `TableCell.new(id, index)` | 一个数据单元格，从 1 开始 |
| `TableCaption.new(id)` | caption 该在的视觉位置；它不带 caption role |
| `v_virtual_list(…)` | 只描述屏幕内内容的纵向列表 |
| `h_virtual_list(…)` | 另一个轴上的同一件事；`item_sizes` 是宽度 |
| `VirtualListScrollHandle.new()` | 虚拟列表的滚动位置，跨帧保留 |
| `Scrollbar.new(id)` | `new(id)`、`horizontal(id)`、`vertical(id)`——一条由你自己摆放的滚动条 |

两种虚拟列表都接受 `(id, item_count, item_sizes, get_key, render)`。`render(range, cx)` 是这套接口里唯一由宿主在一帧*进行中*调用的回调，所以在它内部注册处理器、创建留存状态与调用 `cx.notify()` 都会被拒绝。

### 留存句柄

下面每一个都只创建一次——在 `init` 或事件处理器里，绝不在 `render` 里——并用 `release()` 释放。

| 句柄 | 成员 |
| --- | --- |
| `InputStateHandle` | `value`、`set_value`、`on("change" \| "submit" \| "focus" \| "blur")`、`set_step`、`set_min`、`set_max`、`set_masked`、`set_loading` |
| `TextareaStateHandle` | `value`、`set_value`、`on(…)`、`set_rows`、`set_auto_grow`、`set_soft_wrap` |
| `SliderStateHandle` | `value`、`set_value`、`min_value`、`max_value`、`step_value`、`on("change" \| "release")` |
| `OtpStateHandle` | `value`、`set_value`、`len`、`is_masked`、`set_masked`、`focus`、`on("change" \| "focus" \| "blur")` |
| `VirtualListScrollHandleHandle` | `scroll_to_item(index, strategy?)`、`scroll_to_bottom` |

### 主题

| 名称 | 说明 |
| --- | --- |
| `set_theme(theme)` | 用应用自己的主题替换 `gpui-base` 当前生效的语义 token |
| `Theme` | `cx.theme()` 返回的东西：语义 token，加上 `appearance` 与 `is_dark` |
| `SemanticThemeTokens` | `colors`、`spacing`、`radius` |
| `ColorTokens` | 每个语义角色一个 `Color` |
| `SpacingTokens` | `xxs` `xs` `sm` `md` `lg` `xl` `xxl` |
| `RadiusTokens` | `none` `sm` `md` `lg` `xl` `full` |

读主题用 `cx.theme()`。替换整套调色板是应用层面的动作，谈不上属于哪一次调用的 context——这就是 `set_theme` 是一个自由函数的原因。

### 其他类型

| 名称 | 说明 |
| --- | --- |
| `GroupAxis` | `"horizontal"` 或 `"vertical"`，只报读、不绘制 |
| `ScrollbarMode` | `"scrolling"`、`"hover"` 或 `"always"` |
| `ItemRange` | 虚拟列表的可见项，写作半开区间 `[start, end)` |
| `SliderValue` | 一个数字，或区间 slider 的 `[start, end]` |
| `PopupType`、`DatePickerType`、`ScrollbarType` | 构造器参数不止一个 id 的那三个类型的工厂形态 |

## `gpui-fps` 模块

| 名称 | 说明 |
| --- | --- |
| `fps_monitor()` | 原生 `gpui-fps` HUD，每个窗口共享一个，固定在右上角 |

它的父元素必须设置 `relative()`。HUD 自己拥有完整外观；普通样式与子元素对它不起作用。

## 元素方法

所有元素共享同一个 prototype，所以下面每个方法在任何元素上都能通过类型检查——某个方法实际适合哪个组件，类型并不表达。交给一个不承接它的组件的行为 builder 会被写进日志，而不是被悄悄丢掉。

每个方法都返回同一个元素，所以一条链就是一个表达式。元素被用作子元素时即被消费，并且属于构建它的那一趟渲染。

### 组合

| 方法 | 作用 |
| --- | --- |
| `child(value)` | 添加一个子元素：元素、`Entity`，或字符串、数字、布尔值 |
| `children(iterable)` | 按顺序添加多个 |
| `when(condition, branch)` | `condition` 为真时应用 `branch`，让链保持完整 |
| `id(name)` | 给这个元素一个稳定的名字，作为它的身份 |

### 插槽

插槽不是子元素：元素被组件消费，渲染在组件决定的位置。

| 方法 | 作用 |
| --- | --- |
| `content(element)` | `Collapsible`、`Popover`、`HoverCard` 或 `Popup` 的内容 |
| `trigger(element)` | `Popover` 或 `HoverCard` 的触发器 |
| `input(element)` | `NumberInput` 的编辑器插槽；留空则画出裸编辑器 |
| `decrement_button(element)` | `NumberInput` 减少按钮的外观——重放到 base 的按钮上，而不是直接渲染 |
| `increment_button(element)` | 增加按钮，重放方式相同 |
| `controls_right()` | 把两个步进按钮叠放在文本右侧 |

### 事件

| 方法 | 交付什么 |
| --- | --- |
| `on_click(handler)` | 激活时的 `(ClickEvent, cx)` |
| `on_mouse_move(handler)` | 指针悬停在元素上时的 `(MouseMoveEvent, cx)` |
| `on_hover(handler)` | 指针进入与离开时的 `(hovered, cx)` |
| `on_change(handler)` | 开关变化时的 `(checked, cx)`；新值由脚本保存 |
| `on_step(handler)` | `("increment" \| "decrement", cx)`，并且它会**取代**内置的步进 |
| `on_item_click(handler)` | 虚拟列表某一行被点击时的 `(key, cx)`，按 key 而不是按下标 |
| `on_open_change(handler)` | 脚本之外的东西改变了 `Popover` 的打开状态时的 `(open, cx)` |
| `on_confirm(handler)` | 在打开的 `Select` 或 `Combobox` 中按下回车；无参数 |
| `on_dismiss(handler)` | 在打开的 `Select` 或 `Combobox` 中按下 Escape，早于 `on_open_change(false)` |
| `on_resize(handler)` | 可调整组的拖拽结束后的 `(sizes, cx)` |

### 控件状态

| 方法 | 设置什么 |
| --- | --- |
| `disabled(value)` | 阻止激活并报告该状态；外观自己画 |
| `selected(value)` | `Button` 的 selected 状态 |
| `checked(value)` | `Checkbox`、`Switch` 或 `Radio` 的受控值 |
| `pressed(value)` | `Toggle` 的受控状态 |
| `value(percent)` | 报读的进度百分比，钳制在 `0..=100`；它不会让屏幕上任何东西移动 |
| `indeterminate(value)` | 把 `Progress` 的数值从无障碍树里撤下 |
| `open(value)` | `Collapsible` 是否渲染内容，或浮层是否正在显示 |
| `default_open(value)` | 非受控的 `Popover` 是否以打开状态开始 |
| `start(value)` | `SliderThumb` 是区间 slider 的哪一个滑块 |
| `href(url)` | `Link` 的绝对 HTTP(S) 目标 |

### 无障碍

| 方法 | 报读什么 |
| --- | --- |
| `accessibility_label(text)` | 屏幕阅读器读出的内容；纯图标控件没有它就什么都不会被读出 |
| `role(name)` | 这个元素把自己报读成什么——仅限朴素元素、`Button` 与 `Checkbox` |
| `aria_selected(value)` | 脚本自己搭的列表里某一项的选中状态 |
| `aria_active_descendant()` | 在祖先持有键盘时，把本元素报读为当前焦点项 |
| `set_position(position, size)` | 从 1 开始的位置与总数——“第 2 个 tab，共 5 个” |
| `row_count(count)` | `Table` 的总行数，包含未渲染的行 |
| `column_count(count)` | `Table` 的总列数 |
| `axis(value)` | `RadioGroup` 或 `ToggleGroup` 的方向；只有语义，不做任何布局 |
| `tooltip(text)` | 只对指针有效的悬停说明，不能替代 `accessibility_label` |

### 焦点与键盘

| 方法 | 作用 |
| --- | --- |
| `track_focus(handle)` | 让这个元素成为该 handle 所指的对象 |
| `content_focus_handle(handle)` | `Select` 或 `Combobox` 打开时把键盘移到哪里 |
| `tab_index(index)` | 这个元素在 Tab 顺序中的位置；同时也把它变成一个 tab stop |
| `tab_stop(value)` | Tab 能否落到这里，不改变它在顺序中的位置 |

### 滚动与面板

| 方法 | 作用 |
| --- | --- |
| `overflow_scroll()` | 接管双轴的滚轮与触控滚动 |
| `overflow_x_scroll()` / `overflow_y_scroll()` | 单轴上的同一件事 |
| `overflow_scrollbar()` | 双轴滚动并绘制基础层的滚动条 |
| `overflow_x_scrollbar()` / `overflow_y_scrollbar()` | 单轴上的同一件事 |
| `mode(value)` | `Scrollbar` 的显示策略；不写则跟随主题 |
| `scroll_size(width, height)` | `Scrollbar` 据以计算滑块的内容尺寸 |
| `viewport_from_layout()` | 让 `Scrollbar` 从自身的盒子取 viewport |
| `track_scroll(handle)` | 给虚拟列表一个脚本可以驱动的滚动位置 |
| `with_item_to_measure_index(index)` | 虚拟列表在它滚动的那个轴上测量哪一项 |
| `size_range(min, max?)` | `resizable_panel()` 可被拖拽的范围，单位为像素 |

### 锚定浮层

| 方法 | 设置什么 |
| --- | --- |
| `anchor(value)` | 哪个角固定在触发元素上；无论怎样都会被钳进窗口 |
| `mouse_button(value)` | 哪个指针按键打开 `Popover` |
| `open_delay(ms)` | 指针要在 `HoverCard` 触发器上停留多久；默认 600 |
| `close_delay(ms)` | `HoverCard` 关闭前等待多久；默认 300 |
| `overlay_closable(value)` | 在打开的 `Popover` 之外按下是否将其关闭 |

### 动效

| 方法 | 作用 |
| --- | --- |
| `transition(property, policy)` | 完全在原生 GPUI 代码里，对之后的目标变化做动画 |
| `spring(property, policy?)` | 改用弹簧 |

property 取 `"opacity"`、`"width"`、`"height"`、`"left"`、`"top"` 之一，每一帧都不会进入 JavaScript。

### 样式模板

每一个都接受一个函数，函数收到一个游离的元素用来收集样式；返回值会被忽略，所以写成一条链或写成块状函数体都可以。

| 方法 | 作用于什么 |
| --- | --- |
| `hover(declare)` | 指针悬停在元素上时 |
| `active(declare)` | 元素被按下时 |
| `focus(declare)` | 元素持有焦点时 |
| `range_style(declare)` | `SliderIndicator` 已填充的部分——只管它长什么样，从不管它在哪里 |
| `cell_style(declare)` | `OtpInput` 的每个格子；没有它屏幕上什么都没有 |
| `cell_active_style(declare)` | 叠在上面一层，用于下一个数字将落入的那个格子 |
| `caret_style(declare)` | 那个格子为空时，里面闪烁的光标 |

### 样式方法

元素上其余的一切都是样式。它们分成两族，而且从不重叠：

- **59 个带参数的方法**，手工绑定：size、padding、margin、position、flex、border、radius 与 paint 各族。每个方法接受哪种长度类型跟随它的 Rust 签名，所以 `.p("auto")` 是类型错误，理由与它在运行时抛异常完全相同。
- **3,143 个无参方法**，从 GPUI 的反射表生成，完全零维护：`flex_col`、`items_center`、`gap_2`、`rounded_md`、`text_sm`、`size_full`、`truncate` 以及这一族的其余成员。这个数字跟着 GPUI 变；`gpui-shell types` 会打印你这次构建的数字。

两者都记录在 [Styling](./styling.md) 里，还有长度与颜色的语法，以及调色板定义的 token。
