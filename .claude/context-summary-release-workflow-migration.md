## 项目上下文摘要（release-workflow-migration）
生成时间：2026-02-28 14:24:04 +0800

### 1. 相似实现分析
- 实现1：.github/workflows/build-release.yml
  - 模式：矩阵构建 + 多平台打包 + GitHub Release 上传
  - 可复用：目标平台矩阵、打包命名、checksum 生成
  - 注意点：依赖 `script/bootstrap` 与 `script/bundle-macos.sh`
- 实现2：.github/workflows/release.yml
  - 模式：tag 触发后执行 `cargo publish`
  - 可复用：crates 发布入口
  - 注意点：原实现 `--workspace` 与 publish=false 包冲突
- 实现3：.github/workflows/ci.yml
  - 模式：跨平台 Rust 工具链 + 缓存 + 脚本依赖安装
  - 可复用：setup-rust-toolchain、actions/cache 使用方式

### 2. 项目约定
- 命名约定：workflow 文件使用 kebab-case
- 文件组织：CI/CD 在 `.github/workflows/`
- 代码风格：YAML 两空格缩进，步骤命名清晰

### 3. 可复用组件清单
- `script/bootstrap`：Linux/macOS 依赖安装
- `script/bundle-macos.sh`：macOS app bundle 打包
- `resources/linux/*`：Linux 包资源

### 4. 测试策略
- 通过静态审查验证 workflow 触发条件、job 依赖和 secrets 条件
- 通过 `cargo publish --help` 校验参数可用性

### 5. 依赖和集成点
- GitHub Actions：`actions/checkout`、`setup-rust-toolchain`、`actions/cache`
- 发布动作：`softprops/action-gh-release`
- secrets：`SUPABASE_URL`、`SUPABASE_ANON_KEY`、`CARGO_REGISTRY_TOKEN`

### 6. 技术选型理由
- 选择单一 `release.yml` 承载发布，避免双 workflow 并发与职责重叠
- 保留原构建矩阵和产物命名，降低迁移风险
- crates 发布改为显式包列表，规避 publish=false 成员导致失败

### 7. 关键风险点
- 未配置 `CARGO_REGISTRY_TOKEN` 时 crates 不会发布
- crates 发布顺序若不正确可能受依赖可见性影响
- tag 版本与 crate 版本不一致会导致发布失败