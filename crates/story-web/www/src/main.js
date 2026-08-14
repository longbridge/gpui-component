const embedded = window.parent !== window;

// `gpui_web` reads keyboard and IME input through a 1x1 transparent `<input>`
// it appends to the body, and focuses that element when the window opens and
// again on every `pointerdown`. On a phone, focusing a text field raises the
// on-screen keyboard over the page — so an embedded gallery pops up the iOS
// keyboard while the reader is only scrolling past it, or taps a button.
//
// Touch-only devices therefore get that element marked `readonly` with
// `inputmode="none"`: iOS leaves the keyboard closed for a read-only field,
// and the element stays focusable, so gpui still tracks window activation and
// key events. Typing into a canvas through an off-screen input is not usable
// on a phone regardless. Devices with a real pointer are left alone, so
// desktop keyboards and IME composition behave exactly as before.
const touchOnly =
  window.matchMedia?.('(hover: none) and (pointer: coarse)').matches ?? false;

function keepKeyboardClosed(node) {
  if (node.tagName !== 'INPUT') return;

  if (touchOnly) {
    node.readOnly = true;
    node.setAttribute('inputmode', 'none');
  }

  // The focus on window creation lands before any interaction. Embedded, it
  // also takes focus away from the page hosting this gallery, which moves the
  // reader's caret and Tab order into the iframe. Hand it back; the next
  // `pointerdown` inside the canvas focuses it again.
  if ((touchOnly || embedded) && document.activeElement === node) {
    node.blur();
  }
}

// Watch from before the module boots, so the element is handled as soon as
// gpui appends it rather than after the keyboard has had a chance to appear.
function watchPlatformInput() {
  document.querySelectorAll('body > input').forEach(keepKeyboardClosed);
  new MutationObserver((records) => {
    for (const record of records) {
      record.addedNodes.forEach(keepKeyboardClosed);
    }
  }).observe(document.body, { childList: true });
}

// The gallery is embedded same-origin in the documentation site, so it can read
// the host page's appearance directly. That keeps the very first frame correct;
// asking the host to post it to us would paint a light frame first.
function hostPrefersDark() {
  if (!embedded) return undefined;
  try {
    return window.parent.document.documentElement.classList.contains('dark');
  } catch {
    // Cross-origin embedding: fall back to the viewer's own preference.
    return window.matchMedia('(prefers-color-scheme: dark)').matches;
  }
}

// Follow the host page when the reader toggles its theme.
function watchHostTheme(wasm) {
  if (!embedded) return;
  let root;
  try {
    root = window.parent.document.documentElement;
  } catch {
    return;
  }

  let current = root.classList.contains('dark');
  new MutationObserver(() => {
    const next = root.classList.contains('dark');
    if (next !== current) {
      current = next;
      document.documentElement.classList.toggle('dark', next);
      wasm.set_theme(next);
    }
  }).observe(root, { attributes: true, attributeFilter: ['class'] });
}

async function init() {
  const loadingEl = document.getElementById('loading');

  watchPlatformInput();

  try {
    // Import the WASM module
    const wasm = await import('./wasm/gpui_component_story_web.js');
    await wasm.default();

    // A documentation page can deep-link to the matching Rust story while the
    // standalone gallery keeps its normal overview.
    const story = new URLSearchParams(window.location.search).get('story');
    await wasm.run(story || undefined, hostPrefersDark());
    watchHostTheme(wasm);

    // Hide loading indicator
    loadingEl?.remove();
  } catch (error) {
    console.error('Failed to initialize:', error);

    // Show error message
    if (loadingEl) {
      loadingEl.innerHTML = `
        <div class="error">
          <h2>Failed to load the application</h2>
          <p>${error.message || error}</p>
          <p style="margin-top: 10px; font-size: 14px;">
            Please check the console for more details.
          </p>
        </div>
      `;
    }
  }
}

init();
