<template>
    <div class="contributors-page">
        <h1>{{ title }}</h1>
        <p>{{ description }}</p>
        <div class="contributors-list">
            <a
                :href="contributor.html_url"
                v-for="contributor in contributors"
                :key="contributor.id"
                class="contributor-card"
                rel="noopener noreferrer"
            >
                <img
                    :src="contributor.avatar_url"
                    :alt="contributor.login"
                    class="contributor-avatar"
                />
                <div class="contributor-info">
                    {{ contributor.login }}
                </div>
            </a>
        </div>
        <div class="mt-6 text-(--muted-foreground)">
            {{ moreText }}
            <a
                href="https://github.com/longbridge/gpui-component/graphs/contributors"
                target="_blank"
            >
                {{ contributorsLinkText }}</a
            >
            {{ suffixText }}
        </div>
    </div>
</template>

<script setup>
import { computed } from "vue";
import { useData } from "vitepress";
import { data } from "./data/contributors.data";

const { localeIndex } = useData();
const isZh = computed(() => localeIndex.value === "zh-CN");
const contributors = data;
const title = computed(() => (isZh.value ? "贡献者" : "Contributors"));
const description = computed(() =>
    isZh.value
        ? "感谢所有为这个项目做出贡献的开发者。"
        : "Thanks to all the people who have contributed to this project!",
);
const moreText = computed(() =>
    isZh.value ? "这里没有展示全部贡献者，完整列表请查看 GitHub 上的 " : "More contributors not shown here. See the full ",
);
const contributorsLinkText = computed(() =>
    isZh.value ? "贡献者列表" : "Contributors",
);
const suffixText = computed(() => (isZh.value ? "。" : " on GitHub."));
</script>

<style lang="scss" scoped>
@reference "./.vitepress/theme/style.css";

.contributors-page {
    @apply py-10 pt-30;
    background: url("/contributors.svg") no-repeat;
    background-position: top 20px right 20px;
}

.contributors-list {
    @apply mt-8 grid grid-cols-2 md:grid-cols-3 lg:grid-cols-4 border-r border-b border-(--border);
}

.contributor-card {
    @apply px-4 py-3 gap-3 border border-(--border) hover:bg-(--secondary) transition-shadow
    items-center text-center text-(--foreground) hover:text-(--foreground) justify-center no-underline flex flex-col
    border border-b-0 border-(--border);

    @apply border-r-0 last:border-r-0 md:last:border-b-0 lg:last:border-b-0;

    .contributor-avatar {
        @apply w-12 h-12 rounded-full;
    }
}
</style>
