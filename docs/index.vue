<template>
    <div class="banner">
        <h1>GPUI Component</h1>
        <div class="banner-description">
            {{ bannerPrefix }}
            <a href="https://gpui.rs" target="_blank">GPUI</a><span v-if="!isZh">.</span>
            {{ bannerSuffix }}
        </div>
        <div class="actions">
            <a :href="gettingStartedHref" class="btn-primary">{{ getStartedText }}</a>
            <a :href="componentsHref"><Blocks /> {{ componentsText }}</a>
        </div>
        <div class="version">
            {{ versionLabel }}
            <a href="https://crates.io/crates/gpui-component" target="_blank">{{
                VERSION
            }}</a>
        </div>
    </div>
    <div class="features">
        <div class="feature-card">
            <h3>
                <div class="icon bg-green-500 dark:bg-green-700">
                    <Blocks />
                </div>
                <div>{{ features.componentCount.title }}</div>
            </h3>
            <div>
                {{ features.componentCount.description }}
            </div>
        </div>
        <div class="feature-card">
            <h3>
                <div class="icon bg-blue-500 dark:bg-blue-700">
                    <Zap />
                </div>
                <div>{{ features.performance.title }}</div>
            </h3>
            <div>
                {{ features.performance.description }}
            </div>
        </div>

        <div class="feature-card">
            <h3>
                <div class="icon bg-red-500 dark:bg-red-700">
                    <Palette />
                </div>
                <div>{{ features.theme.title }}</div>
            </h3>
            <div>
                {{ features.theme.description }}
            </div>
        </div>

        <div class="feature-card">
            <h3>
                <div class="icon bg-yellow-500 dark:bg-yellow-700">
                    <Layout />
                </div>
                <div>{{ features.layout.title }}</div>
            </h3>
            <div>
                {{ features.layout.description }}
            </div>
        </div>

        <div class="feature-card">
            <h3>
                <div class="icon bg-pink-500 dark:bg-pink-700">
                    <BarChart3 />
                </div>
                <div>{{ features.chart.title }}</div>
            </h3>
            <div>
                {{ features.chart.description }}
            </div>
        </div>

        <div class="feature-card">
            <h3>
                <div class="icon bg-cyan-500 dark:bg-cyan-700">
                    <SquareCode />
                </div>
                <div>{{ features.editor.title }}</div>
            </h3>
            <div>
                {{ features.editor.description }}
            </div>
        </div>
    </div>
</template>

<script setup>
import { computed } from "vue";
import { useData, withBase } from "vitepress";
import {
    Blocks,
    Zap,
    Palette,
    Layout,
    BarChart3,
    SquareCode,
} from "lucide-vue-next";

const { localeIndex } = useData();
const isZh = computed(() => localeIndex.value === "zh-CN");
const localePrefix = computed(() => (isZh.value ? "/zh-CN" : ""));
const gettingStartedHref = computed(
    () => withBase(`${localePrefix.value}/docs/getting-started`),
);
const componentsHref = computed(() =>
    withBase(`${localePrefix.value}/docs/components`),
);
const bannerPrefix = computed(() =>
    isZh.value
        ? "基于 Rust + "
        : "Rust GUI components for building fantastic cross-platform desktop application by using ",
);
const bannerSuffix = computed(() =>
    isZh.value ? " 构建卓越的桌面应用程序" : "",
);
const getStartedText = computed(() => (isZh.value ? "开始使用" : "Get Started"));
const componentsText = computed(() => (isZh.value ? "组件" : "Components"));
const versionLabel = computed(() => (isZh.value ? "版本：" : "Version:"));
const features = computed(() =>
    isZh.value
        ? {
              componentCount: {
                  title: "60+ 组件",
                  description: "覆盖丰富桌面场景的跨平台组件库，可直接用于构建复杂应用。",
              },
              performance: {
                  title: "高性能",
                  description: "内置虚拟列表与虚拟表格，面对大数据量渲染依然保持流畅。",
              },
              theme: {
                  title: "可主题化",
                  description: "内建主题系统与 20+ 主题，并原生支持明暗模式切换。",
              },
              layout: {
                  title: "灵活布局",
                  description: "支持 Dock、可调整面板和自由布局，适合复杂桌面应用结构。",
              },
              chart: {
                  title: "数据可视化",
                  description: "内置折线、柱状、面积、饼图等图表组件，便于快速展示数据。",
              },
              editor: {
                  title: "代码编辑器",
                  description: "高性能编辑器内置 LSP 与语法高亮，底层基于 Tree-sitter 和 Rope。",
              },
          }
        : {
              componentCount: {
                  title: "60+ Components",
                  description:
                      "Comprehensive library of cross-platform desktop UI components for building feature-rich applications.",
              },
              performance: {
                  title: "High Performance",
                  description:
                      "Virtualized Table and List components for smooth rendering of large datasets with minimal memory footprint.",
              },
              theme: {
                  title: "Themeable",
                  description:
                      "Built-in theme system with with 20+ themes, and dark mode out of the box.",
              },
              layout: {
                  title: "Flexible Layouts",
                  description:
                      "Dock layout for panel arrangements, resizable panels, and freeform layouts for any application structure.",
              },
              chart: {
                  title: "Data Visualization",
                  description:
                      "Built-in chart components for visualizing data with Line, Bar, Area, and Pie charts.",
              },
              editor: {
                  title: "Code Editor",
                  description:
                      "High-performance code editor with LSP support, syntax highlighting, powered by Tree-sitter and Rope.",
              },
          },
);
</script>

<style lang="scss">
@reference "./.vitepress/theme/style.css";

.banner {
    @apply flex flex-col gap-2 lg:gap-4 -mt-20  py-12 xl:py-30 text-center border border-b-0 border-(--border);

    background: url("/home.svg") no-repeat;
    background-position: bottom -90px right -90px;

    h1 {
        @apply mt-20 text-3xl xl:text-5xl font-bold mb-2 text-(--primary);
    }
    .banner-description {
        @apply text-lg xl:text-2xl text-(--muted-foreground);
    }
    .actions {
        @apply gap-4 flex justify-center text-sm;
        a {
            @apply flex items-center h-8 rounded-md gap-1.5 px-3 has-[>svg]:px-2.5 no-underline
            bg-(--secondary) hover:bg-(--secondary)/70 text-(--secondary-foreground);

            &.btn-primary {
                @apply bg-(--primary) hover:bg-(--primary)/90 text-(--primary-foreground);
            }

            .lucide {
                @apply w-4 h-4;
            }
        }
    }
    .version {
        @apply text-sm text-(--muted-foreground) pb-10;
        a {
            @apply text-(--muted-foreground) no-underline hover:underline;
        }
    }
}

.features {
    @apply grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 mb-12 border-b border-r  border-(--border);
}

.feature-card {
    @apply flex flex-col text-sm gap-2 py-3.5 px-5 border border-b-0 border-(--border);
    @apply border-r-0 last:border-r-0 md:last:border-b-0 lg:last:border-b-0;

    h3 {
        @apply m-0 p-0 text-lg text-(--primary) flex gap-3 items-center;

        .icon {
            @apply flex h-9 w-9 items-center justify-center rounded-md text-white;

            .lucide {
                @apply w-5 h-5;
            }
        }
    }
}
</style>
