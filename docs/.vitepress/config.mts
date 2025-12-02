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
const sidebar = generateSidebar([
  {
    scanStartPath: "/docs/",
    rootGroupText: "Introduction",
    collapsed: false,
    useTitleFromFrontmatter: true,
    useTitleFromFileHeading: true,
    sortMenusByFrontmatterOrder: true,
    includeRootIndexFile: false,
  },
]);

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
  themeConfig: {
    logo: {
      light: "/logo.svg",
      dark: "/logo-dark.svg",
    },
    footer: {
      message: `GPUI Component is an open source project under the Apache-2.0 License,
        developed by <a href='https://longbridge.com' target='_blank'>Longbridge</a>.`,
      copyright: `
        <a href="https://gpui.rs">GPUI</a>
        |
        <a href="/gpui-component/contributors">Contributors</a>
        |
        <a href="/gpui-component/llms-full.txt" target="_blank">llms-full.txt</a>
        |
        <a href="https://github.com/longbridge/gpui-component/issues" target="_blank">Report Bug</a>
        |
        <a href="https://github.com/longbridge/gpui-component/discussions" target="_blank">Discussion</a>
        <br />
        Icon resources are used <a href="https://lucide.dev" target="_blank">Lucide</a>,
        <a href="https://isocons.app" target="_blank">Isocons</a>.
      `,
    },
    // https://vitepress.dev/reference/default-theme-config
    nav: [
      { text: "Home", link: "/" },
      { text: "Getting Started", link: "/docs/getting-started" },
      { text: "Components", link: "/docs/components" },
      { text: "API Doc", link: "https://docs.rs/gpui-component" },
      {
        text: "Resources",
        items: [
          {
            text: "Contributors",
            link: "/contributors",
          },
          {
            text: "Releases",
            link: "https://github.com/longbridge/gpui-component/releases",
          },
          {
            text: "Issues",
            link: "https://github.com/longbridge/gpui-component/issues",
          },
          {
            text: "Discussion",
            link: "https://github.com/longbridge/gpui-component/discussions",
          },
        ],
      },
      {
        component: "GitHubStar",
      },
    ],

    sidebar: sidebar as any,

    socialLinks: null,
    editLink: {
      pattern:
        "https://github.com/longbridge/gpui-component/edit/main/docs/:path",
    },
    search: {
      provider: "local",
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
