## 项目上下文摘要（OnetCli 官网）
生成时间：2026-03-28 14:32:17 +0800

### 1. 相似实现分析
- **实现 1**: `docs/index.md` + `docs/index.vue`
  - 模式：Markdown 入口挂载 Vue 单文件组件，首页自定义布局。
  - 可复用：首页入口模式、Tailwind + CSS 变量写法、VitePress 自定义首页。
  - 需注意：当前内容仍是 gpui-component 文案，必须整体替换。

- **实现 2**: `docs/contributors.md` + `docs/contributors.vue`
  - 模式：独立营销型页面由 `.md` 负责路由，`.vue` 负责视觉呈现。
  - 可复用：页面拆分方式、局部样式作用域、语义清晰的内容块组织。
  - 需注意：页面仍沿用现有 docs 主题外壳，新增官网页面时应保持一致。

- **实现 3**: `.github/workflows/release-docs.yml` + `.github/workflows/test-docs.yml`
  - 模式：`docs` 目录使用 Bun 安装依赖，构建产物为 `docs/.vitepress/dist`，并已有 GitHub Pages 上传流程。
  - 可复用：Bun 构建命令、`configure-pages` / `upload-pages-artifact` / `deploy-pages` 链路。
  - 需注意：`release-docs.yml` 当前监听 `Release Crate`，与仓库里的 `Release` 工作流名不一致，需要修正触发策略。

### 2. 项目约定
- **命名约定**：配置文件与 Vue 组件使用英文标识符；面对用户的文案统一使用简体中文。
- **文件组织**：站点配置集中在 `docs/.vitepress/`，页面内容位于 `docs/` 根目录或 `docs/docs/` 子目录。
- **导入顺序**：现有 VitePress 配置和 Vue 组件采用先框架依赖、后本地依赖的顺序。
- **代码风格**：VitePress 配置使用 TypeScript；Vue 页面使用 `<template> + <script setup> + <style lang=\"scss\">`；样式大量复用 Tailwind 原子类与 CSS 变量。

### 3. 可复用组件清单
- `docs/.vitepress/config.mts`：站点标题、导航、页脚、`head`、`base` 等全局配置入口。
- `docs/.vitepress/theme/style.css`：全局视觉变量和 VitePress 默认主题覆盖点。
- `docs/index.vue`：自定义首页呈现方式，可直接改造成 OnetCli 官网首页。
- `.github/workflows/release-docs.yml`：GitHub Pages 部署基础链路。
- `.github/workflows/test-docs.yml`：docs 目录 PR 构建校验。

### 4. 测试策略
- **测试框架**：当前 docs 侧没有单元测试框架，已有最小验证方式是 `bun run build`。
- **测试模式**：以静态站构建校验为主，辅以工作流 YAML 静态检查和关键配置人工核对。
- **参考文件**：`.github/workflows/test-docs.yml`。
- **覆盖要求**：至少覆盖页面可构建、导航可解析、GitHub Pages 工作流产物路径正确。

### 5. 依赖和集成点
- **外部依赖**：`vitepress`、`tailwindcss`、`lucide-vue-next`、`vitepress-plugin-llms`、`vitepress-sidebar`。
- **内部依赖**：站点内容依赖 `README_CN.md` 中的产品事实与仓库内现有截图资源。
- **集成方式**：静态页面由 VitePress 编译输出；GitHub Pages 通过 Actions 从 `docs/.vitepress/dist` 部署。
- **配置来源**：`docs/package.json`、`docs/.vitepress/config.mts`、`.github/workflows/*.yml`。

### 6. 技术选型理由
- **为什么用这个方案**：仓库已有 VitePress 站点和 Pages 工作流，复用成本最低，最适合“展示第一、下载第二”的无状态官网。
- **优势**：静态输出快；SEO/GEO 友好；后续扩展更新日志成本低；GitHub Pages 托管简单。
- **劣势和风险**：需要清理 gpui-component 旧内容；GitHub Pages 项目页下 `base` 处理错误会导致资源路径失效。

### 7. 关键风险点
- **路径风险**：GitHub Pages 项目页需要正确 `base`，否则静态资源与跳转会失效。
- **品牌残留**：旧导航、页脚、元信息和文案较多，若清理不全会造成品牌混杂。
- **构建风险**：docs 构建依赖 Bun 安装与 VitePress 当前 alpha 版本，变更后必须本地构建验证。
- **内容风险**：SEO/GEO 需要真实、可抽取的产品表述，不能沿用组件库式文档结构。
