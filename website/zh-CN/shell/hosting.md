---
title: Hosting the Runtime
description: Rust 这一侧的全貌——运行时的生命周期、挂载脚本视图、从宿主状态刷新它、指标、退出请求与 hot-reload。
order: 10
---

# Hosting the Runtime

[Getting Started](./getting-started.md) 给的是把脚本视图放上屏幕的那四行。这一页是 Rust 接口的其余部分：该调什么、什么时候调，以及那两三处“看起来该调的那个其实是错的”。

## 运行时

一个 `ShellRuntime` 拥有一个 VM。它是一个带内部可变性的 `Rc`——既不是 `Send` 也不是 `Sync`——所以它待在拥有 `App` 的那个线程上。

```rust
gpui_shell::init(cx);                     // gpui-base、默认 token 调色板、样式表

let runtime = ShellRuntime::new()?;       // 一个 VM
runtime.set_global(cx);                   // 之后可用 ShellRuntime::global(cx) 取回
```

`set_global` 的作用，是让回调、native 模块或 hot-reload 之后还能找回这个运行时，而不必让宿主把句柄一路传下去。跑一个应用的宿主调用它一次；全局只有一个，所以想要两个互相隔离的运行时的宿主，得自己拿着第二个句柄。

## 加载与实例化

加载把源码变成一个**视图类型**——脚本 default 导出的那个类。实例化把这个类型变成一个**视图对象**，也就是一个活的实例：

```rust
let view_type = runtime.load_app(&root, "main.js")?;    // 一个目录
let view_type = runtime.load_source("inline", source)?; // 一个字符串，测试用

let object = runtime.instantiate(&view_type, window, cx)?;
```

`load_app` 会解析目录、读取入口文件、求值该模块。这里的每一种失败都是一个带着脚本自身调用栈的 `ShellError`——语法错误、解析到应用根目录之外的 import、缺失或形态不对的 default 导出。

实例化会执行脚本的 `init`，因此它需要一个活的 `Window`：`init` 里可能会创建 `InputState` 这类留存状态。

## 挂载

脚本视图和别的 GPUI 视图没有两样，它挂在**一个 `ShellRoot` 之下**：

```rust
cx.open_window(options, move |window, cx| {
    let object = runtime.instantiate(&view_type, window, cx).expect("view");
    let content = cx.new(|_| ScriptView::new(runtime.clone(), object));
    cx.new(|cx| ShellRoot::new(content.into(), window, cx))
})
```

`ShellRoot` 持有 dialog 栈、sheet、toast 栈、焦点恢复与 Tab 导航——正是 `Root` 对一个 `gpui-component` 窗口所起的作用。`window.open_dialog` 这一类调用要经由它找到根视图，所以挂在别的根视图之下的脚本会拿到一条讲清原因的拒绝，而不是悄无声息地没反应。

宿主也可以直接驱动同样这几个界面，插件面板与宿主自己的 UI 因此落在同一个栈里：

```rust
root.update(cx, |root, cx| {
    root.open_dialog(view.into(), window, cx);
    root.push_toast(ToastRequest::new("Saved").with_level(ToastLevel::Success), window, cx);
    root.close_all_dialogs(window, cx);
});
```

## 宿主状态变了，怎么刷新视图

这是最容易调错的一个，而且调错了不会报错。

```text
cx.notify()        ── 把这个视图再画一遍       （不跑脚本）
view.refresh(cx)   ── 而且它的描述已经过期了   （脚本会跑）
```

因为脚本的一次 `render` [不等于一帧渲染](./state.md#render-什么时候执行)，光调 `cx.notify()` 重绘的是已经存在的那份 snapshot。如果宿主改动的是脚本**会读到**的东西——某个 native 模块背后的实体、一项设置、一份文档——就必须告诉视图：描述本身已经过期了。

```rust
script_view.update(cx, |view, cx| view.refresh(cx));
```

`refresh` 等于 `invalidate` 加 `notify`。单独用 `invalidate` 只标记视图、不安排帧，适合多处改动一起落地、由一次重绘覆盖它们的场景。

反过来调错则立刻看得见——界面就是不更新——这与 GPUI 里忘了调 `cx.notify()` 是同一种失败方式。

## 脚本能碰到什么

三项授权，每一项都在调用时读取，所以改动在下一次调用生效，而不必等重启：

```rust
gpui_shell::set_capabilities(
    Capabilities::new()
        .read_roots([app_root.clone()])
        .write_roots([data_dir.clone()])
        .store(true),
);
gpui_shell::set_store_path(data_dir.join("store.json"));
gpui_shell::set_native_modules(modules);
```

三项的默认都是“什么都没有”：没有文件访问、没有存储位置、没有 native 模块。见 [Capabilities](./capabilities.md) 与 [Native Modules](./native.md)。

## 观察它花了多少

运行时把两件事分开计数，而这两个数之间的差就是重点：

```rust
let reading = runtime.read_metrics();
reading.script_renders();      // 跟着 cx.notify()、重载、主题变化走
reading.materializations();    // 跟着帧走
reading.script_render_time();  // 脚本 render 里的总耗时
reading.native_time();         // 其中花在 native 模块里的部分
reading.slowest_script_render();
```

`RuntimeMetrics::since(&earlier)` 给出两次读数之间的差值，每秒速率就是这么算的。这里没有重置：计数器属于运行时，把它们清零会把正在读它们的其他人一起挪动。要量某一段，就自己留一个基线再相减——Shell story 每次切换 feed 都会取一次基线，所以它的读数回答的是“这个 feed 要花多少”，而不是“这个窗口从打开到现在干了多少”。

回归测试可以直接对 `script_renders` 做断言；[基准测试里的第三个数](./engine.md#那次实测)靠的正是这一点。

## 退出请求

脚本里的 `process.exit(code)` 是**一个请求，绝不是 `exit(2)`**。一个插件不能把宿主进程带走，而宿主可能还有未保存的状态。运行时把这个请求交给宿主，由宿主决定怎么办：

```rust
gpui_shell::on_exit_request(|request, window, cx| {
    match request.view() {
        Some(view) => close_the_panel_showing(view, window, cx),
        None => cx.quit(),
    }
});
```

`request.code()` 是脚本要求的退出码，`request.view()` 在有的情况下会指出请求来自哪个视图——插件宿主关掉的应该是**那个**插件的面板，若换成关窗口，就等于让一个插件终结了别人的工作。

**授权了 exit 却没装处理器的宿主，会在调用现场被告知**，而不是永远不知道：`process.exit()` 会抛出异常，并点名 `on_exit_request`。一个没人回应的请求，是朝着讨好方向说的谎——脚本拿到了成功，而什么都没发生。

## Hot-reload

一个调用就能开起来，`--watch` 用的也是这一个：

```rust
gpui_shell::watch::reload_in_debug(
    &runtime, &view, app_root.clone(), "main.js", window, cx,
).forget();
```

这个签名有两点要说。它在 **release 构建下什么都不做**——返回一个空句柄，所以把这行留在代码里只值一个分支。另外返回的 `Watch` 本身就是这次监听：把它 drop 掉，循环就停，这正是宿主卸下一块面板时想要的；而 `.forget()` 让它跟着视图一直活下去。视图、运行时或窗口任意一个消失时，循环也会自己结束——因为它对这三者都只持弱引用；这里若持强引用，dock 已经移除的面板，其运行时会一直不被释放。

一次重载会重新读取**每一个**模块，入口也在内——一个悄悄用了旧 import 的 hot-reload 比没有更糟，因为它看起来是成功的。它会先把所有可能失败的活干完，再去碰活着的那个视图：新代码加载失败时，上一个视图继续运行，错误进 `tracing`，窗口里由一条固定 id 的 toast 报出来；下一次成功的重载会撤掉这条 toast。

视图本身能挺过重载。`ScriptView::replace_object` 只换掉脚本产出的那部分，实体保留下来，随之保留的还有窗口、焦点与元素身份。

## 脚本出错的时候

抛异常的脚本不会把界面一起带走。最后一份可用的 snapshot 仍然挂在那里，失败信息报在它上面，读者的滚动位置、焦点、正在读的内容都还在。在有什么让视图失效之前，运行时不会重跑那个失败的 `render`。

记得装一个 `tracing` subscriber。运行时通过 `tracing` 报告脚本错误、未处理的 promise rejection 与非法 phase 调用，target 是 `gpui_shell::script`；没有 subscriber 的话这些全部被丢弃，症状就是一个悄悄不再响应的视图。

## 还没有的东西

- **一个进程里两个运行时。** 全局句柄只有一个；第二个运行时得自己一路传下去。
- **给卡住的脚本做监管。** 解释器自己的中断会切断一次调用，但没有东西会去重启一个反复撞上中断的运行时。
- **插件模型。** `PluginManager` 和 `PluginManifest` 已经写好也有测试，但对外不可见——目前还没有任何东西真的去加载一个插件，把它们公开就等于对一套从未被调用过的 API 作出承诺。对一个只跑单个应用的宿主来说，上面那三项授权就是全部；要同时跑多个应用，公开的是 `Policy`。
