# `readonly` / `readOnly` / `read_only` API 命名调研

调研日期：2026-08-14

## 结论

建议 `gpui-component` **继续使用 `readonly`**，包括：

```rust
Input::new(state).readonly(true);
state.set_readonly(true, cx);
```

理由：

1. Rust 公共标识符必须使用 `snake_case`，但这并不意味着每个自然语言复合词都必须拆开。Rust 标准库本身已经把该概念拼作 [`Permissions::readonly()`](https://doc.rust-lang.org/std/fs/struct.Permissions.html#method.readonly) 和 [`Permissions::set_readonly()`](https://doc.rust-lang.org/std/fs/struct.Permissions.html#method.set_readonly)。因此 `readonly` / `set_readonly` 都符合 Rust 的实际先例。
2. HTML 和多个声明式框架把它视为一个稳定的术语：标记层使用 `readonly`，JavaScript、Dart、Kotlin API 使用 `readOnly`。在 Rust 中转成小写后自然就是 `readonly`。
3. 当前项目已经公开或内部使用 `readonly`, `set_readonly` 与 `.readonly(true)`；改为 `read_only` 会制造迁移成本，却没有得到跨生态惯例或 Rust 标准库先例的支持。

建议在代码标识符中使用 `readonly`，在英文文档叙述中使用形容词 `read-only`。不建议新增 `read_only` 别名。

## 调研范围与判定方法

这里只记录规范、项目官方文档或项目官方源码。对 Vue、Svelte 这类直接渲染原生元素的框架，需要把其“原生 HTML attribute 透传/书写规则”和 HTML 规范一起看；这不是框架另行定义的组件 prop。

| 生态 | 面向使用者的拼写 | 一手资料与判定 |
|---|---|---|
| HTML 标记 | `readonly` | HTML Standard 将输入控件的布尔 content attribute 定义为 [`readonly`](https://html.spec.whatwg.org/multipage/input.html#attr-input-readonly)，示例为 `<input readonly>`。 |
| DOM | `readOnly` | 同一规范的 [`HTMLInputElement`](https://html.spec.whatwg.org/multipage/input.html#htmlinputelement) IDL 定义 `attribute boolean readOnly`；因此 JavaScript 写 `input.readOnly = true`。这说明标记拼写和编程语言属性拼写本来就不同。 |
| React DOM | `readOnly` | React 官方 [`<input>` reference](https://react.dev/reference/react-dom/components/input) 把 `readOnly` 列为布尔 prop，并示例 `<input value={something} readOnly={true} />`。 |
| Vue | 原生元素上为 `readonly` | Vue 官方说明模板 attribute 的基本写法和 `v-bind` 缩写（[`Attribute Bindings`](https://vuejs.org/guide/essentials/template-syntax.html#attribute-bindings)）。用于原生 `<input>` 时，attribute 本身由 HTML Standard 定义为 [`readonly`](https://html.spec.whatwg.org/multipage/input.html#attr-input-readonly)，所以静态写 `readonly`，动态写 `:readonly="flag"`；不是 `read_only`。 |
| Angular | HTML/模板为 `readonly`；Signals Forms API 为 `readonly(...)` | 原生 attribute 仍是 HTML 的 [`readonly`](https://html.spec.whatwg.org/multipage/input.html#attr-input-readonly)。Angular 官方 Signals Forms 文档使用 [`readonly(schemaPath.username)`](https://angular.dev/guide/forms/signals/field-state-management#readonly-fields)，字段状态访问为 `readonly()`，并说明 `formField` 会自动绑定 `readonly` attribute。 |
| Svelte | 原生元素上为 `readonly` | Svelte 官方 [`Attributes`](https://svelte.dev/docs/svelte/basic-markup#Attributes) 说明元素 attribute 可按 HTML 方式书写，也可用表达式赋值。结合 HTML Standard 的 [`readonly`](https://html.spec.whatwg.org/multipage/input.html#attr-input-readonly)，写法是 `readonly` 或 `readonly={flag}`；不是 `read_only`。 |
| Flutter | `readOnly` | Flutter 官方 [`TextField.readOnly`](https://api.flutter.dev/flutter/material/TextField/readOnly.html) 声明为 `final bool readOnly`；构造参数也是 `TextField(readOnly: true)`。 |
| Jetpack Compose | `readOnly` | AndroidX 官方 `TextField.kt` 源码的 [`TextField` 参数](https://github.com/androidx/androidx/blob/androidx-main/compose/material3/material3/src/commonMain/kotlin/androidx/compose/material3/TextField.kt) 是 `readOnly: Boolean = false`，KDoc 也以 `readOnly` 描述只读状态。 |
| SwiftUI | 没有同名的通用 `TextField` 参数 | Apple 官方 [`TextField`](https://developer.apple.com/documentation/swiftui/textfield) 把它定义为可编辑文本接口，公开 initializer 中没有 `readOnly` / `read_only` 参数。Apple 的 [`EditMode`](https://developer.apple.com/documentation/swiftui/editmode) 示例在非编辑状态改为展示 `Text`；官方教程也展示对 `TextField` 使用 [`.disabled(...)`](https://developer.apple.com/tutorials/swiftui/working-with-ui-controls)。`disabled` 会禁用交互，并不等同于 Web 中“仍可聚焦/选择”的 readonly 语义。 |
| egui | 没有同名 builder；用 `interactive` 或只读 buffer 表达 | egui 官方 API 文档说明 [`TextEdit::interactive(false)`](https://docs.rs/egui/latest/egui/widgets/text_edit/struct.TextEdit.html#method.interactive) 会禁止用户选择文本；若要“可选择但不可编辑”，[`TextEdit`](https://docs.rs/egui/latest/egui/widgets/text_edit/struct.TextEdit.html) 文档建议传入 `&mut &str`。它没有为该语义选择 `readonly` 或 `read_only`。 |
| iced | 没有同名属性；通过是否提供编辑消息表达 | iced 官方 [`TextInput::on_input`](https://docs.rs/iced/latest/iced/widget/struct.TextInput.html#method.on_input) 文档说明，不调用该方法会产生 disabled `TextInput`。它没有独立的 readonly 命名，因此不能作为 `read_only` 的先例。 |
| Slint | Slint DSL 为 `read-only` | Slint 官方 [`LineEdit`](https://docs.slint.dev/latest/docs/slint/reference/std-widgets/views/lineedit/#read-only) 定义 `read-only: bool`，语义是仍可选择和复制但不能编辑。Slint 标识符采用 kebab-case，不能直接推导 Rust API 应写成 `read_only`。 |

## 跨生态规律

可归纳为三层命名：

- 标记或 DSL：HTML/Vue/Svelte/Angular 使用 `readonly`，Slint 使用其 DSL 风格的 `read-only`。
- camelCase 语言 API：DOM、React、Flutter、Jetpack Compose 使用 `readOnly`。
- Rust：被调查的 UI 库没有形成统一属性名；真正相关的 Rust 标准库先例明确采用 `readonly()`。

因此，“Rust 一律把 `readOnly` 机械转换成 `read_only`”并不是可靠规则。Rust API Guidelines 的 [C-CASE](https://rust-lang.github.io/api-guidelines/naming.html#c-case) 要求函数和方法使用 `snake_case`；`readonly` 本身不含大写字母，也没有违反该规则，而标准库先例进一步消除了歧义。

## 对 `gpui-component` 的具体建议

| 场景 | 推荐 | 不推荐 |
|---|---|---|
| Builder 方法 | `readonly(bool)` | `read_only(bool)` |
| 状态 setter | `set_readonly(bool, cx)` | `set_read_only(bool, cx)` |
| 字段/能力标志 | `readonly: bool` | `read_only: bool` |
| 英文说明文字 | “read-only input/mode” | “readonly input/mode” |
| 与 DOM 交互的 JavaScript | `node.readOnly` | `node.readonly`, `node.read_only` |

如果未来需要表达“当前内容能否被编辑”而非一个 HTML 式模式，也可以单独考虑正向命名 `editable` / `is_editable`。这属于不同的 API 设计，不应与 `readonly` / `read_only` 的拼写迁移混在一起。
