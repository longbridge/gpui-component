# Workspace Window Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 将工作区创建/编辑从主页 dialog 改为独立窗口，同时保留“新建连接”类型选择器为 dialog。

**Architecture:** 新增轻量 `WorkspaceFormWindow` 组件承载输入与提交按钮；`HomePage::show_workspace_form` 改为通过 `open_popup_window` 打开该窗口；工作区保存逻辑改成显式接收 `workspace_id`，去掉对主页临时编辑状态的依赖。

**Tech Stack:** Rust, gpui, gpui-component, one_core popup window

---

### Task 1: 接入独立工作区窗口

**Files:**
- Create: `main/src/home/workspace_form_window.rs`
- Modify: `main/src/home/mod.rs`

- [ ] 定义 `WorkspaceFormWindowConfig`，包含 `parent: Entity<HomePage>`、`workspace_id: Option<i64>`、`initial_name: String`
- [ ] 实现 `WorkspaceFormWindow::new`，创建名称输入框并回填编辑态名称
- [ ] 实现窗口渲染：标题栏、名称输入、取消/保存按钮
- [ ] 保存按钮调用 `parent.update(...)` 回到 `HomePage` 执行保存，再关闭窗口

### Task 2: 将 HomePage 工作区入口改为新窗口

**Files:**
- Modify: `main/src/home_tab.rs`

- [ ] 导入 `WorkspaceFormWindow` 与配置类型
- [ ] 将 `show_workspace_form` 从 `open_dialog` 改为 `open_popup_window`
- [ ] 使用现有工作区数据构造编辑态窗口配置
- [ ] 保持“新建连接”选择器不变，仅让其点击“工作区”后进入新窗口

### Task 3: 清理保存状态并验证

**Files:**
- Modify: `main/src/home_tab.rs`

- [ ] 将 `handle_save_workspace` 改为显式接收 `workspace_id`
- [ ] 移除 `editing_workspace_id` 字段及其相关读写
- [ ] 运行定向编译验证，确认新增窗口接线无误
