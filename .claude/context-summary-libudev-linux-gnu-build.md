## 项目上下文摘要（libudev-linux-gnu-build）
生成时间：2026-03-20 15:02:02 +0800

### 1. 相似实现分析
- 实现1：`.github/workflows/release.yml`
  - 模式：按目标平台矩阵构建，并通过 `script/bootstrap` 统一安装 Linux/macOS 系统依赖
  - 可复用：Linux 构建前置依赖安装入口
  - 注意点：Linux GNU release 构建不会单独安装 `libudev-dev`
- 实现2：`.github/workflows/ci.yml`
  - 模式：CI 也复用 `script/bootstrap` 作为非 Windows 环境的依赖安装入口
  - 可复用：同一脚本服务于 CI 与 release，适合集中修复
  - 注意点：不应在 workflow 内重复写一份 apt 安装逻辑
- 实现3：`script/install-linux.sh`
  - 模式：单一 `apt install -y` 清单维护 Ubuntu 构建依赖
  - 可复用：Linux 原生库安装集中点
  - 注意点：当前缺少 `libudev-dev`
- 实现4：`crates/terminal_view/src/serial_form_window.rs`
  - 模式：UI 直接调用 `serialport::available_ports()` 做串口枚举
  - 可复用：证明现有产品能力依赖 `serialport` 默认 Linux 枚举功能
  - 注意点：不能简单改成 `serialport --no-default-features`

### 2. 项目约定
- 命名约定：workflow 与脚本文件使用 kebab-case / shell 常规命名
- 文件组织：CI/CD 位于 `.github/workflows/`，系统依赖脚本位于 `script/`
- 代码风格：YAML 两空格缩进；shell 安装脚本使用连续包列表，不做复杂逻辑分支

### 3. 可复用组件清单
- `script/bootstrap`：Linux/macOS 依赖安装统一入口
- `script/install-linux.sh`：Ubuntu 构建依赖集中清单
- `crates/terminal/src/serial_backend.rs`：串口连接逻辑，证明 `serialport` 为真实运行依赖
- `crates/terminal_view/src/serial_form_window.rs`：串口枚举逻辑，证明 Linux `libudev` 能力不能随意关闭

### 4. 测试策略
- 静态验证 workflow 仍通过 `script/bootstrap` 调用统一脚本
- 使用 `cargo tree -i libudev-sys --target x86_64-unknown-linux-gnu -p main` 校验依赖链
- 当前主机是 macOS，无法本地直接执行 Ubuntu GNU 完整构建；最终闭环依赖 GitHub Linux job 或 Ubuntu 本机验证

### 5. 依赖和集成点
- 依赖链：`libudev-sys -> libudev -> serialport -> terminal/terminal_view -> main`
- 触发入口：`.github/workflows/release.yml` 与 `.github/workflows/ci.yml` 的 Linux job
- 安装路径：`script/bootstrap -> script/install-linux.sh`

### 6. 技术选型理由
- 选择补齐 `libudev-dev`，因为报错已经证明 `pkg-config` 可执行但找不到 `libudev.pc`
- 不调整 Rust feature，因为 `serialport-rs` 官方文档明确 `--no-default-features` 会去掉 `libudev`，而项目当前需要 Linux 串口枚举能力
- 选择只改安装脚本，不改 workflow 结构，保持 CI/release 入口一致

### 7. 关键风险点
- 当前环境不是 Ubuntu，无法本地完整复现 GitHub Linux GNU 构建
- 若未来移除串口枚举功能，可再评估 feature-gate；当前不是构建修复范围
- 若 runner 镜像预装包发生变化，显式声明 `libudev-dev` 仍比依赖镜像默认状态更稳
