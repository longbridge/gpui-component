import { defineConfig } from "vitepress";
import type { UserConfig } from "vitepress";
import { generateSidebar, withSidebar } from "vitepress-sidebar";

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
  // {
  //   scanStartPath: "/docs/components/",
  //   rootGroupText: "Components",
  //   collapsed: false,
  //   useTitleFromFileHeading: true,
  //   sortMenusByFrontmatterOrder: true,
  //   includeRootIndexFile: false,
  // },
]);

// https://vitepress.dev/reference/site-config
const config: UserConfig = {
  title: "GPUI Component",
  description:
    "Rust GUI components for building fantastic cross-platform desktop application by using GPUI.",
  cleanUrls: true,
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
