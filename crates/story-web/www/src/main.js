async function init() {
  const loadingEl = document.getElementById('loading');

  try {
    // Import the WASM module
    const wasm = await import('./wasm/gpui_component_story_web.js');
    await wasm.default();

    // A documentation page can deep-link to the matching Rust story while the
    // standalone gallery keeps its normal overview.
    const story = new URLSearchParams(window.location.search).get('story');
    await wasm.run(story || undefined);

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
