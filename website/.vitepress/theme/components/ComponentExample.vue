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
import { useWindowZoom } from "../composables/zoom";
import WindowZoomButton from "./WindowZoomButton.vue";

const route = useRoute();
const { frontmatter } = useData();

const component = computed(() => {
    if (typeof frontmatter.value.example === "string") {
        return frontmatter.value.example;
    }

    const match = route.path.match(
        /\/(?:docs\/components|base\/primitives)\/([^/]+)$/,
    );
    return match?.[1] === "index" ? undefined : match?.[1];
});

const kind = computed(() =>
    frontmatter.value.exampleKind === "base" ||
    route.path.includes("/base/primitives/")
        ? "base"
        : "component",
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
    "title-bar": "Introduction",
    "virtual-list": "VirtualList",
};

const titleCase = (value: string) =>
    value
        .split(/[-_]/)
        .map((part) => part.charAt(0).toUpperCase() + part.slice(1))
        .join("");

const storyName = computed(() =>
    component.value
        ? (storyNames[component.value] ?? titleCase(component.value))
        : undefined,
);

const src = computed(() => {
    if (!component.value) return undefined;
    if (kind.value === "base") {
        return withBase(
            `/examples/base/?component=${encodeURIComponent(component.value)}`,
        );
    }

    return withBase(`/gallery/?story=${encodeURIComponent(storyName.value)}`);
});

// The frame is presented as a real desktop window, so it carries a window
// title the way a native app would.
const windowTitle = computed(() =>
    storyName.value
        ? `${storyName.value} — ${kind.value === "base" ? "gpui-base" : "gpui-component"}`
        : "",
);

const target = shallowRef<HTMLElement>();

const { zoomed, zoomLabel, setZoomed } = useWindowZoom("example");

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
watch(() => route.path, () => {
    // A new page means a new example, so it starts unzoomed.
    setZoomed(false);
    createTargetAfterDescription();
});
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
                <span>Example</span>
                <span class="component-example__live">Rust &amp; WASM</span>
            </div>
            <div class="mac-window" :class="{ 'mac-window--zoomed': zoomed }">
                <div class="mac-window__bar">
                    <span class="mac-window__lights">
                        <i aria-hidden="true" /><i aria-hidden="true" /><button
                            type="button"
                            class="mac-window__zoom"
                            :title="zoomLabel"
                            :aria-label="zoomLabel"
                            :aria-pressed="zoomed"
                            @click="setZoomed(!zoomed)"
                        />
                    </span>
                    <span class="mac-window__title">{{ windowTitle }}</span>
                    <WindowZoomButton
                        :zoomed="zoomed"
                        :label="zoomLabel"
                        @click="setZoomed(!zoomed)"
                    />
                </div>
                <iframe
                    :key="src"
                    :src="src"
                    :title="`${component} interactive example`"
                    allow="cross-origin-isolated"
                />
            </div>
        </section>
    </Teleport>
</template>
