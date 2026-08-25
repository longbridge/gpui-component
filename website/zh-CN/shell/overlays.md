---
title: Overlays
description: cx 上的 dialog、sheet 与 toast，它们的层叠与关闭顺序，以及为什么只能从事件中打开。
order: 7
---

# Overlays

Dialog、sheet 与 toast 是**宿主**能力，通过 `cx` 访问。它们不是脚本画出来的东西。

Dialog 不是一个浮动的 `div`。它是窗口层叠顺序中的一个位置、一个焦点陷阱、一个 Escape 目标，以及一个关于“按下遮罩意味着什么”的承诺——而这些都必须由窗口的根视图决定，因为只有能同时看到所有浮层的东西才能给它们排序。脚本自己画的 dialog 一样都拥有不了；两个脚本各画一个 dialog，拥有得更少。

所以脚本说的是**放什么**到用户面前，根视图说的是它放在哪里、以及怎么离开。跨越这条边界的东西很少：一个待实例化的视图类、一个贴靠的边、一句要显示的话。

## 接口

```js
const depth = cx.open_dialog(ConfirmClear, {
  escape_dismissable: false,
  backdrop_dismissable: false,
  props: { count },
});
cx.close_dialog();        // -> 有没有关掉东西？
cx.close_all_dialogs();   // -> 关掉了几个

cx.open_sheet("right", FiltersPanel, { props: { filters } });
cx.close_sheet();         // -> 有没有关掉东西？

cx.toast({ title: "Saved", description: "3 files", level: "success",
           timeout: 4000, id: "save" });
cx.dismiss_toast("save");
cx.dismiss_all_toasts();
```

## Dialog

`cx.open_dialog(ViewClass, options?)` 接受的是**类本身**——不是实例，也不是元素：

```text
expected a view class; open_dialog and open_sheet take the class itself,
not an instance and not an element
```

运行时构造一个实例，把 `options.props` 传给它，并像挂载其他脚本视图一样挂载它。Dialog 的内容就是一个带自己 `init` 与 `render` 的普通视图：

```js
// confirm.js
import { View, v_flex, h_flex, text } from "gpui";

export default class ConfirmClear extends View {
  init(props) {
    this.count = props?.count ?? 0;
  }

  render() {
    return v_flex()
      .w(360)
      .bg("surface")
      .border(1)
      .border_color("border")
      .p(24)
      .gap(12)
      .child(text(`Delete ${this.count} completed items?`))
      .child(text("This cannot be undone."))
      .child(
        h_flex()
          .justify_end()
          .gap(8)
          .child(cancelButton((_event, cx) => cx.close_dialog()))
          .child(deleteButton((_event, cx) => cx.close_dialog())),
      );
  }
}
```

注意根视图提供了什么、又没有提供什么。它提供遮罩、位置、层叠、焦点陷阱，以及承载内容的表面；宽度、内边距、边框、文字与按钮和这个运行时里的其他一切一样，都是脚本的。

| 选项 | 默认 | 作用 |
| --- | --- | --- |
| `escape_dismissable` | `true` | Escape 是否关闭它 |
| `backdrop_dismissable` | `true` | 按下遮罩是否关闭它 |
| `props` | — | 传给类的构造函数，从而传给 `init` |

未知选项会被拒绝而不是被忽略，这正是重点：

```text
unknown option `escapeDismissable` for cx.open_dialog(view, options);
expected escape_dismissable, backdrop_dismissable or props
```

一个被悄悄忽略的 `escapeDismissable` 看起来像是生效了，而那个 dialog 照样可以被 Escape 关掉。

`open_dialog` 返回的是**栈的新深度**，不是句柄。根视图按位置而不是按身份寻址 dialog，所以句柄就必须承诺“关掉**这个** dialog”，而那不是一个存在的操作。深度才是脚本用得上的东西——用来断言确实打开了一个，或者退回到某个已知层级。`close_dialog` 返回它有没有找到可关的；`close_all_dialogs` 返回关掉了几个。

::: warning 不要把 `cx` 带进 dialog
打开 dialog 的那个回调里的 `cx` 属于那个回调。等到 dialog 自己的按钮被按下时，它已经过期，使用它会报出 stale context 错误。请通过 `props` 传**数据**，并从 dialog 自身回调的参数里取 `cx`。
:::

## Sheet

```js
cx.open_sheet("right", FiltersPanel, { props: { filters } });
```

同时最多打开一个 sheet，贴靠 `"left"`、`"right"`、`"top"` 或 `"bottom"`。它唯一的选项是 `props`；它没有关闭选项，因为总共只有一个，并且在没有 dialog 压在上面时由 Escape 或它的遮罩关闭。

```text
unknown sheet side `middle`; expected left, right, top or bottom
```

## Toast

Toast 是唯一**是数据而不是视图**的浮层——没有类、没有实例，也没有什么要脚本去渲染——所以它的全部内容以一个选项对象的形式跨越边界。

| 字段 | 默认 | 含义 |
| --- | --- | --- |
| `title` | 必填 | 用户读到的那句话 |
| `description` | — | 第二行 |
| `level` | `info` | `info`、`success`、`warning` 或 `error` |
| `timeout` | 5 秒 | 毫秒数，或 `null` 表示一直留到被关闭 |
| `id` | 自动生成 | 身份，用于替换与关闭 |

省略 `timeout` 使用默认值，显式写 `null` 让 toast 常驻，所以这两者不能合并成一个选项。

`id` 是把“反复失败”变成“一条长期存在的信息”而不是一堆通知的关键。`--watch` 的循环用的正是这个：一次失败的重载会用固定 id 发一条常驻的错误 toast，于是把一份坏文件存五次是替换而不是叠出五条；下一次成功重载再用 `dismiss_toast` 撤回它。

```text
unknown toast level `fatal`; expected info, success, warning or error
```

同时挂载三条 toast。更早的留在管理器里，随着较新的离开再出现，所以一次爆发是被节流而不是被丢弃。

## 层叠与关闭

从后往前绘制：

1. **内容**——脚本的根视图。
2. **Sheet**——最多一个，贴靠某条边。Sheet 是窗口里的一个*位置*，所以它位于 dialog 栈之下：从 sheet 里唤起的 dialog 必须可读，而在 dialog 之下唤起的 sheet 不能盖住它。
3. **Dialog 栈**——按打开顺序，最早的在最下面。
4. **Toast**——在所有东西之上。Toast 报告的是用户刚做的那个操作的结果，而“正开着一个 dialog”恰恰是这个结果最重要的时刻，所以它是唯一永不被遮挡的一层。

只有最上层的 dialog 画遮罩：三层 dialog 让窗口变暗一次而不是三次，而那唯一一层遮罩正是把活跃的 dialog 与它背后失效的那些区分开的东西。

关闭永远是**一层，绝不连锁**：

- **Escape** 只关闭最上层的 dialog。下层 dialog 渲染时禁用键盘处理，所以连按 Escape 会一层一层退栈，并且在还有 dialog 打开时永远不会波及 sheet。
- `escape_dismissable: false` 撤掉的是**按键绑定**，不是底层的取消动作。脚本放在 dialog 里的关闭控件照样有效——这正是“不可关闭的 dialog”意味着用户必须回答它，而不是被困在里面。
- **按下遮罩**关闭最上层的 dialog，且仅当它是以 `backdrop_dismissable` 打开的。
- **回车在这一层什么都不做。** Base 的 dialog host 把回车视为“确认并关闭”；那属于 dialog 自己的主按钮，而主按钮归脚本所有，所以根视图否决了内建的确认行为，而不是去猜哪份内容需要它。
- **Sheet** 只在没有 dialog 打开时由 Escape 或它的遮罩关闭，因为压在上面的 dialog 持有焦点。
- `close_all_dialogs` 是唯一会整体退栈的操作，而且它不动 sheet。

**焦点**沿着栈自身的历史恢复。打开浮层时记录当前焦点并把焦点交给浮层，关闭时恢复。关掉第二个 dialog 会把焦点还给第一个，关掉第一个则还给二者打开之前窗口所在的位置。Tab 与 Shift-Tab 遵守焦点陷阱，所以在浮层里按 Tab 是在浮层内循环，而不是走进它背后的内容。

## Phase 规则

**浮层只能从事件回调或任务中打开与关闭。**

```text
cx.open_dialog(view, options) is not allowed during the `render` phase;
overlays may only be opened or closed while handling an event or a task
```

打开或关闭浮层会修改窗口，而 `render` phase 正在读它。GPUI 的借用模型无法表达“脚本在这里可以 notify、在那里不行”，所以运行时显式携带 [phase](./state.md#phase)，每一个浮层入口都拒绝 `render`、`layout`，以及根本不在任何宿主调用中的情形——最后这种情况下也没有窗口可以触达。

拒绝信息会写明它是从哪个 phase 发出的，因为那是作者唯一的线索。

## 浮层需要 `ShellRoot`

上述每一个调用最终都会到达窗口的根视图。第一层视图不是 `ShellRoot` 的窗口会拒绝它们，并指明这是哪一类错误——宿主接线问题，不是脚本问题：

```text
cx.open_dialog(view, options) needs a ShellRoot as the window's first view;
this window was opened with another view
```

见 [Getting Started](./getting-started.md#把运行时接进-rust-应用)。

## 还没有的东西

- **Dialog 的返回值。** `open_dialog` 返回的是深度，不是一个在 dialog 关闭时 settle 的 promise。请通过 `props` 传一个回调，或者让 dialog 写回打开方会读取的状态。
- **Popover、tooltip 与右键菜单。** Base 有这些构件，脚本接口上一个都没有。
- **定位选项。** Dialog 居中，sheet 贴边，两者都不能指定位置。
