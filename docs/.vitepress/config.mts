import { defineConfig } from "vitepress";
import type { UserConfig } from "vitepress";
import llmstxt from "vitepress-plugin-llms";
import tailwindcss from "@tailwindcss/vite";
import { lightTheme, darkTheme } from "./language";
import { ViteToml } from "vite-plugin-toml";

// https://vitepress.dev/reference/site-config
const config: UserConfig = {
  srcExclude: [
    "README.md",
    "contributors.md",
    "skills.md",
    "docs/**",
    "design/**",
    "superpowers/**",
  ],
  title: "OnetCli",
  base: "/onetcli/",
  description:
    "OnetCli 是一个统一管理数据库、SSH、终端与 AI 工作流的跨平台桌面客户端。",
  cleanUrls: true,
  head: [
    ["meta", { name: "theme-color", content: "#12071f" }],
    ["meta", { property: "og:type", content: "website" }],
    ["meta", { property: "og:title", content: "OnetCli" }],
    [
      "meta",
      {
        property: "og:description",
        content:
          "OnetCli 是一个统一管理数据库、SSH、终端与 AI 工作流的跨平台桌面客户端。",
      },
    ],
    ["meta", { name: "twitter:card", content: "summary_large_image" }],
    [
      "link",
      {
        rel: "icon",
        href: "/onetcli/logo.svg",
        media: "(prefers-color-scheme: light)",
      },
    ],
    [
      "link",
      {
        rel: "icon",
        href: "/onetcli/logo-dark.svg",
        media: "(prefers-color-scheme: dark)",
      },
    ],
  ],
  vite: {
    plugins: [llmstxt(), tailwindcss(), ViteToml()],
  },
  themeConfig: {
    logo: {
      light: "/logo.svg",
      dark: "/logo-dark.svg",
    },
    footer: {
      message: `OnetCli 是一个面向数据库、服务器和 AI 工作流的一体化桌面客户端。`,
      copyright: `
        <a href="https://github.com/feigeCode/onetcli">GitHub</a>
        |
        <a href="https://github.com/feigeCode/onetcli/releases">Releases</a>
        |
        <a href="/onetcli/changelog">更新日志</a>
        |
        <a href="/onetcli/download">下载</a>
        <br />
        界面图标资源来自 <a href="https://lucide.dev" target="_blank">Lucide</a>。
      `,
    },
    // https://vitepress.dev/reference/default-theme-config
    nav: [
      { text: "首页", link: "/" },
      { text: "功能", link: "/features" },
      { text: "下载", link: "/download" },
      { text: "更新日志", link: "/changelog" },
      { text: "文档", link: "/guide" },
      {
        component: "GitHubStar",
      },
    ],

    sidebar: false,

    socialLinks: null,
    editLink: {
      pattern: "https://github.com/feigeCode/onetcli/edit/dev/docs/:path",
    },
  },
  markdown: {
    math: true,
    defaultHighlightLang: "rs",
    theme: {
      light: lightTheme,
      dark: darkTheme,
    },
  },
};

export default defineConfig(config);
