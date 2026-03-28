# OnetCli 官网实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 将现有 VitePress 文档站改造成 OnetCli 官网，并打通 GitHub Pages 部署。

**Architecture:** 复用 `docs/` 下现有 VitePress 结构与 Bun 构建流程，重写页面内容和站点配置，不新建第二套前端。部署层继续使用 GitHub Pages 官方 Actions 链路，构建产物保持 `docs/.vitepress/dist`。

**Tech Stack:** VitePress、Vue SFC、Tailwind CSS、Bun、GitHub Actions Pages

---

### Task 1: 规格与上下文文件

**Files:**
- Create: `.claude/context-summary-onetcli-website.md`
- Create: `docs/superpowers/specs/2026-03-28-onetcli-website-design.md`
- Modify: `.claude/operations-log.md`

- [ ] **Step 1: 写入上下文摘要**

记录至少 3 个相似实现、可复用组件、GitHub Pages 风险和测试策略。

- [ ] **Step 2: 写入官网规格**

覆盖页面集合、首页结构、SEO/GEO、GitHub Pages 约束和不在范围内的事项。

- [ ] **Step 3: 在操作日志中追加编码前检查**

说明将复用的组件、命名约定、代码风格与不重复造轮子的依据。

### Task 2: 站点骨架改造

**Files:**
- Modify: `docs/.vitepress/config.mts`
- Modify: `docs/index.md`
- Modify: `docs/index.vue`
- Create: `docs/features.md`
- Create: `docs/download.md`
- Create: `docs/changelog.md`

- [ ] **Step 1: 先创建最小页面骨架**

新增功能页、下载页、更新日志页，先保证路由存在。

- [ ] **Step 2: 更新 VitePress 全局配置**

替换站点标题、描述、导航、页脚、仓库链接与 `base`。

- [ ] **Step 3: 重写首页**

把首页替换为 OnetCli 的产品旗舰型结构，文案围绕展示与下载。

- [ ] **Step 4: 运行 docs 构建验证**

Run: `cd docs && bun run build`
Expected: 构建成功，输出位于 `.vitepress/dist`

### Task 3: SEO / GEO 增强

**Files:**
- Modify: `docs/index.vue`
- Modify: `docs/.vitepress/config.mts`
- Modify: `docs/.vitepress/theme/style.css`

- [ ] **Step 1: 先为首页添加 FAQ 与结构化数据需求点**

明确 FAQ 问题列表和 `SoftwareApplication` / `FAQPage` JSON-LD 内容。

- [ ] **Step 2: 实现语义化首页结构与 FAQ**

确保首页具备可抽取内容块和清晰标题层级。

- [ ] **Step 3: 补充元信息与分享信息**

在站点配置中加入适合 GitHub Pages 的基础 `head` 信息。

- [ ] **Step 4: 再次运行 docs 构建验证**

Run: `cd docs && bun run build`
Expected: 构建成功，无运行时报错

### Task 4: GitHub Pages 工作流与收尾

**Files:**
- Modify: `.github/workflows/release-docs.yml`
- Modify: `.github/workflows/test-docs.yml`
- Modify: `.gitignore`
- Modify: `.claude/operations-log.md`
- Modify: `.claude/verification-report.md`

- [ ] **Step 1: 修正正式部署工作流**

将发布触发逻辑调整为适合 OnetCli 官网的 GitHub Pages 发布方式。

- [ ] **Step 2: 保留并核对 PR 构建工作流**

确保 `docs/**` 改动仍会触发构建校验。

- [ ] **Step 3: 忽略本地可视化目录**

在 `.gitignore` 中加入 `.superpowers/`。

- [ ] **Step 4: 本地完整验证**

Run: `cd docs && bun install && bun run build`
Expected: 安装和构建都成功

- [ ] **Step 5: 记录日志与审查结果**

在 `.claude/operations-log.md` 和 `.claude/verification-report.md` 中补齐实施结果、验证命令、风险与评分。
