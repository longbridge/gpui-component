## 项目上下文摘要（终端功能增强）
生成时间：2026-03-14 20:34:13

### 1. 相似实现分析
- **实现1**: main/src/setting_tab.rs:78
  - 模式：全局配置 AppSettings + SettingPage/SettingGroup 生成设置项
  - 可复用：SettingField::number_input / SettingField::switch
  - 需注意：设置变更通过 AppSettings::save() 持久化

- **实现2**: main/src/home/home_tabs.rs:14
  - 模式：HomePage 负责将 AppSettings 应用到 TerminalView，并订阅 TerminalViewEvent 写回持久化
  - 可复用：apply_terminal_font_size / bind_terminal_font_persistence
  - 需注意：事件订阅使用 cx.subscribe 并写入 AppSettings

- **实现3**: crates/terminal_view/src/view.rs:360
  - 模式：TerminalView 订阅 TerminalSidebarEvent，在 handle_sidebar_event 内集中处理设置变更
  - 可复用：事件链路 SettingsPanelEvent -> TerminalSidebarEvent -> TerminalView
  - 需注意：render 中通过 on_mouse_down/on_mouse_up 绑定鼠标行为

### 2. 项目约定
- **命名约定**：Rust 结构体/方法使用 snake_case，事件枚举使用 PascalCase
- **文件组织**：UI 设置页在 main/src/setting_tab.rs，终端视图在 crates/terminal_view/src/view.rs，侧边栏设置在 crates/terminal_view/src/sidebar/settings_panel.rs
- **导入顺序**：标准库 -> 外部 crate -> 本地 crate
- **代码风格**：显式注释描述意图，使用 t!("...") 做多语言文本

### 3. 可复用组件清单
- `main/src/setting_tab.rs`: AppSettings 全局配置与设置页构建
- `main/src/home/home_tabs.rs`: 终端视图设置应用与持久化订阅
- `crates/terminal_view/src/sidebar/settings_panel.rs`: 终端侧边栏设置事件与 UI 渲染

### 4. 测试策略
- **测试框架**: 未发现 *.spec.* / *.test.*
- **测试模式**: 目前仅可执行手动验证
- **参考文件**: 无
- **覆盖要求**: 需补充手动验证步骤并记录

### 5. 依赖和集成点
- **外部依赖**: gpui / gpui_component（鼠标事件与控件）
- **内部依赖**: TerminalView <-> TerminalSidebar <-> SettingsPanel
- **集成方式**: 事件链路触发设置变更，AppSettings 负责持久化
- **配置来源**: settings.json（由 AppSettings::save() 写入）

### 6. 技术选型理由
- **为什么用这个方案**: 复用已有的设置持久化与侧边栏事件链路，改动最小、逻辑集中
- **优势**: 不引入新模块，保持一致的 UI 与事件模式
- **劣势和风险**: 依赖已有事件订阅与 UI 结构，新增字段需覆盖到所有入口

### 7. 关键风险点
- **并发问题**: 订阅回调中读写 AppSettings 需避免借用冲突
- **边界条件**: 鼠标中键事件需确保仅在启用时生效
- **性能瓶颈**: 自动复制需避免空字符串写剪贴板
- **安全考虑**: 不新增安全逻辑，仅扩展已有设置
