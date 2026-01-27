# Focus Trap Example

这个示例展示了如何使用 `.focus_trap()` 方法来限制焦点在特定容器内循环。

## 什么是 Focus Trap？

Focus Trap 是一种无障碍功能，确保键盘焦点（通过 Tab 键导航）被限制在特定的容器内。当用户在一个启用了 focus trap 的容器内按 Tab 键时，焦点会在容器内的可聚焦元素间循环，而不会跳出到父级或兄弟元素。

这对于模态对话框、侧边栏、弹出菜单等场景特别有用，可以防止用户意外地将焦点移到被遮挡的背景内容上。

## 使用方法

```rust
use gpui_component::*;

impl MyView {
    fn render(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        // 构建内容并应用 focus trap
        v_flex()
            .child(Button::new("btn1").label("Button 1"))
            .child(Button::new("btn2").label("Button 2"))
            .child(Button::new("btn3").label("Button 3"))
            .focus_trap("my-trap", cx)  // 就这么简单！
        // 按 Tab 键会在这 3 个按钮间循环，不会跳出
    }
}
```

### 重要提示

- **`.focus_trap()` 必须在构建完所有子元素之后调用**
- 不需要手动管理 focus handles，系统会自动处理
- Focus trap 会拦截 Tab/Shift-Tab 键，使焦点在容器内循环
- 适用于任何包含可聚焦元素的容器（Dialog, Sheet, Popover 等）

## 运行示例

```bash
cargo run --example focus_trap
```

## 示例说明

示例程序展示了三种不同的区域：

1. **外部区域（无 focus trap）** - 按 Tab 键会正常导航到下一个可聚焦元素
2. **Focus Trap 区域 1** - 灰色背景区域，包含 3 个按钮，焦点会在这些按钮间循环
3. **Focus Trap 区域 2** - 蓝色背景区域，包含 4 个按钮，焦点会在这些按钮间循环

尝试在不同区域间按 Tab 键，观察焦点如何在 focus trap 容器内循环。

## 技术实现

`.focus_trap()` 的实现原理：

1. **全局管理器**：使用 `FocusTrapManager` 全局状态管理所有 focus trap 容器
2. **注册机制**：在元素渲染时自动注册容器的 focus handle
3. **Tab 拦截**：Root 组件拦截 Tab/Shift-Tab 键事件
4. **动态循环**：当检测到焦点即将跳出 trap 时，自动循环回容器内的第一个/最后一个可聚焦元素

### 关键优势

- **零配置**：不需要手动收集或管理 focus handles
- **自动化**：系统自动检测并处理焦点循环
- **灵活**：适用于任何类型的容器和可聚焦元素

### 关键代码位置

- `crates/ui/src/focus_trap.rs` - FocusTrapElement 和管理器实现
- `crates/ui/src/element_ext.rs` - `.focus_trap()` 方法定义
- `crates/ui/src/root.rs` - Root 中的 Tab 事件处理和焦点循环逻辑
- `examples/focus_trap/` - 完整使用示例

参考：
- [focus-trap-react](https://github.com/focus-trap/focus-trap-react) - 启发此功能的 React 库
