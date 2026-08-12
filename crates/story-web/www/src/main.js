const embedded = window.parent !== window;

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
