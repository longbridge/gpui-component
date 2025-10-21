import { defineConfig } from "vitepress";
import type { UserConfig } from "vitepress";
import { generateSidebar } from "vitepress-sidebar";
import llmstxt from "vitepress-plugin-llms";
import tailwindcss from "@tailwindcss/vite";
import { lightTheme, darkTheme } from "./language";

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
  vite: {
    plugins: [llmstxt(), tailwindcss()],
  },
  themeConfig: {
    // https://vitepress.dev/reference/default-theme-config
    nav: [
      { text: "Home", link: "/" },
      { text: "Getting Started", link: "/docs/getting-started" },
      { text: "Components", link: "/docs/components" },
      { text: "Docs", link: "https://docs.rs/gpui-component" },
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
    ],

    sidebar: sidebar as any,

    socialLinks: [
      { icon: "github", link: "https://github.com/longbridge/gpui-component" },
    ],
    editLink: {
      pattern:
        "https://github.com/longbridge/gpui-component/edit/main/docs/:path",
    },
    search: {
      provider: "local",
    },
  },
  markdown: {
    defaultHighlightLang: "rs",
    theme: {
      light: lightTheme,
      dark: darkTheme,
    },
  },
};

export default defineConfig(config);
