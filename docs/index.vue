<template>
    <div class="banner">
        <h1>GPUI Component</h1>
        <div class="banner-description">
            Rust GUI components for building fantastic cross-platform desktop
            application by using
            <a href="https://gpui.rs" target="_blank">GPUI</a>.
        </div>
        <div class="actions">
            <a href="docs/getting-started" class="btn-primary">Get Started</a>
            <a href="docs/components"><Blocks /> Components</a>
        </div>
        <div class="version">
            Version:
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
                <div>60+ Components</div>
            </h3>
            <div>
                Comprehensive library of cross-platform desktop UI components
                for building feature-rich applications.
            </div>
        </div>
        <div class="feature-card">
            <h3>
                <div class="icon bg-blue-500 dark:bg-blue-700">
                    <Zap />
                </div>
                <div>High Performance</div>
            </h3>
            <div>
                Virtualized Table and List components for smooth rendering of
                large datasets with minimal memory footprint.
            </div>
        </div>

        <div class="feature-card">
            <h3>
                <div class="icon bg-red-500 dark:bg-red-700">
                    <Palette />
                </div>
                <div>Themeable</div>
            </h3>
            <div>
                Built-in theme system with with 20+ themes, and dark mode out of
                the box.
            </div>
        </div>

        <div class="feature-card">
            <h3>
                <div class="icon bg-yellow-500 dark:bg-yellow-700">
                    <Layout />
                </div>
                <div>Flexible Layouts</div>
            </h3>
            <div>
                Dock layout for panel arrangements, resizable panels, and
                freeform layouts for any application structure.
            </div>
        </div>

        <div class="feature-card">
            <h3>
                <div class="icon bg-pink-500 dark:bg-pink-700">
                    <BarChart3 />
                </div>
                <div>Data Visualization</div>
            </h3>
            <div>
                Built-in chart components for visualizing data with Line, Bar,
                Area, and Pie charts.
            </div>
        </div>

        <div class="feature-card">
            <h3>
                <div class="icon bg-cyan-500 dark:bg-cyan-700">
                    <SquareCode />
                </div>
                <div>Code Editor</div>
            </h3>
            <div>
                High-performance code editor with LSP support, syntax
                highlighting, powered by Tree-sitter and Rope.
            </div>
        </div>
    </div>
</template>

<script setup>
import {
    Blocks,
    Zap,
    Palette,
    Layout,
    BarChart3,
    SquareCode,
    Github,
} from "lucide-vue-next";
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
