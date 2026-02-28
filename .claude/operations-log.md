## 编码前检查 - release-workflow-migration
时间：2026-02-28 14:24:04 +0800

- 已查阅上下文摘要文件：`.claude/context-summary-release-workflow-migration.md`
- 复用组件：
  - `script/bootstrap`：系统依赖安装
  - `script/bundle-macos.sh`：macOS 打包
- 命名约定：沿用 workflow/job/step 命名风格
- 代码风格：沿用现有 YAML 缩进与动作版本
- 不重复造轮子证明：基于已有 build-release/release/ci 三个 workflow 合并

## 编码后声明 - release-workflow-migration
时间：2026-02-28 14:24:04 +0800

### 1. 复用组件
- `script/bootstrap`：保持 Linux/macOS 依赖安装入口不变
- `script/bundle-macos.sh`：保持 macOS app bundle 打包逻辑不变

### 2. 遵循约定
- 命名约定：保留 `release.yml`，job 名称使用 `build/release/publish_crate`
- 代码风格：维持 actions 版本与缓存结构一致
- 文件组织：仅修改 `.github/workflows` 与 README

### 3. 相似实现对比
- 对比 build-release：迁入矩阵构建、打包、checksum、GitHub Release 上传
- 对比旧 release：保留 crates 发布职责并增加保护条件

### 4. 未重复造轮子证明
- 复用了已有脚本与矩阵配置，未新增自定义打包脚本或额外发布工具