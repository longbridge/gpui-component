<p align="center">
  <img src="logo.svg" alt="OnetCli" width="120" />
</p>

<h1 align="center">OnetCli</h1>

<p align="center">
  跨平台桌面客户端，数据库、SSH、终端与 AI 一站式管理。
</p>

<p align="center">
  基于 <a href="https://gpui.rs">GPUI</a> 构建 · GPU 加速渲染 · 原生性能
</p>

<p align="center">
  <a href="README.md">English</a> ·
  <a href="#安装">安装</a> ·
  <a href="#功能特性">功能特性</a> ·
  <a href="#应用截图">截图</a> ·
  <a href="CONTRIBUTING.md">参与贡献</a>
</p>

---

<!-- 替换为实际截图 -->
<p align="center">
  <img src="app.png" alt="OnetCli 概览" width="800" />
</p>

## 功能特性

**多数据库管理** — 在同一界面连接 PostgreSQL、MySQL、SQLite、SQL Server、Oracle 和 ClickHouse。

**Redis** — 专用 Redis 视图，支持键浏览、值查看与集群连接。

**MongoDB** — MongoDB 浏览器，支持集合浏览、文档查看与查询。

**SSH 与 SFTP** — 集成 SSH 终端和 SFTP 文件管理器，支持密钥认证。

**本地终端** — 内置终端，支持多标签页工作流。

**AI 助手** — 应用内直接与 AI 对话，支持自然语言生成 SQL、查询解释、BI 数据分析与图表生成，基于流式 LLM 集成。

**云端同步** — 跨设备同步连接和设置，密钥加密存储（AES-GCM、Ed25519）。

**主题与国际化** — 亮色 / 暗色模式切换，支持 English、简体中文、繁体中文。

## 应用截图

<!-- 将下方占位图替换为实际截图 -->

| 数据库浏览器 | SQL 编辑器 |
|:-:|:-:|
| ![数据库浏览器](docs/public/screenshots/database-explorer.png) | ![SQL 编辑器](docs/public/screenshots/sql-editor.png) |

| 终端 | SSH 与 SFTP |
|:-:|:-:|
| ![终端](docs/public/screenshots/terminal.png) | ![SSH 与 SFTP](docs/public/screenshots/ssh-sftp.png) |

| Redis 视图 | AI 对话 |
|:-:|:-:|
| ![Redis](docs/public/screenshots/redis.png) | ![AI 对话](docs/public/screenshots/ai-chat.png) |

## 平台支持

| 平台 | 架构 | 渲染后端 |
|------|------|---------|
| macOS | aarch64, x86_64 | Metal |
| Linux | x86_64 | Vulkan |
| Windows | x86_64 | — |

## 安装

### 前置条件

- Rust（2024 edition）
- 各平台系统依赖（见下方说明）

### 系统依赖

**macOS / Linux：**

```bash
./script/bootstrap
```

**Windows（PowerShell）：**

```powershell
.\script\install-window.ps1
```

### 构建与运行

```bash
cargo run -p main
```

### macOS 常见问题

如果 macOS 安装 DMG 后提示无法打开（"Apple 无法检查其是否包含恶意软件"），请执行：

```bash
sudo xattr -rd com.apple.quarantine /Applications/OnetCli.app
```

### Oracle 支持

使用 Oracle 连接需要先安装 [Oracle Instant Client](https://www.oracle.com/database/technologies/instant-client/downloads.html)（Basic 包）。请下载与你平台对应的版本，并确保库文件在系统的库搜索路径中。

## 开发

```bash
# 构建
cargo build

# 测试
cargo test --all

# Lint
cargo clippy -- --deny warnings

# 格式检查
cargo fmt --check

# 拼写检查
typos
```

详细开发指南请参阅 [CONTRIBUTING.md](CONTRIBUTING.md)。

## 技术栈

| 类别 | 技术 |
|------|------|
| UI 框架 | [GPUI](https://gpui.rs)（来自 Zed 编辑器） |
| 数据库驱动 | tokio-postgres, mysql_async, rusqlite, tiberius, oracle, clickhouse, redis, mongodb |
| SSH/SFTP | russh, russh-sftp |
| 终端仿真 | alacritty_terminal |
| 文本编辑 | ropey, tree-sitter, sqlparser |
| AI | llm-connector（流式输出） |
| 加密 | aes-gcm, sha2, ed25519 |
| 国际化 | rust-i18n |

## 许可证

本项目基于 [Apache License 2.0](LICENSE-APACHE) 开源。

OnetCli 应用的分发与使用须同时遵守 [OnetCli 补充协议](ONETCLI_LICENSE)，该补充协议在 Apache 2.0 基础上增加了以下限制：

- 禁止二次分发、转售或将本软件作为独立产品再分发
- 禁止基于本软件代码创建竞争性产品或服务
- 禁止将本软件托管于未经授权的分发平台

如有许可证与版权相关问题，请联系 xiaofei.hf@gmail.com。
