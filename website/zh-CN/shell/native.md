---
title: Native Modules
description: 宿主如何把自己的 Rust 借给脚本——注册方式、纯数据边界，以及一个 native 函数要遵守的规则。
order: 8
---

# Native Modules

[Capabilities](./capabilities.md) 管的是脚本**碰不到**什么，这一页管的是另一半：宿主主动递过去什么。

脚本无法加载原生扩展。`dlopen` 进来的 Rust 没有稳定 ABI，而且一旦进了这个进程，它就握有进程所有的权限——一个允许这种事的沙箱等于没有沙箱。所以方向是反过来的：**由宿主在编译期注册它愿意暴露的 Rust**，脚本能碰到的恰好就是这些，此外没有别的。

```rust
use gpui_shell::native::{NativeModules, NativeValue};

let mut modules = NativeModules::new();
modules.register("workspace", |module| {
    module.function("project_name", |_| Ok(NativeValue::from("gpui-component")));
});
gpui_shell::set_native_modules(modules);
```

```js
import { native } from "gpui";

const workspace = native("workspace");
workspace.project_name();      // "gpui-component"
```

机制就这些。这一页余下的部分讲它要付什么代价、以及它拒绝什么。

## 注册表本身就是授权

默认的注册表是**空的**，形状与 `Capabilities::default()` 一样。什么都没注册的宿主就是没有授予任何原生访问，脚本来要模块时会被直接告知：

```text
native module `market` is not available: this host registered none.
Native modules are granted by the embedding application, with
gpui_shell::set_native_modules(...).
```

注册了东西之后，这条信息会改成报出确实存在的那些：

```text
unknown native module `marker`; this host registered: market, theme
```

```text
native module `market` has no function `quote`; it provides: quotes, ticks, watch, watch_all
```

在这之上刻意没有再加一层"按模块授权"。名单是宿主自己定的，所以**名单就是授权**——要收回一个模块，就是注册另一份名单，下一次调用即生效，不必等重启。

## 边界上只有纯数据

一个 native 函数收到 `NativeArguments`，返回 `NativeValue`：null、布尔、数字、字符串、数组、对象。这六种是脚本引擎与 JSON 都能承载的交集，正因如此，同一份注册表可以服务[分界线](./engine.md)之下的任何引擎。

它从不接收脚本句柄，这不是图方便：句柄会让宿主把一个脚本值的引用留到产生它的那次调用之后，也留到那个让上下文有效的 CallScope 之后。

参数按位置取出，类型检查与错误信息一并包含在内：

| 调用 | 得到 |
| --- | --- |
| `arguments.string(0)` | `&str`，否则报错并说明实际来的是什么 |
| `arguments.number(0)` | `f64` |
| `arguments.integer(0)` | `i64`，小数会被拒绝 |
| `arguments.boolean(0)` | `bool` |
| `arguments.value(0)` | 原始的 `NativeValue`，供接受多种形态的函数使用 |
| `arguments.get(0)` | `Option<&NativeValue>`，供可选参数使用 |

返回一条记录用的是 builder 而不是 map，因为一个对象往往**就是**脚本要渲染的那一行，字段顺序应该由宿主决定：

```rust
use gpui_shell::native::NativeObject;

NativeObject::new()
    .field("symbol", "AAPL.US")
    .field("last", 224.22)
    .field("watched", true)
```

错误是一条消息而不是一个类型：`NativeError::new("no such symbol")` 到了脚本那边就是一个可以 catch 的 `Error`。

## native 函数要遵守的三条

**不能回调进脚本引擎。** 一次 native 调用发生在一次脚本调用**之内**，而后者又在一次宿主调用之内；从这里再进 VM，就是在引擎栈帧还在、渲染过程正在构建元素树的时候执行脚本代码。不持有脚本句柄让这件事很难被无意写出来，而派发器本身也会直接拒绝嵌套调用——这样即使宿主绕别的路（比如推动 GPUI 直到某个视图重渲染）也会拿到一条能诊断的错误，而不是未定义行为。

**读写宿主状态正是它存在的意义。** 函数通过 `gpui_shell::scope::with_current_app` 拿到当前的 `App`，在活跃调用之外它是 `None`：

```rust
fn with_app<R>(read: impl FnOnce(&mut App) -> R) -> Result<R, NativeError> {
    gpui_shell::scope::with_current_app(read)
        .ok_or_else(|| NativeError::new("only reachable while a script call is in progress"))
}
```

**在里面调 `cx.notify()`，通知会在这次调用退栈之后才送达。** 所以一个 native 函数可以改动实体、并要求关注它的视图重新渲染，而这次重渲染不会发生在调用它的那段脚本脚下。

## 一个真实的例子

gallery 的 Shell story 只注册了两个模块，它们就是它的脚本能碰到的全部扩展面。宿主这一侧：

```rust
fn install_native_modules(market: &Entity<Market>) {
    let mut modules = NativeModules::new();

    modules.register("market", |module| {
        let read = market.clone();
        module.function("quotes", move |_| with_app(|cx| read.read(cx).to_native()));

        let flip = market.clone();
        module.function("watch", move |arguments| {
            let symbol = arguments.string(0)?;
            with_app(|cx| {
                flip.update(cx, |market, cx| {
                    let watched = market.watch(&symbol)?;
                    // 在这次调用退栈之后才送达，所以不会重入引擎：
                    // story 与脚本视图会一起重新渲染。
                    cx.notify();
                    Ok(NativeValue::from(watched))
                })
            })?
        });
    });

    modules.register("theme", |module| {
        module.function("palette", |_| with_app(palette));
    });

    gpui_shell::set_native_modules(modules);
}
```

用它的脚本这一侧——读的是旁边那块 Rust 面板同一个 `Market` 实体：

```js
import { native } from "gpui";

const market = native("market");
const quotes = market.quotes();
const watched = quotes.filter((quote) => quote.watched).length;
```

用 `cargo run -- shell` 就能跑起来。两块面板经由两条路径读同一个实体，一旦对不上就会立刻看出来。

## 给它们加类型

`gpui-shell types` 无从知道宿主注册了什么，所以生成的 `gpui.d.ts` 留下一个空的 `NativeModules` 接口，交给应用自己补：

```ts
declare module "gpui" {
  interface NativeModules {
    market: {
      quotes(): Quote[];
      ticks(): number;
      watch(symbol: string): boolean;
      watch_all(on: boolean): number;
    };
  }
}
```

把这段写在脚本旁边的 `.d.ts` 里，`native("market")` 就变成了有类型检查的调用——模块名会被核对，它的函数也能补全。不写也没有代价：还有一个不带类型的重载兜底，从不写这份声明的应用照常工作。

## 还没有的东西

- **异步 native 函数。** 函数返回的是值而不是 promise；耗时的活会阻塞渲染所在的线程。
- **按模块授权。** 一个宿主一份名单，这是刻意的设计。要让两个插件各拿一份不同的名单，那是 `Policy` 的职责，而分发它的插件模型还没有文档。
- **流式返回，或回调进宿主。** 脚本不能把一个函数交给 native 模块，模块只能被调用。
