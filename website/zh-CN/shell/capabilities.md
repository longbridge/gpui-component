---
title: 能力
description: 默认全部拒绝的模型，fs / store / clipboard / log / process 接口，存储位置，以及沙箱裁掉了什么。
order: 7
---

# 能力

脚本默认**什么都拿不到**。没有文件访问、没有存储、没有剪贴板、没有进程执行、没有网络。`Capabilities::default()` 就是空集，这一条写在代码里可断言，而不是写在注释里。

授权由宿主决定，因为只有宿主知道它对即将运行的这段代码信任到什么程度。每个入口都在调用时重新读取授权，所以撤销一项能力在下一次调用就生效，而不是等到下次重启。

```rust
gpui_shell::set_capabilities(
    Capabilities::new()
        .with_read_roots([application_root.clone()])
        .with_write_roots([data_directory.clone()])
        .store(true),
);
```

## 本地运行的应用被授予什么

从命令行运行一个目录，是一次明确的信任行为——与 `node app.js` 一样——所以 `gpui-shell <directory>` 授予的是一组具体且很窄的能力：

| | |
| --- | --- |
| 读 | 应用目录，以及它自己的存储目录 |
| 写 | 它自己的存储目录 |
| 存储 | 授予 |
| 剪贴板 | **不**授予 |
| 进程执行 | **不**授予 |
| 网络 | **不**授予 |

因此应用可以读自己的源码与资源、使用自己的存储，除此之外没有别的。它刻意比"全部放开"要窄，因为将来安装的插件会走同一条代码路径、由 manifest 来决定授权——而一个对本地运行足够宽松的默认，继承过去就是错的默认。

## 拒绝信息会写明怎么修

每一条拒绝都以"要声明什么"结尾，而不只是说了句拒绝：

```text
filesystem read is not granted; declare capabilities.fs.read in the manifest
```

```text
`/etc/passwd` is outside every granted read root;
add its directory to capabilities.fs.read in the manifest
```

```text
storage is not granted; set capabilities.store to true
```

```text
running `git` is not granted; add it to capabilities.fs.execute in the manifest
```

::: warning 目前还没有 manifest
这些信息提到 manifest 的 key，是因为插件模型落地后授权将来自那里。今天由宿主直接调用 `gpui_shell::set_capabilities`，而这些 key 名就是那个 API 将来会沿用的词汇。
:::

## `fs`

```js
import { fs } from "gpui";
```

| 调用 | 返回 |
| --- | --- |
| `fs.read_text(path)` | 文件内容 |
| `fs.write_text(path, contents)` | — |
| `fs.read_dir(path)` | `[{ name, is_dir }]`，按名字排序 |
| `fs.exists(path)` | `true` / `false` |
| `fs.remove(path)` | — |
| `fs.create_dir_all(path)` | — |

相对路径相对某个已授权的根解析；绝对路径必须本来就在某个根之内。这套接口里的每一条路径都经过**同一个解析器**：先做归一化，再要求结果仍位于某个根之下——所以 `../../etc/passwd` 在到达文件系统之前就被拒绝，也不存在第二处让穿越漏洞藏身的地方。

其中三项的行为值得说明，理由都是同一个：

**被拒绝的路径抛异常，而不是返回 `false`。** "你不能看"和"它不存在"是两个不同的事实，把它们合并会让脚本能一次一个布尔值地探测自己根目录之外的文件系统。

**`remove` 不递归。** 写权限是按根授予的，递归删除会把一次路径笔误变成整个应用数据目录的丢失。真要这么做的脚本可以自己遍历。

**`read_dir` 已排序。** 渲染列表的脚本不该自己再排一遍，也不该继承文件系统的任意顺序。

::: warning 这些调用是阻塞的
文件系统接口今天是**同步**的：它返回值而不是 promise，并且会阻塞渲染所在的线程。改成异步在计划之内，届时这些签名会变。请把工作量控制得小一些，不要在 `render` 里读文件。
:::

## `store`

跨重启存活的键值存储。

```js
import { store } from "gpui";

store.set("todolist.items", items);
const saved = store.get("todolist.items");   // 键不存在时为 null
store.remove("todolist.items");
store.keys();
store.flush();
```

值是 JSON：`null`、布尔、数字、字符串、数组与普通对象。函数与值为 `undefined` 的属性会像 `JSON.stringify` 一样被丢弃，所以心智模型可以直接迁移过来。`NaN` 与 `Infinity` 没有 JSON 形式，会被拒绝而不是悄悄变成 `null`。嵌套深度上限 64 层——真实配置远达不到，而引用环立刻就会超过。

值缓存在内存里，因为 `get` 在 `render` 里也可达，每次渲染读一次文件是荒唐的。**每次写入立即持久化**，先写临时文件再改名覆盖目标——所以写到一半崩溃留下的是之前完整的配置，而不是一个被截断的文件。因此 `flush` 不必调用；它留在 API 里，是为了将来写入变成可 await 的 promise 时充当持久化屏障。

### 存储在哪里

存储按应用划分，位置由宿主选择——应用不能指定自己的存储位置，否则两个应用可以故意撞在一起。

本地运行时，身份是**应用目录的规范化路径**，所以同一个目录总是访问到同一份数据，两个目录也永远不会冲突——包括同一个应用的两个 checkout，它们确实是不同的安装。路径是：

| 平台 | 位置 |
| --- | --- |
| Linux 与其他 Unix | `$XDG_DATA_HOME/gpui-shell/apps/<name>-<digest>/store.json`，默认 `~/.local/share` |
| macOS | `~/Library/Application Support/gpui-shell/apps/<name>-<digest>/store.json` |
| Windows | `%APPDATA%\gpui-shell\apps\<name>-<digest>\store.json` |

`<name>` 是应用目录名，保留下来是为了这个文件夹认得出来；`<digest>` 是完整路径的一个短哈希，只用来消歧。它放在用户数据目录而不是应用内部，因为应用目录可能只读、往往是一个 git checkout，也不是用户预期自己数据所在的地方。

### 未被授权时的退化

未被授权的存储会抛异常，而写得好的应用会把它当作关于宿主的一个事实，而不是一个错误：

```js
// storage.js —— 取自示例应用
import { store, log } from "gpui";

export function load() {
  try {
    const saved = store.get(KEY);
    return Array.isArray(saved) ? saved : [];
  } catch (error) {
    log.warn(`todolist: storage unavailable, starting empty (${error.message})`);
    return [];
  }
}
```

示例的页脚随后会在界面上说明这一点——"Not saved — this host did not grant storage, so the list lasts for this run only"——这才是对的形态：在边界处吸收拒绝，并对用户说实话。

## `clipboard`

```js
import { clipboard } from "gpui";

clipboard.write_text("copied");
const text = clipboard.read_text();   // 剪贴板中没有文本时为 undefined
```

读与写是**两项独立授权**，拒绝信息会指出缺的是哪一半：

```text
writing the clipboard is not granted; declare capabilities.clipboard.write in the manifest
```

剪贴板需要一次实时的宿主调用——GPUI 的 `App` 只在一次调用期间存在——所以在模块顶层调用它会直说，而不是 panic：

```text
clipboard.read_text() needs a live host call; call it from render, an event handler or a task
```

## `log`

```js
import { log } from "gpui";

log.info("loaded", count, { source: "disk" });
log.warn("could not save");
```

`debug`、`info`、`warn` 与 `error`。**不需要任何能力**：能跑起来的脚本本来就能说话，禁掉它只会让作者失去自己的诊断信息，别的什么都拦不住。

多余的参数会以空格分隔追加在后面，与 `console.log` 的行为一致。结构化的值以 JSON 打印，因为那是读日志的人想看到的形式。

输出通过 `tracing` 走，target 是 `gpui_shell::script`，所以在日志过滤里脚本输出与宿主输出是可分开的。**没有安装 `tracing` subscriber 的宿主会把这些全部丢弃**——连同运行时自己报告的抛异常的处理函数、未处理的 rejection 与 phase 非法的调用。`gpui-shell` 二进制安装的是一个 `INFO` 级别的 stderr sink，`--dev` 下是 `DEBUG`。

## `process`

```js
import { process } from "gpui";   // 同时也是一个全局

const code = process.run("git", ["status"]);
process.exit(0);
```

`process.run` 受执行授权约束，授权有三种形态：拒绝（默认）、命令名白名单，或不受限。

`process.exit` 是**一个请求，绝不是 `exit(2)`**。它记录退出码；宿主在脚本调用返回后取走它并决定怎么做——关闭插件的面板、关闭窗口，或者忽略。一个插件不能把宿主进程带走，而宿主可能还有未保存的状态。

这个名字上的撞车是刻意的。`process` 正是 JavaScript 作者——或者生成 JavaScript 的模型——会去伸手拿的名字，所以运行时把自己受能力约束的接口放在那里，而不是把这个名字空着、任其看起来像 Node 的却行为不同。

::: warning `process.exit` 挂在了错误的 key 上
`process.exit` 目前要求的是文件系统授权（`capabilities.fs`）而不是它自己的一项授权，拒绝信息也是这么说的。这是能力集合里还没有为它设条目留下的痕迹。
:::

## 沙箱

除了能力授权之外，运行时还会裁剪语言本身。以下全部在**未开启开发模式**时生效。

**没有动态代码。** `globalThis.eval` 被直接删除——`ReferenceError` 不会被特性探测误认为是一个可用的 `eval`，而一个抛异常的桩会。四个函数编译器全部被替换：`Function`，以及通过 `(async function(){}).constructor`、`(function*(){}).constructor` 和异步生成器等价物可达的那三个。`Function` 是被*替换*而不是删除，并保留了真正的 `Function.prototype`，所以 `x instanceof Function` 与 `.call` / `.apply` / `.bind` 继续可用，只有构造会抛异常。

**冻结内建原型。** `Object`、`Array`、`Function`、`String` 与 `Number` 的原型被冻结。一个 VM 将来会承载多个插件，这使得这些原型成为共享可变状态：一个插件给 `Object.prototype` 加一个可枚举属性，就改变了其他所有插件以及运行时自身 prelude 的 `for...in`。代价是真实的——一个给 `Array.prototype` 打补丁的库会在 import 时就停止工作——所以明知要运行这类库的宿主可以关掉冻结，并保留沙箱的其余部分。

**模块解析被限制在应用根目录内。** `import "./ui.js"` 相对发起 import 的文件解析；任何解析到应用目录之外的结果都会被拒绝。动态 `import()` 刻意保持可用——延迟加载将来靠它——并且由同一个解析器约束。

**资源上限**，让失控的脚本报错而不是把窗口一起带走：

| 上限 | 值 |
| --- | --- |
| 堆 | 256 MiB——泄漏表现为一个可捕获的 JavaScript 异常，而不是整个宿主被 OOM kill |
| 解释器栈 | 1 MiB——深递归表现为 `RangeError`，而不是原生栈溢出 |
| 单次调用耗时：render 与 layout | 50 ms |
| 单次调用耗时：event 与 task | 500 ms |
| 单次调用耗时：不在任何调用中，例如模块求值 | 5 秒 |

时钟在每一次宿主调用时重置，这正是渲染路径能比事件回调有更紧预算的原因。**中断无法被 `catch` 吞掉**——这一点有测试来度量，因为如果能被吞掉，中断就根本不是一道防线。

这里没有 `std` 也没有 `os`：quickjs-libc 从一开始就没有被编进这个构建。

::: warning 开发模式还没有接完
`--dev` 目前只启用源码监听。它本该打开的放宽项——恢复 `eval`、让内建原型保持可写，这是 REPL 需要的——还无法从二进制里触达，它会打印一条警告说明。库函数是存在的（`gpui_shell::set_development_mode`），并且必须在运行时构造之前调用，因为策略是在创建上下文时读取的。

开发模式从不放宽能力约束。它让语言更好摆弄，但不会发出任何人没有声明过的访问权限——因为一项作者从没写下来的授权，就是一项在生产环境里会缺失的授权。
:::

## 还没有的东西

- **`gpui.http`。** 能力模型里有 `capabilities.network.hosts`，`fetch` 的拒绝信息也提到了它，但没有 HTTP 接口。
- **Manifest，以及它所属的插件模型。** 今天授权来自宿主。
- **异步的 `fs` 与 `store.flush`。** 两者都阻塞。
- **`process.exit` 自己的那项能力。**
- **向用户询问授权。** 授权在应用加载之前就已决定，不会在使用的那一刻弹出询问。
