<script setup lang="ts">
import {
    computed,
    nextTick,
    onBeforeUnmount,
    onMounted,
    shallowRef,
    watch,
} from "vue";
import { useData, useRoute, withBase } from "vitepress";

const route = useRoute();
const { frontmatter } = useData();

const component = computed(() => {
    const match = route.path.match(/\/(?:docs|base)\/components\/([^/]+)$/);
    return match?.[1] === "index" ? undefined : match?.[1];
});

const kind = computed(() =>
    route.path.includes("/base/components/") ? "base" : "component",
);

const storyNames: Record<string, string> = {
    "alert-dialog": "AlertDialog",
    "color-picker": "ColorPicker",
    "data-table": "DataTable",
    "date-picker": "DatePicker",
    "description-list": "DescriptionList",
    dropdown_button: "DropdownButton",
    "focus-trap": "Dialog",
    "group-box": "GroupBox",
    "hover-card": "HoverCard",
    "native-menu": "NativeMenu",
    notification: "Notification",
    "number-input": "NumberInput",
    "otp-input": "OtpInput",
    plot: "Chart",
    scrollable: "Scrollbar",
    "status-bar": "StatusBar",
    "text-view": "Editor",
    "title-bar": "Welcome",
    "virtual-list": "VirtualList",
};

const titleCase = (value: string) =>
    value
        .split(/[-_]/)
        .map((part) => part.charAt(0).toUpperCase() + part.slice(1))
        .join("");

const src = computed(() => {
    if (!component.value) return undefined;
    if (kind.value === "base") {
        return withBase(
            `/examples/base/?component=${encodeURIComponent(component.value)}`,
        );
    }

    const story = storyNames[component.value] ?? titleCase(component.value);
    return withBase(`/gallery/?story=${encodeURIComponent(story)}`);
});

const target = shallowRef<HTMLElement>();

const createTargetAfterDescription = async () => {
    await nextTick();
    target.value?.remove();
    target.value = undefined;

    if (!src.value || frontmatter.value.example === false) return;
    const title = document.querySelector<HTMLElement>(".vp-doc h1");
    const description = title?.nextElementSibling;
    if (!title) return;

    const mountPoint = document.createElement("div");
    mountPoint.className = "component-example-mount";
    if (description?.tagName === "P") {
        description.after(mountPoint);
    } else {
        title.after(mountPoint);
    }
    target.value = mountPoint;
};

onMounted(createTargetAfterDescription);
watch(() => route.path, createTargetAfterDescription);
onBeforeUnmount(() => target.value?.remove());
</script>

<template>
    <Teleport
        v-if="target && src && frontmatter.example !== false"
        :to="target"
    >
        <section
            class="component-example"
            :class="`component-example--${kind}`"
        >
            <div class="component-example__label">
                <span>Example</span><span>Rust & WASM</span>
            </div>
            <iframe
                :key="src"
                :src="src"
                :title="`${component} interactive example`"
                allow="cross-origin-isolated"
            />
        </section>
    </Teleport>
</template>
