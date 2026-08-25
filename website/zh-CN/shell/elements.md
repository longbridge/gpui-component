---
title: Elements
description: 构造器、用 child / children / when 组合，以及元素描述为什么只能使用一次。
order: 4
---

# Elements

`gpui-shell` 里的元素是一段**描述**，不是一个对象。它只在一次渲染中存在，被使用时即被消费。本页讲能构建什么、怎么组合，以及一段描述被用了两次时运行时会做什么。

## 构造器

一次 import 就是整个命名空间：

```js
import { div, h_flex, v_flex, text, svg, Button, Checkbox, Switch, Input, InputState } from "gpui";
```

函数是小写的，组件类型首字母大写并通过 `.new` 构造。这与 Rust 侧一一对应：那边 `div()` 同样是自由函数，`Button::new(id)` 同样是类型上的关联函数。

| 构造器 | 产出 |
| --- | --- |
| `div()` | 自身不带布局的元素 |
| `h_flex()` | 一行 |
| `v_flex()` | 一列 |
| `text(value)` | 文本元素，参数会被转成字符串 |
| `svg(path)` | 来自应用自身目录的图片 |
| `Button.new(id)` | base 的 `Button`：激活、焦点、disabled 与 selected 状态，无样式 |
| `Checkbox.new(id)` | base 的受控 checkbox，无样式也无勾选标记 |
| `Switch.new(id)` | base 的受控 switch，无样式 |
| `Input.new(state)` | 由 [`InputState`](./state.md#留存状态) 支撑的文本框 |

### 为什么是 `.new(id)` 而不是 `new Button(id)`

JavaScript 的习惯写法是 `new Button(id)`。运行时不提供它，理由正是本页的主题：`new` 承诺的是一个有身份的对象——可以保存、可以挂在实例上、可以再次使用。而描述恰恰不是这种东西。`Button.new(id)` 读起来是“构造一段描述”，它做的也正是这件事，并且与 Rust 侧一字不差。

视图是相反的情形，用的就是标准写法：`class Counter extends View`。视图确实有身份、有跨帧状态，并且由 GPUI 拥有。同一份文件里出现两种构造形态，是因为这两类东西的生命周期本来就不同。

### id

`Button`、`Checkbox` 与 `Switch` 的 `id` 用于跨渲染标识元素，GPUI 据此保留焦点与元素状态。请保持它稳定，并在兄弟节点之间唯一——用 `` `item-${item.id}` ``，而不是一个会在列表被筛选时移位的数组下标。

其余元素——`div`、`h_flex`——的身份是**它在这次渲染所构建的树里所处的位置**。只要树的形状不变，这就够用；而一旦上方多出一个条件子节点，它下面的每个元素都会移位，按下状态、焦点以及其他按身份记录的东西都跟着移位。

`.id(name)` 用来说明“这是哪个元素”，而不是“它落在了哪里”：

```js
div()
  .id("toolbar")
  .active((el) => el.opacity(0.7))
```

凡是身份必须扛得住邻居变化的元素，都给它取个名字。`Button`、`Checkbox` 与 `Switch` 已经从 `new(id)` 拿到了身份，会忽略这里的名字——并且是给出警告，而不是默不作声。

### 文本

`text(value)` 会把参数转成字符串，所以模板字符串和数字可以直接用：

```js
text(`${this.remaining} of ${this.items.length} remaining`);
text(42);
```

文本元素最终变成一个包含该字符串的 `div`，所以它和其他元素一样接受样式，也可以再挂子元素。

### 图片

```js
svg("icons/check.svg").w(14).h(14).flex_none();
```

`svg` 的路径相对于**应用根目录**——也就是交给 `gpui-shell` 的那个目录——而不是相对于调用它的文件。这个不对称常常让人意外，所以值得直说：`import "./ui.js"` 相对于发起 import 的文件解析，和所有 JavaScript 模块系统一样；而 `svg("icons/check.svg")` 相对于应用根目录解析，和 Web 应用的 public 目录一样。运行时无法知道是哪个模块调用了 `svg`，因此按文件解析的资源路径对它并不可得。

越出应用目录的路径会被拒绝。缺失的文件会按路径去重报告一次，并附上查找位置，而不是安静地什么都不画。

图标会继承周围的文字颜色，所以深色按钮里的图标不用脚本说第二遍就是浅色的：

```js
div()
  .bg("foreground")
  .text_color("surface")
  .child(svg("icons/check.svg").w(11).h(11));  // 以 surface 绘制
```

## 组合

| 方法 | 作用 |
| --- | --- |
| `.child(element)` | 添加一个子元素，该子元素随即被消费 |
| `.children(iterable)` | 按顺序添加多个 |
| `.when(condition, branch)` | 仅当 `condition` 为真时应用 `branch` |

```js
v_flex()
  .gap(8)
  .child(this.header())
  .children(this.visible().map((item) => this.row(item)))
  .when(this.items.length === 0, (el) => el.child(text("Nothing yet")));
```

`.when` 的存在是为了不让一个条件把链断成两截。`branch` **必须返回该元素**——不返回的分支会立刻抛异常，而不是悄悄丢掉它构建的一切：

```text
when(...) must return the element
```

这与 GPUI 自己的 `FluentBuilder`，以及本仓库 Rust 侧“元素构建保持一条流式链”的风格规则同源。

如果条件是在两个元素之间二选一，普通三元表达式比 `when` 更清楚：

```js
.child(
  visible.length === 0
    ? emptyState("No items yet", "Type above and press Add.")
    : v_flex().children(visible.map((item) => this.row(item))),
)
```

## 行为方法

这些不是样式。它们把状态报告给基础层，由基础层处理交互，外观仍然交给你。

| 方法 | 用于 | 作用 |
| --- | --- | --- |
| `.on_click(handler)` | `Button` | `handler(event, cx)`，点击**以及**键盘激活都会触发 |
| `.on_change(handler)` | `Checkbox`、`Switch` | `handler(checked, cx)`，由脚本保存新值 |
| `.disabled(value)` | `Button`、`Checkbox`、`Switch` | 阻止激活并报告该状态 |
| `.selected(value)` | `Button` | 报告 selected 状态 |
| `.checked(value)` | `Checkbox`、`Switch` | 受控值 |
| `.accessibility_label(text)` | `Button`、`Checkbox` | 屏幕阅读器读出的内容 |
| `.id(name)` | `div`、`h_flex`、`v_flex` | 一个稳定的身份，取代“在树中的位置” |

disabled、selected 与 checked 的**外观**由你来画。基础层只报告状态，脚本不说就什么都不会变：

```js
Button.new("clear")
  .disabled(this.completed === 0)
  .when(this.completed === 0, (el) => el.opacity(0.4))
  .child(text("Clear completed"));
```

`.accessibility_label` 对纯图标控件最重要——没有它，这类控件什么都不会被读出来：

```js
Button.new(`remove-${item.id}`)
  .accessibility_label(`Remove “${item.caption}”`)
  .child(svg("icons/trash.svg").w(14).h(14));
```

### 受控值只报告意图

base 的 checkbox 不会自己改状态。它只报告用户的请求，由脚本决定：

```js
Checkbox.new(`item-${item.id}`)
  .checked(item.done)                       // 值来自脚本状态
  .on_change((done, cx) => {                // 回调只是一个请求
    this.toggle(item.id, done, cx);
  })
  .child(indicator(item.done))
  .child(label(item.caption));
```

运行时绝不会替脚本悄悄维护一个 checked 标志。如果它这么做，脚本作者与 Rust 作者会对同一个控件持有不同的心智模型，而这两类作者共存于同一个应用里。

### 事件对象

`on_click` 的处理函数收到的是一个普通对象，字段名与 Rust 结构一致：

```js
.on_click((event, cx) => {
  // event.click_count === 1
  // event.modifiers === { shift, control, alt, platform }
});
```

`platform` 在 macOS 上是 Command，其他平台是 Windows 键。这里只暴露基础层已经归一化过的语义——Base 把“回车激活按钮”与“点击按钮”归为同一个回调，脚本看不到这个差别。

::: tip 事件处理器请用箭头函数
箭头函数不绑定自己的 `this`，所以处理函数里的 `this` 仍然是视图实例。用 `function () {}` 写会拿到错误的 `this`。这是为本运行时写脚本时最常见的一处错误，人和模型都一样。
:::

## 元素是一次性的

这条规则最容易让新读者意外，所以下面写清它长什么样、以及为什么成立。

```js
const row = h_flex().child(text("hello"));

v_flex()
  .child(row)
  .child(row);   // 抛异常
```

```text
element `h_flex` was already added to a parent; elements are single-use values
```

跨帧保存也是同样的失败：

```js
init() {
  this.header = h_flex().child(text("Todo"));   // 错误
}

render() {
  return v_flex().child(text("Todo list")).child(this.header);
}
```

```text
this element belongs to a previous render pass; elements are single-use values
and must be rebuilt each time render runs
```

有一处毛刺值得知道：arena 每一趟都会清空并复用下标，所以一个过期元素偶尔会正好持有运行时刚分配给“它要挂上去的那个节点”的下标。误用仍然会被抓到，但信息变成 `an element cannot be added to itself`。两者含义相同——这个元素属于一趟已经结束的渲染。

### 为什么

这条限制来自 GPUI 本身：`RenderOnce::render` **按值**取走 `self`，`.child()` 也按值取走子元素。Rust 里编译器用移动语义强制这一点：使用已移动的值是编译错误。JavaScript 既没有移动语义也没有编译器，于是运行时在运行期强制同一条规则——而描述 arena 本来就有做这件事所需的记录，因为节点被挂载的那一刻就会被标记为已有父节点。

另一种做法是在重复使用时复制描述。这一条被否决了：它会让同一段脚本在 Rust 与 JavaScript 里含义不同，而重复使用几乎总是错误而非本意。

### 可行的写法

在 `render` 里构建，把重复部分抽成**每次返回新元素的函数**：

```js
const label = (value) => text(value).text_size(12).text_color("foreground");

render() {
  return v_flex()
    .child(label("first"))
    .child(label("second"));
}
```

[示例应用](https://github.com/longbridge/gpui-component/tree/main/examples/js_todolist)就是这样写的：`ui.js` 把 `button`、`label`、`icon`、`checkbox` 等导出为函数，`main.js` 调用它们。读起来像一个组件库，而且不花什么代价——一次函数调用就是一段新描述的来源。

## 回调属于它所在的那次渲染

传给 `.on_click` 的处理函数属于那次渲染产出的那份描述——而不是属于某一帧。那份描述会[被之后的每一帧复用，直到有东西让它失效](./state.md#render-什么时候执行)，处理函数在这期间一直可调用。描述里只记录一个 id；Rust 装配的闭包持有对运行时的弱引用加上这个 id。

被替换掉的那份描述会多保留一代，因为事件可能针对一个已经被取代的帧派发。再晚到达的事件会被丢弃并记一条 `debug` 日志，而不是报错——作者没有做错什么，也没有什么可修。

实际后果是：渲染期注册的回调不是订阅。需要活得比本次渲染更久的东西——比如响应输入框的 `change` 事件——见 [State and Views](./state.md#输入事件)。

## 未知方法是错误

既不是样式也不属于上面那批行为方法的调用，会在调用点失败；如果有相近的名字，会给出建议：

```text
unknown element method `items_centre` (did you mean `items_center`?)
```

```text
unknown element method `on_clicked`; it is neither a style method nor one of
child, children, when, on_click, on_change, disabled, selected, checked, id
```

这件事比看上去重要。拼错的样式名不会改变画面——它只是没起作用——没有诊断的话完全不可见。运行时如何在不给每次渲染加负担的前提下产生这条信息，见 [Styling](./styling.md#未知方法)。

## 还没有的东西

元素接口是 M0 的集合。以下是刻意缺失的部分，各属于后续里程碑：

- Select、tabs、list、table、tree 以及 `gpui-base` 的其他组件；
- `gpui.memo`——它能让未变化的子树跳过重建描述的那部分脚本工作；
- dock 面板，以及让脚本绘制 dock 自身 chrome 的 renderer trait；
- `img()` 构造器——今天唯一的图片元素是 `svg()`。
