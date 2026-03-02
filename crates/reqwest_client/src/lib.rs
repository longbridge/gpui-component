#[cfg(not(target_arch = "wasm32"))]
mod http_client_tls;
#[cfg(not(target_arch = "wasm32"))]
mod native_client;
#[cfg(not(target_arch = "wasm32"))]
pub use native_client::ReqwestClient;

#[cfg(target_arch = "wasm32")]
mod wasm_client;
#[cfg(target_arch = "wasm32")]
pub use wasm_client::WasmHttpClient;

// Re-export the appropriate client based on target
#[cfg(not(target_arch = "wasm32"))]
pub type HttpClient = ReqwestClient;

#[cfg(target_arch = "wasm32")]
pub type HttpClient = WasmHttpClient;
