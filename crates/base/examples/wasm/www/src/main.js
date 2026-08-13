async function init() {
  const loading = document.getElementById('loading');
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
