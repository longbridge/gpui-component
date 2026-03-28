## 项目上下文摘要（OnetCli 官网二次修整）
生成时间：2026-03-28 15:45:04 +0800

### 1. 相似实现分析
- **实现 1**: `docs/index.md` + `docs/index.vue`
  - 模式：Markdown 入口挂载 Vue 单文件组件，自定义首页视觉和结构化数据都集中在 `index.vue`。
  - 可复用：首页入口、`withBase` 站内链接、Tailwind 原子类与局部样式写法。
  - 需注意：当前首页内仍残留错误 GitHub Releases 链接与偏重的 hero 视觉。

- **实现 2**: `docs/features.md`、`docs/download.md`、`docs/changelog.md`
  - 模式：最小官网内容页采用纯 Markdown 页面，保留清晰的 frontmatter 和简单正文结构。
  - 可复用：新增 `guide.md` 时应沿用相同文件组织与文案密度。
  - 需注意：`download.md`、`changelog.md` 当前也残留错误仓库地址。

- **实现 3**: `docs/.vitepress/config.mts` + `docs/.vitepress/theme/style.css` + `docs/.vitepress/theme/components/GitHubStar.vue`
  - 模式：VitePress 全局配置、主题覆盖样式和导航组件共同决定官网导航、页脚与顶部观感。
  - 可复用：继续沿用现有导航配置、主题变量和 GitHub 入口组件，不额外引入新的主题系统。
  - 需注意：配置中仍有 `hufei/onetcli`、错误文档外链和 `search.provider = "local"`。

### 2. 项目约定
- **命名约定**：配置和组件标识符使用英文，用户可见文案保持简体中文。
- **文件组织**：VitePress 站点内容在 `docs/` 根目录，配置在 `docs/.vitepress/`，测试在 `docs/tests/`。
- **导入顺序**：先第三方依赖，后本地模块。
- **代码风格**：Vue 页面保持 `<template> + <script setup> + <style lang="scss">`，样式沿用 Tailwind 原子类和 CSS 变量。

### 3. 可复用组件清单
- `docs/.vitepress/config.mts`：统一处理导航、页脚、编辑链接和站点级 meta。
- `docs/.vitepress/theme/style.css`：全局导航与默认主题覆盖入口。
- `docs/.vitepress/theme/components/GitHubStar.vue`：顶部 GitHub 入口组件。
- `docs/index.vue`：官网首页主体结构和 JSON-LD 数据所在文件。
- `docs/tests/site-config.test.mjs`、`docs/tests/site-content.test.mjs`：站点配置与内容级回归测试。

### 4. 测试策略
- **测试框架**：Node 内置 `node:test`。
- **测试模式**：配置与内容静态断言 + VitePress 构建验证。
- **参考文件**：`docs/tests/site-config.test.mjs`、`docs/tests/site-content.test.mjs`、`docs/tests/seo-and-deploy.test.mjs`。
- **覆盖要求**：修正文档入口、统一仓库链接、补充 guide 页面存在性，并保证 `npm run build` 成功。

### 5. 依赖和集成点
- **外部依赖**：`vitepress`、`tailwindcss`、`vitepress-plugin-llms`、`vite-plugin-toml`。
- **内部依赖**：首页截图资源位于 `docs/public/screenshots/`，logo 位于 `docs/public/logo*.svg`。
- **集成方式**：GitHub Pages 读取 `docs/.vitepress/dist`，站点基础路径为 `/onetcli/`。
- **配置来源**：`docs/.vitepress/config.mts` 与 `.github/workflows/release-docs.yml`。

### 6. 技术选型理由
- **为什么用这个方案**：继续复用现有 VitePress 站点是修复成本最低、风险最小的方案。
- **优势**：改动集中、SEO 友好、无额外依赖、与现有 Pages 流程完全兼容。
- **劣势和风险**：如果外链或 `base` 处理不一致，仍会产生 GitHub Pages 项目页下的 404。

### 7. 关键风险点
- **链接风险**：所有站内链接必须与 `/onetcli/` 基础路径兼容，外链仓库 owner 必须统一为 `feigeCode`。
- **视觉风险**：首页需改好看，但不能偏离当前主题系统，也不能引入大规模重构。
- **验证风险**：本次需要补足 guide 页面断言，否则链接修复缺乏回归保障。
