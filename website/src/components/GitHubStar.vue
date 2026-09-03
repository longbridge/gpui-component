<template>
    <a
        href="https://github.com/longbridge/gpui-kit"
        target="_blank"
        rel="noopener noreferrer"
        class="github-star-button"
        :title="`${stargazersCount} stars on GitHub`"
    >
        <Github :size="16" :stroke-width="1.8" />
        <span>{{ starLabel }}</span>
    </a>
</template>

<script setup lang="ts">
import { computed } from "vue";
import { Github } from "lucide-vue-next";

const props = defineProps<{
    stargazersCount: number;
}>();

const starLabel = computed(() =>
    props.stargazersCount / 1000 >= 1
        ? (props.stargazersCount / 1000.0).toFixed(1) + "k"
        : props.stargazersCount.toString()
);
</script>

<style scoped>
.github-star-button {
    display: inline-flex;
    width: auto;
    padding: 0 0.5rem;
    align-items: center;
    font-size: 0.76rem;
    gap: 0.5rem;
    color: var(--foreground);
    text-decoration: none;
    border-radius: var(--radius);
    font-variant-numeric: tabular-nums;
    transition: color 140ms ease;
}

.github-star-button:hover {
    color: var(--brand);
}
</style>
