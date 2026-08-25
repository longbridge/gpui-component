---
title: Capabilities
description: 默认全部拒绝的模型，fs / store / clipboard / log / process 接口，存储位置，以及沙箱裁掉了什么。
order: 8
---

# Capabilities

脚本默认**什么都拿不到**。没有文件访问、没有存储、没有剪贴板、没有进程执行、没有网络。`Capabilities::default()` 就是空集，并有一条断言把它钉在那里。

授权由宿主决定，因为只有宿主知道它对即将运行的这段代码信任到什么程度。至于它主动**递出去**的东西——它自己的、有意暴露的那部分 Rust——见 [Native Modules](./native.md)。每个入口都在调用时重新读取授权，所以撤销一项能力在下一次调用就生效，而不是等到下次重启。

```rust
gpui_shell::set_capabilities(
    Capabilities::new()
        .read_roots([application_root.clone()])
        .write_roots([data_directory.clone()])
        .store(true),
);
```

## 本地运行的应用被授予什么

从命令行运行一个目录，是一次明确的信任行为——与 `node app.js` 一样——所以 `gpui-shell <directory>` 授予的是一组具体且很窄的能力：

|          |                                |
| -------- | ------------------------------ |
| 读       | 应用目录，以及它自己的存储目录 |
| 写       | 它自己的存储目录               |
| 存储     | 授予                           |
| 剪贴板   | **不**授予                     |
| 进程执行 | **不**授予                     |
| 网络     | **不**授予                     |

因此应用可以读自己的源码与资源、使用自己的存储，除此之外没有别的。它刻意比“全部放开”要窄，因为将来安装的插件会走同一条代码路径、由 manifest 来决定授权——而一个对本地运行足够宽松的默认，继承过去就是错的默认。

## 拒绝信息会写明怎么修

每一条拒绝都以“要声明什么”结尾，而不只是说了句拒绝：

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

每个调用都返回 promise。`await` 它们，或者接 `.then`——另见下面关于 `render` 的提示。

| 调用                            | resolve 结果                     |
| ------------------------------- | -------------------------------- |
| `fs.read_text(path)`            | 文件内容                         |
| `fs.write_text(path, contents)` | —                                |
| `fs.read_dir(path)`             | `[{ name, is_dir }]`，按名字排序 |
| `fs.exists(path)`               | `true` / `false`                 |
| `fs.remove_file(path)`          | —                                |
| `fs.remove_dir(path)`           | —                                |
| `fs.mkdir(path, options?)`      | —                                |

```js
const source = await fs.read_text("notes.md");
await fs.write_text("notes.md", source + "\n");
```

相对路径相对某个已授权的根解析；绝对路径必须本来就在某个根之内。这套接口里的每一条路径都经过**同一个解析器**，所以不存在第二处让穿越漏洞藏身的地方。它先做归一化——`../../etc/passwd` 在到达文件系统之前就被拒绝——然后把「是否在根之内」这件事交给文件系统判定，而不是判定字符串：授权承诺的是一个**目录**，而 `data/escape/passwd` 在字面上位于根之内，一旦 `escape` 是符号链接就读到了 `/etc/passwd`。路径中已经存在的最深一段会被连同链接一起解析，其结果必须仍在根之下；解析不到任何目标的符号链接会被直接拒绝，而不是猜它指向哪里。

**授权是一个句柄，不是一个字符串。** 解析器交回一个打开的目录，它无法被诱导指向自身之外的任何东西；读、写、列目录、删除、建目录全部对着**它**执行——于是一条路径永远不会被解析两次，「判定允许」与「实际使用」之间也就没有窗口。

这一点要紧，是因为显而易见的写法行不通。先检查路径再调 `std::fs`，路径被解析了两次：检查时就在的链接会被抓住，而在两次解析**之间**替换掉某个目录组件的，会被第二次解析跟出根目录。这里用的是 [`cap-std`](https://docs.rs/cap-std)——在 Linux 上是 `openat2(RESOLVE_BENEATH)`，其他平台是逐级 `openat` 遍历。

其中三项的行为值得说明，理由都是同一个：

**被拒绝的路径抛异常，而不是返回 `false`。** “你不能看”和“它不存在”是两个不同的事实，把它们合并会让脚本能一次一个布尔值地探测自己根目录之外的文件系统。

**删文件和删目录是两个调用**，和 Rust 一样——单独一个 "remove" 说不清目录算不算在内。`remove_dir` 只收空目录：写权限是按根授予的，递归删除会把一次路径笔误变成整个应用数据目录的丢失。真要这么做的脚本可以自己遍历。

**`mkdir` 就是别处那个 `mkdir`。** 不带参数时只建一层，父目录不存在就报错；`{ recursive: true }` 才把父目录一起建出来。它原来叫 `create_dir_all`——那个名字确实说清了它做什么，代价是它不是每个脚本作者已经认识的那个名字。

**`read_dir` 已排序。** 渲染列表的脚本不该自己再排一遍，也不该继承文件系统的任意顺序。

**每个调用都返回 promise。** 系统调用在主线程之外执行——磁盘要花多久没有上界，而在这里阻塞会同时卡住帧和 VM，而且卡在中断预算看不见的地方，因为那段时间花在内核里。

**拒绝仍然在调用点抛出**，而不是变成 rejected promise。能力检查几乎不花时间，留在调用线程上；而没人 await 的 rejected promise，等于没人看得见的拒绝。

`read_text` 会拒绝超过 64 MiB 的文件，并指出文件名和上限。没有这个上限的话，替代方案是一个必须塞进 JavaScript 堆的字符串——而那个堆本身也有上限——于是失败会表现为 VM 内部的内存耗尽，而不是一句你能据以行动的话。

::: tip 仍然不要在 `render` 里读文件
`render` 描述界面，它没法 await。在 `init` 或事件回调里读，把结果留在视图上，拿到后 `cx.notify()`。
:::

## `store`

跨重启存活的键值存储。

```js
import { store } from "gpui";

store.set("todolist.items", items);
const saved = store.get("todolist.items"); // 键不存在时为 null
store.remove("todolist.items");
store.keys();
await store.flush();
```

值是 JSON：`null`、布尔、数字、字符串、数组与普通对象。函数与值为 `undefined` 的属性会像 `JSON.stringify` 一样被丢弃，所以心智模型可以直接迁移过来。`NaN` 与 `Infinity` 没有 JSON 形式，会被拒绝而不是悄悄变成 `null`。嵌套深度上限 64 层——真实配置远达不到，而引用环立刻就会超过。

`get`、`set`、`remove`、`keys` 是同步的，这是刻意的：`get` 在 `render` 里也可达，所以值缓存在内存里，读取从缓存回答。每次渲染读一次文件是荒唐的。

**一次修改安排一次写入，而不是执行一次写入。** 文件在后台线程写出——先写临时文件再改名覆盖目标，所以写到一半崩溃留下的是之前完整的配置，而不是一个被截断的文件——并且同时只有一次写入在途，于是一连串 `set` 汇成一个文件，而不是一次一个文件。写入在途期间发生的改动，由下一次写入带上。

需要确认落盘时 `await store.flush()`。它是**屏障，不是第二个写入者**：等待此前所有修改抵达磁盘，写入失败时用写入自己的错误 reject。若让它自己再写一次，就会与自动写入抢同一个临时文件，两者之间没有任何顺序保证——旧版本可能最后落盘，把新版本抹掉。

### 存储在哪里

存储按应用划分，位置由宿主选择——应用不能指定自己的存储位置，否则两个应用可以故意撞在一起。

**宿主给应用起名字，数据跟着这个名字走：**

```rust
let data = gpui_shell::set_bundle_id("com.example.notes")?;
gpui_shell::set_capabilities(Capabilities::new().write_roots([data]));
```

| 平台              | 位置                                                                  |
| ----------------- | --------------------------------------------------------------------- |
| Linux 与其他 Unix | `$XDG_DATA_HOME/gpui-shell/apps/<id>/store.json`，默认 `~/.local/share` |
| macOS             | `~/Library/Application Support/gpui-shell/apps/<id>/store.json`         |
| Windows           | `%APPDATA%\gpui-shell\apps\<id>\store.json`                            |

id 就是身份，所以目录被改名、被移动、被一次升级整个替换掉，数据都还在——这正是用户说"我的设置"时指的东西。改用路径作 key，一次升级就等于悄悄让用户从头开始。

**运行时不会去某个文件里找这个 id。** 只有安装了这个应用的那一层知道它叫什么；运行时自己挑一个 manifest 去读，等于对一件不属于它的事情宣称权威。

被"指向"某个目录的宿主——这个命令行、一个 dev server——没有这样一个名字，而在那种情况下路径确实就是身份。`gpui_shell::bundle_id_for_path(root)` 用目录名加完整路径的摘要造一个，于是同一个目录总是访问到同一份数据，同一份源码的两个 checkout 也互不干扰。这在你正在编辑它时是对的，在它已经被安装之后是错的——而这正是声明一个真名字带来的区别。

id 允许 `a-z`、`0-9`、`.`、`-`、`_`，不允许 `..`。这不是整洁问题：它会被拼到用户数据目录后面，没检查的 id 能够到目录里的其他东西。数据放在那里而不是应用内部，因为应用目录可能只读、往往是一个 git checkout，也不是用户预期自己数据所在的地方。

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
    log.warn(
      `todolist: storage unavailable, starting empty (${error.message})`,
    );
    return [];
  }
}
```

示例的页脚随后会在界面上说明这一点——“Not saved — this host did not grant storage, so the list lasts for this run only”——这才是对的形态：在边界处吸收拒绝，并对用户说实话。

## `clipboard`

```js
import { clipboard } from "gpui";

clipboard.write_text("copied");
const text = clipboard.read_text(); // 剪贴板中没有文本时为 undefined
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
import { process } from "gpui"; // 同时也是一个全局

const { code, stdout, stderr } = await process.run("git", ["status"]);
process.exit(0);
```

`process.run` 返回 promise，理由是 `fs` 那条的加强版。文件读取没有时间上界；子进程连上界的影子都没有——它可以算上几分钟、等一个永远不来的输入，甚至活得比窗口还久。在这个线程上等它，会把帧和 VM 一起卡住，而且卡在内核里，interrupt budget 看不见。

输出是**捕获的，不是继承的**：跑一条命令的脚本几乎总是想要它说了什么，而在一个窗口程序里，子进程往宿主的 stdout 写，是写到没人会看的地方。`code` 成功时是 `0`，被信号杀死时是 `-1`——那种情况本来就没有退出码。

它受执行授权约束，授权有三种形态：拒绝（默认）、命令名白名单，或不受限。被拒绝的命令**在调用处抛出**而不是 reject，和被拒绝的 `fs` 路径一样——没人 await 的 rejected promise，等于没人看见的拒绝。

`process.exit` 在运行时内部是**一个请求，绝不是 `exit(2)`**。它把退出码交给宿主安装的处理函数，由后者决定怎么做——关闭插件的面板、关闭窗口、结束进程。一个插件不能把宿主进程带走，而宿主可能还有未保存的状态。

处理函数不是可选的：授予了这项能力却没有安装处理函数的宿主，会让这次调用**直接失败**并指明是宿主漏了什么。没人应答的请求比拒绝更糟，因为脚本分辨不出这两者。`gpui-shell` 这个二进制安装的是「宿主本身就是进程」时该有的策略——按脚本要求的退出码结束进程。

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

| 上限                                       | 值                                                                         |
| ------------------------------------------ | -------------------------------------------------------------------------- |
| 堆                                         | 256 MiB——泄漏表现为一个可捕获的 JavaScript 异常，而不是整个宿主被 OOM kill |
| 解释器栈                                   | 1 MiB——深递归表现为 `RangeError`，而不是原生栈溢出                         |
| 单次调用耗时：render 与 layout             | 50 ms                                                                      |
| 单次调用耗时：event 与 task                | 500 ms                                                                     |
| 单次调用耗时：不在任何调用中，例如模块求值 | 5 秒                                                                       |

时钟在每一次宿主调用时重置，这正是渲染路径能比事件回调有更紧预算的原因。**中断无法被 `catch` 吞掉**——这一点有测试来度量，因为如果能被吞掉，中断就根本不是一道防线。

这里没有 `std` 也没有 `os`：quickjs-libc 从一开始就没有被编进这个构建。

::: warning 开发模式还没有接完
`--dev` 目前只启用源码监听。它本该打开的放宽项——恢复 `eval`、让内建原型保持可写，这是 REPL 需要的——还无法从二进制里触达，它会打印一条警告说明。库函数是存在的（`gpui_shell::set_development_mode`），并且必须在运行时构造之前调用，因为策略是在创建上下文时读取的。

开发模式从不放宽能力约束。它让语言更好摆弄，但不会发出任何人没有声明过的访问权限——因为一项作者从没写下来的授权，就是一项在生产环境里会缺失的授权。
:::

## 还没有的东西

- **`gpui.http`（落地时返回 promise —— 一个 socket 可以花上一分钟，而文件系统已经演示过发布一个阻塞接口的代价）。** 能力模型里有 `capabilities.network.hosts`，`fetch` 的拒绝信息也提到了它，但没有 HTTP 接口。
- **Manifest，以及它所属的插件模型。** 今天授权来自宿主。
- **`process.exit` 自己的那项能力。**
- **向用户询问授权。** 授权在应用加载之前就已决定，不会在使用的那一刻弹出询问。
