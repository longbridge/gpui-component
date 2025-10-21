import { defineConfig } from "vitepress";
import type { UserConfig } from "vitepress";
import { generateSidebar } from "vitepress-sidebar";
import llmstxt from "vitepress-plugin-llms";
import tailwindcss from "@tailwindcss/vite";

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
    ],

    sidebar: sidebar as any,

    socialLinks: [
      { icon: "github", link: "https://github.com/vuejs/vitepress" },
    ],
  },
};

export default defineConfig(config);
