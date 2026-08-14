const embedded = window.parent !== window;

// `gpui_web` reads keyboard and IME input through a 1x1 transparent `<input>`
// it appends to the body, and focuses that element when the window opens and
// again on every `pointerdown`. On a phone, focusing a text field raises the
// on-screen keyboard over the page — so an example embedded in the docs pops up
// the iOS keyboard as soon as the page loads, or when a button is tapped.
//
// Touch-only devices therefore get that element marked `readonly` with
// `inputmode="none"`: iOS leaves the keyboard closed for a read-only field, and
// the element stays focusable, so gpui still tracks window activation and key
// events. Typing into a canvas through an off-screen input is not usable on a
// phone regardless. Devices with a real pointer are left alone, so desktop
// keyboards and IME composition behave exactly as before.
const touchOnly =
  window.matchMedia?.('(hover: none) and (pointer: coarse)').matches ?? false;

function keepKeyboardClosed(node) {
  if (node.tagName !== 'INPUT') return;

  if (touchOnly) {
    node.readOnly = true;
    node.setAttribute('inputmode', 'none');
  }

  // The focus on window creation lands before any interaction. Embedded, it
  // also takes focus away from the page hosting this example, which moves the
  // reader's caret and Tab order into the iframe. Hand it back; the next
  // `pointerdown` inside the canvas focuses it again.
  if ((touchOnly || embedded) && document.activeElement === node) {
    node.blur();
  }
}

// Watch from before the module boots, so the element is handled as soon as gpui
// appends it rather than after the keyboard has had a chance to appear.
function watchPlatformInput() {
  document.querySelectorAll('body > input').forEach(keepKeyboardClosed);
  new MutationObserver((records) => {
    for (const record of records) {
      record.addedNodes.forEach(keepKeyboardClosed);
    }
  }).observe(document.body, { childList: true });
}

async function init() {
  const loading = document.getElementById('loading');

  watchPlatformInput();

  try {
    const wasm = await import('./wasm/gpui_base_examples_wasm.js');
    await wasm.default();
    const component = new URLSearchParams(window.location.search).get('component');
    await wasm.run(component || undefined);
    loading?.remove();
  } catch (error) {
    console.error('Failed to initialize gpui-base example:', error);
    if (loading) loading.textContent = `Failed to load example: ${error?.message || error}`;
  }
}
init();
