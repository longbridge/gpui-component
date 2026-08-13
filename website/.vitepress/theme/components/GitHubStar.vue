<template>
    <a
        href="https://github.com/longbridge/gpui-component"
        target="_blank"
        rel="noopener noreferrer"
        class="github-star-button"
        :title="`${stargazers_count} stars on GitHub`"
    >
        <Github />
        <span>{{ starLabel }}</span>
    </a>
</template>

<script setup>
import { data } from "../../../data/repo.data";
import { Github } from "lucide-vue-next";
let stargazers_count = data.stargazers_count;

const starLabel =
    stargazers_count / 1000 >= 1
        ? (stargazers_count / 1000.0).toFixed(1) + "k"
        : stargazers_count.toString();
</script>

<style lang="scss" scoped>
@reference "../style.css";

/* The button only ever sits in the navbar, so it stays an inline control at
   every width; the navbar itself decides when the count is dropped. */
.github-star-button {
    @apply inline-flex w-auto py-0 px-2;
    @apply items-center text-sm gap-2 text-[var(--vp-c-text-1)] hover:text-[var(--vp-c-brand-1)] no-underline;
    border-radius: var(--radius);
    font-size: 0.76rem;
    font-variant-numeric: tabular-nums;

    svg {
        @apply w-4 h-4 shrink-0;
        stroke-width: 1.8;
    }
}
</style>
