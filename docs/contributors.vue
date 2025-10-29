<template>
    <div class="contributors-page">
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
    </div>
</template>

<script setup>
import { data } from "./data/contributors.data";
const contributors = data;
</script>

<style lang="scss" scoped>
@reference "./.vitepress/theme/style.css";
.contributors-list {
    @apply my-8 grid grid-cols-1 lg:grid-cols-4 gap-6;
}

.contributor-card {
    @apply px-4 py-3 gap-3  border border-[var(--border)] rounded-lg hover:shadow-lg transition-shadow items-center text-center justify-center no-underline flex flex-col;

    .contributor-avatar {
        @apply w-12 h-12 rounded-full;
    }
}
</style>
