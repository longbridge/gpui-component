import { defineConfig } from "vitepress";
import type { UserConfig } from "vitepress";
import { generateSidebar } from "vitepress-sidebar";
import llmstxt from "vitepress-plugin-llms";
import tailwindcss from "@tailwindcss/vite";
import { lightTheme, darkTheme } from "./language";
import { ViteToml } from "vite-plugin-toml";

/**
 * https://github.com/jooy2/vitepress-sidebar
 */
function createSidebar(scanStartPath: string, rootGroupText: string) {
  return generateSidebar([
    {
      scanStartPath,
      rootGroupText,
      collapsed: false,
      useTitleFromFrontmatter: true,
      useTitleFromFileHeading: true,
      sortMenusByFrontmatterOrder: true,
      includeRootIndexFile: false,
    },
  ]) as any;
}

const enSidebar = createSidebar("/docs/", "Introduction");
const zhSidebar = createSidebar("/zh-CN/docs/", "文档");

function createFooter(prefix = "", locale: "en" | "zh" = "en") {
  const contributorsText = locale === "zh" ? "贡献者" : "Contributors";
  const skillsText = "Skills";
  const reportBugText = locale === "zh" ? "报告问题" : "Report Bug";
  const discussionText = locale === "zh" ? "讨论" : "Discussion";
  const message =
    locale === "zh"
      ? `GPUI Component 是一个基于 Apache-2.0 许可证的开源项目，
        由 <a href='https://longbridge.com' target='_blank'>Longbridge</a> 开发。`
      : `GPUI Component is an open source project under the Apache-2.0 License,
        developed by <a href='https://longbridge.com' target='_blank'>Longbridge</a>.`;

  return {
    message,
    copyright: `
      <a href="https://gpui.rs">GPUI</a>
      |
      <a href="/gpui-component/gallery/" target="_blank">Gallery</a>
      |
      <a href="/gpui-component${prefix}/contributors">${contributorsText}</a>
      |
      <a href="/gpui-component${prefix}/skills" target="_blank">${skillsText}</a>
      |
      <a href="/gpui-component/llms-full.txt" target="_blank">llms-full.txt</a>
      |
      <a href="https://github.com/longbridge/gpui-component/issues" target="_blank">${reportBugText}</a>
      |
      <a href="https://github.com/longbridge/gpui-component/discussions" target="_blank">${discussionText}</a>
      <br />
      Icon resources are used <a href="https://lucide.dev" target="_blank">Lucide</a>,
      <a href="https://isocons.app" target="_blank">Isocons</a>.
    `,
  };
}

function createNav(prefix = "", locale: "en" | "zh" = "en") {
  const homeText = locale === "zh" ? "首页" : "Home";
  const gettingStartedText = locale === "zh" ? "开始使用" : "Getting Started";
  const componentsText = locale === "zh" ? "组件" : "Components";
  const resourcesText = locale === "zh" ? "资源" : "Resources";
  const contributorsText = locale === "zh" ? "贡献者" : "Contributors";
  const releasesText = locale === "zh" ? "版本发布" : "Releases";
  const issuesText = "Issues";
  const discussionText = locale === "zh" ? "讨论" : "Discussion";

  return [
    { text: homeText, link: `${prefix}/` || "/" },
    { text: gettingStartedText, link: `${prefix}/docs/getting-started` || "/docs/getting-started" },
    { text: componentsText, link: `${prefix}/docs/components` || "/docs/components" },
    { text: "Gallery", link: "/gallery/", target: "_blank" },
    { text: "API Doc", link: "https://docs.rs/gpui-component" },
    {
      text: resourcesText,
      items: [
        {
          text: contributorsText,
          link: `${prefix}/contributors` || "/contributors",
        },
        {
          text: releasesText,
          link: "https://github.com/longbridge/gpui-component/releases",
        },
        {
          text: issuesText,
          link: "https://github.com/longbridge/gpui-component/issues",
        },
        {
          text: discussionText,
          link: "https://github.com/longbridge/gpui-component/discussions",
        },
      ],
    },
    {
      component: "LanguageSwitcher",
    },
    {
      component: "GitHubStar",
    },
  ];
}

const sharedThemeConfig = {
  logo: {
    light: "/logo.svg",
    dark: "/logo-dark.svg",
  },
  socialLinks: null,
  search: {
    provider: "local",
  },
};

// https://vitepress.dev/reference/site-config
const config: UserConfig = {
  title: "GPUI Component",
  base: "/gpui-component/",
  description:
    "Rust GUI components for building fantastic cross-platform desktop application by using GPUI.",
  cleanUrls: true,
  head: [
    [
      "link",
      {
        rel: "icon",
        href: "/gpui-component/logo.svg",
        media: "(prefers-color-scheme: light)",
      },
    ],
    [
      "link",
      {
        rel: "icon",
        href: "/gpui-component/logo-dark.svg",
        media: "(prefers-color-scheme: dark)",
      },
    ],
  ],
  vite: {
    plugins: [llmstxt(), tailwindcss(), ViteToml()],
  },
  themeConfig: sharedThemeConfig,
  locales: {
    root: {
      label: "English",
      lang: "en-US",
      themeConfig: {
        ...sharedThemeConfig,
        langMenuLabel: "Languages",
        nav: createNav("", "en"),
        sidebar: enSidebar,
        footer: createFooter("", "en"),
        editLink: {
          pattern:
            "https://github.com/longbridge/gpui-component/edit/main/docs/:path",
        },
      },
    },
    "zh-CN": {
      label: "简体中文",
      lang: "zh-CN",
      link: "/zh-CN/",
      themeConfig: {
        ...sharedThemeConfig,
        nav: createNav("/zh-CN", "zh"),
        sidebar: zhSidebar,
        footer: createFooter("/zh-CN", "zh"),
        langMenuLabel: "语言",
        returnToTopLabel: "返回顶部",
        sidebarMenuLabel: "菜单",
        darkModeSwitchLabel: "外观",
        lightModeSwitchTitle: "切换到浅色模式",
        darkModeSwitchTitle: "切换到深色模式",
        editLink: {
          pattern:
            "https://github.com/longbridge/gpui-component/edit/main/docs/:path",
        },
      },
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

