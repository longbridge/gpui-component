# OnetCli

OnetCli (One Net Client) 是一款跨平台桌面客户端，基于 [GPUI](https://gpui.rs) 构建，为数据库管理、SSH/SFTP、终端与 AI 工具提供统一入口。

## 功能特性

- **多协议数据库管理**：PostgreSQL、MySQL、SQLite、SQL Server、Oracle、ClickHouse
- **Redis 与 MongoDB** 专项视图
- **SSH 终端与 SFTP 文件管理**
- **本地终端集成** 与多标签页工作流
- **Dock 面板系统**：支持拖拽、缩放、分屏布局与持久化
- **连接管理**：工作区组织、搜索筛选
- **云端同步与账号体系**：含密钥加密存储（AES-GCM、Ed25519）
- **内置 AI 对话入口**：OnetCli Provider（基于 `llm-connector`，支持流式输出）
- **主题系统**：支持亮色/暗色模式切换
- **国际化**：支持 English、简体中文、繁体中文

## 平台支持

| 平台 | 架构 | 渲染后端 | 备注 |
|------|------|---------|------|
| macOS | aarch64, x86_64 | Metal | — |
| Linux | x86_64 | Vulkan | 客户端侧装饰，需要 GTK 依赖 |
| Windows | x86_64 | — | 配置 8MB 栈大小（`.cargo/config.toml`）|

## 快速开始

### 依赖要求

- Rust（workspace 使用 2024 edition）
- GPUI 运行依赖（见下方各平台安装说明）

### 安装系统依赖

**Linux / macOS：**

```bash
./script/bootstrap
```

**Windows（PowerShell）：**

```powershell
.\script\install-window.ps1
```

该脚本会安装 Visual Studio 2022 Community（Native Desktop 工作负载）和 CMake。

### 运行应用

```bash
cargo run -p main
```

### 构建与测试

```bash
# 构建
cargo build

# 运行全部测试
cargo test --all

# 运行特定 crate 的测试
cargo test -p gpui-component
cargo test -p db
cargo test -p one-core

# 运行文档测试
cargo test -p gpui-component --doc
```

### 代码检查

```bash
# Lint
cargo clippy -- --deny warnings

# 格式检查
cargo fmt --check

# 拼写检查
typos

# 未使用依赖检查
cargo machete
```

### 性能分析

```bash
# macOS Metal HUD（查看 FPS 等指标）
MTL_HUD_ENABLED=1 cargo run -p main

# 使用 Samply 进行详细性能分析
samply record cargo run -p main
```

## 项目结构

Workspace 包含 4 个 default members 和 25+ 个 crate。

### 应用层

| 目录 | 说明 |
|------|------|
| `main/` | 应用入口与主界面，编排认证、设置、授权、更新、首页等子系统 |

### 核心基础设施

| Crate | 包名 | 说明 |
|-------|------|------|
| `crates/core` | one-core | 连接管理、云端同步、AI 集成、配置、加密 |
| `crates/ui` | gpui-component | 可复用 UI 组件库（60+ 组件），已发布到 crates.io |
| `crates/one_ui` | one-ui | 应用专用 UI 组件，扩展 gpui-component |
| `crates/macros` | gpui-component-macros | 过程宏 |
| `crates/assets` | gpui-component-assets | 打包静态资源（rust-embed） |

### 功能 Crate（后端 + 视图配对）

| 领域 | 后端 Crate | 视图 Crate |
|------|-----------|-----------|
| 数据库 | `crates/db` | `crates/db_view` |
| 终端 | `crates/terminal` | `crates/terminal_view` |
| SSH/SFTP | `crates/ssh`、`crates/sftp` | `crates/sftp_view` |
| Redis | — | `crates/redis_view` |
| MongoDB | — | `crates/mongodb_view` |

### 工具 Crate

| Crate | 说明 |
|-------|------|
| `crates/reqwest_client` | 基于 Zed 定制 reqwest fork 的 HTTP 客户端封装 |
| `crates/webview` (gpui-wry) | 通过 Wry 集成 WebView |
| `crates/license_tool` | 许可证密钥生成与管理 |
| `crates/story` | 组件展示画廊 / Showcase 应用 |

## 示例

`examples/` 目录提供了多个独立示例，每个示例聚焦展示一个功能点：

```bash
# 查看可用示例
ls examples/

# 运行特定示例
cargo run --example hello_world
```

| 示例 | 说明 |
|------|------|
| `hello_world` | 基础 GPUI 应用 |
| `input` | 输入组件使用 |
| `window_title` | 窗口管理 |
| `dialog_overlay` | 对话框实现 |
| `webview` | WebView 集成（Wry）|
| `system_monitor` | 系统监控 |
| `focus_trap` | 焦点管理 |
| `app_assets` | 自定义资源/图标嵌入 |

## 组件展示（Story）

`crates/story` 是一个组件画廊应用，可以在界面中浏览和测试所有 GPUI 组件：

```bash
# 运行组件画廊
cargo run

# 运行单个 story 示例
cargo run --example dock
cargo run --example editor
cargo run --example markdown
```

## 配置

### 环境变量

开发环境按优先级加载环境文件：`.env.local`（优先）→ `.env`（回退）。

可配置项：
- `SUPABASE_URL` — Supabase 后端地址（可编译期内嵌，也可运行时覆盖）
- `SUPABASE_ANON_KEY` — Supabase 匿名密钥
- `ONETCLI_UPDATE_URL` — 自动更新地址

### Windows 栈大小

Windows 平台通过 `.cargo/config.toml` 配置 8MB 栈大小：

```toml
[target.x86_64-pc-windows-msvc]
rustflags = ["-C", "link-arg=/STACK:8000000"]
```

## 技术栈

### 框架与渲染

- **[GPUI](https://gpui.rs)** — 来自 Zed 编辑器的高性能 GPU 加速 UI 框架
- **Metal** (macOS) / **Vulkan** (Linux) — 原生 GPU 渲染后端

### 数据库驱动

- **PostgreSQL** — tokio-postgres / deadpool-postgres
- **MySQL** — mysql_async
- **SQLite** — rusqlite（bundled）
- **SQL Server** — tiberius + bb8
- **Oracle** — oracle
- **ClickHouse** — clickhouse
- **Redis** — redis（tokio-comp, cluster-async）
- **MongoDB** — mongodb

### 网络与安全

- **SSH/SFTP** — russh、russh-sftp、russh-keys
- **HTTP** — Zed 定制的 reqwest fork (zed-reqwest)
- **加密** — aes-gcm、sha2、ed25519

### 终端

- **终端仿真** — alacritty_terminal
- **本地 PTY** — portable-pty

### 文本与编辑

- **Rope 数据结构** — ropey
- **语法高亮** — tree-sitter
- **SQL 解析** — sqlparser、sqlformat

### AI

- **LLM 连接器** — llm-connector（支持流式输出）

### 国际化

- **rust-i18n** — 支持 `en`、`zh-CN`、`zh-HK`，语言文件位于 `crates/ui/locales/`

## CI/CD

项目使用 GitHub Actions 进行持续集成与发布：

- **ci.yml** — 主测试流水线，在 macOS (aarch64)、Linux (x86_64)、Windows (x86_64) 三个平台运行：
  - `cargo clippy -- --deny warnings`（macOS）
  - `cargo machete`（macOS）
  - `typos` 拼写检查（macOS）
  - `cargo test --all` 全平台测试
- **build-release.yml** — 版本标签（`v*`）触发或手动触发，构建全平台二进制并发布 GitHub Release：
  - 构建目标：macOS (aarch64, x86_64)、Linux (x86_64)、Windows (x86_64)
  - 自动打包（tar.gz / zip）、生成 SHA256 校验和
  - 创建 GitHub Release 并上传所有产物
- **release.yml** — 版本标签触发，自动发布到 crates.io
- **release-docs.yml** — 自动构建并部署 VitePress 文档到 GitHub Pages
- **test-docs.yml** — 文档变更时触发文档构建测试

## 贡献

请参阅 [CONTRIBUTING.md](CONTRIBUTING.md) 了解详细的贡献指南，包括：

- 代码风格与组织规范
- AI 生成代码的披露要求
- 开发环境搭建
- UI 设计参考指南
- 性能测试方法
- 版本发布流程

## 许可证

本项目采用 OnetCli License，详见 [ONETCLI_LICENSE](ONETCLI_LICENSE)。

允许个人及商业使用，但禁止二次分发、转售或基于本软件创建竞争性产品。如有许可证与版权相关问题，请联系 xiaofei.hf@gmail.com。
