import { computed, onBeforeUnmount, onMounted, shallowRef } from "vue";

/**
 * Zoom state for a `.mac-window` frame that shows the library running.
 *
 * Zooming lifts the frame over the viewport rather than entering browser
 * fullscreen: the page keeps its own chrome one Escape away, and the iframe is
 * never re-created, so the wasm instance inside it keeps running.
 */
export function useWindowZoom(subject = "window") {
    const zoomed = shallowRef(false);

    const zoomLabel = computed(() =>
        zoomed.value
            ? `Restore ${subject} (Esc)`
            : `Zoom ${subject} to full page`,
    );

    const setZoomed = (value: boolean) => {
        zoomed.value = value;
        document.documentElement.classList.toggle("has-zoomed-window", value);
    };

    const onKeydown = (event: KeyboardEvent) => {
        if (event.key === "Escape" && zoomed.value) setZoomed(false);
    };

    onMounted(() => document.addEventListener("keydown", onKeydown));
    onBeforeUnmount(() => {
        document.removeEventListener("keydown", onKeydown);
        setZoomed(false);
    });

    return { zoomed, zoomLabel, setZoomed };
}
