# OnetCli 官网设计规格

**生成时间**：2026-03-28 14:32:17 +0800

## 目标

将当前 `docs/` 下的 VitePress 站点从 gpui-component 文档站改造成 OnetCli 官网，满足以下目标：

- 首要目标是展示 OnetCli 的产品价值。
- 次要目标是把用户引导到 GitHub Releases 下载。
- 首批页面控制在最小集合：首页、功能页、下载页、更新日志页。
- 站点保持无登录、无存储、无后端状态。
- 先托管到 GitHub Pages，后续可迁移到自定义域名或其他平台。

## 现有基础

- 站点框架：VitePress。
- 页面模式：Markdown 路由入口 + Vue 自定义页面。
- 样式系统：Tailwind 4 + 自定义 CSS 变量。
- 文档构建：Bun。
- 部署链路：GitHub Actions Pages 工作流已存在，但需要修正触发策略。

## 页面范围

### 首页

首页采用“产品旗舰型”结构：

1. Hero 首屏
2. 真实产品截图
3. 四大核心能力
4. 为什么选择 OnetCli
5. 下载引导
6. FAQ
7. 最近更新

首页的主 CTA 为“下载 OnetCli”，次 CTA 为“查看功能”。

### 功能页

功能页按用户心智而不是技术模块组织：

- 多数据库管理
- SSH / SFTP
- 本地终端
- AI 助手

每个模块都需要有：

- 简洁定义句
- 真实使用价值
- 截图或界面说明
- 适合搜索引擎和答案引擎抽取的短段落

### 下载页

下载页只承担一件事：把平台用户导向 GitHub Releases。

页面结构：

- 版本说明
- 平台卡片（macOS / Windows / Linux）
- 下载按钮
- 校验信息与安装提示入口

### 更新日志页

更新日志页采用静态内容页方式，先做一个聚合页，收录近期版本摘要。后续如果持续更新，可以扩展为单版本独立页面。

## 导航与页脚

顶部导航建议为：

- 首页
- 功能
- 下载
- 更新日志
- 文档
- GitHub

页脚保留：

- GitHub 仓库
- Releases
- License
- 可选的 README / 文档入口

删除 gpui-component、Contributors、Skills、Discussion 等不再适用的导航项和页脚项。

## SEO 与 GEO 设计

### 语义结构

关键页面优先采用语义化 HTML：

- `header`
- `nav`
- `main`
- `section`
- `article`
- `footer`

### 标题层级

- 每页只保留一个 `h1`
- 一级模块用 `h2`
- 子模块用 `h3`

### 可抽取内容块

每个核心模块前两段文案必须写成可以直接回答问题的定义式表述，例如：

- OnetCli 是什么
- OnetCli 支持哪些数据库
- OnetCli 是否支持 SSH / SFTP
- OnetCli 是否支持 AI 生成 SQL

### FAQ

首页加入 FAQ 区块，问题采用真实搜索表达，答案保持简洁明确。

### 结构化数据

- 首页加入 `SoftwareApplication`
- FAQ 区块加入 `FAQPage`

### 元信息

每页单独配置：

- `title`
- `description`
- Open Graph 信息
- 站点图标

## GitHub Pages 部署

采用 GitHub 官方 Pages Actions 链路：

- `actions/configure-pages`
- `actions/upload-pages-artifact`
- `actions/deploy-pages`

站点构建产物保持为 `docs/.vitepress/dist`。

### base 策略

当前先按 GitHub Pages 项目页处理，`base` 需要与仓库路径一致。后续如果切到自定义域名，可再调整为根路径。

## 不在本次范围内

- 登录
- 用户系统
- 评论
- 后端 API
- 云端同步
- Cloudflare Pages / Workers
- 自研 CMS

## 风险与对策

- **品牌残留风险**：全面替换旧导航、页脚、首页文案与元信息，并通过构建后检查确认无 gpui-component 残留。
- **路径风险**：在 VitePress 配置中集中处理 `base`，并以本地构建结果验证资源引用。
- **内容质量风险**：所有产品能力描述以仓库现有 README 和截图为事实来源，避免夸大。

## 验收标准

- 站点首批页面完整可访问。
- 首页结构符合产品旗舰型方案。
- 下载按钮统一跳转 GitHub Releases。
- 本地 `bun run build` 通过。
- GitHub Pages 工作流可稳定部署 `docs/.vitepress/dist`。
- 页面具备基本 SEO/GEO 要素：语义化结构、FAQ、结构化数据、清晰元信息。
