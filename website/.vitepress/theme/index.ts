// https://vitepress.dev/guide/custom-theme
import { h } from "vue";
import type { Theme } from "vitepress";
import DefaultTheme from "vitepress/theme";
import "@fontsource-variable/jetbrains-mono";
import "./style.css";
import GitHubStar from "./components/GitHubStar.vue";
import LanguageSwitcher from "./components/LanguageSwitcher.vue";
import ComponentExample from "./components/ComponentExample.vue";
import config from "../../../crates/ui/Cargo.toml";

/** @type {import('vitepress').Theme} */
export default {
  extends: DefaultTheme,
  Layout: () => {
    return h(DefaultTheme.Layout, null, {
      "doc-before": () => h(ComponentExample),
    });
  },
  enhanceApp({ app, router, siteData }) {
    // ...
    app.component("GitHubStar", GitHubStar);
    app.component("LanguageSwitcher", LanguageSwitcher);
    app.component("ComponentExample", ComponentExample);

    app.config.globalProperties.GPUI_VERSION = "0.2.2";
    app.config.globalProperties.VERSION = config.package.version;
  },
} satisfies Theme;
