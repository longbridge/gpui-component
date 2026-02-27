use anyhow::anyhow;
use gpui::{AssetSource, Result, SharedString};
use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;
use web_sys::{Request, RequestInit, RequestMode};

/// WASM implementation - return error until downloaded
pub struct Assets {
    cache: Arc<RwLock<HashMap<String, Vec<u8>>>>,
    pending: Arc<RwLock<HashMap<String, bool>>>,
}

impl Assets {
    pub fn new() -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
            pending: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    async fn fetch_icon(url: &str) -> std::result::Result<Vec<u8>, wasm_bindgen::JsValue> {
        let opts = RequestInit::new();
        opts.set_method("GET");
        opts.set_mode(RequestMode::Cors);

        let request = Request::new_with_str_and_init(url, &opts)?;
        let window = web_sys::window().ok_or_else(|| wasm_bindgen::JsValue::from_str("No window"))?;
        let resp_value = wasm_bindgen_futures::JsFuture::from(window.fetch_with_request(&request)).await?;
        let resp: web_sys::Response = resp_value.dyn_into()?;

        if !resp.ok() {
            return Err(wasm_bindgen::JsValue::from_str(&format!("HTTP error: {}", resp.status())));
        }

        let array_buffer = wasm_bindgen_futures::JsFuture::from(resp.array_buffer()?).await?;
        let uint8_array = js_sys::Uint8Array::new(&array_buffer);
        Ok(uint8_array.to_vec())
    }
}

impl Default for Assets {
    fn default() -> Self {
        Self::new()
    }
}

impl AssetSource for Assets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        if path.is_empty() {
            return Ok(None);
        }

        if path.starts_with("icons/") && path.ends_with(".svg") {
            // Check if already cached
            if let Ok(cache) = self.cache.read() {
                if let Some(data) = cache.get(path) {
                    return Ok(Some(Cow::Owned(data.clone())));
                }
            }

            // Check if download is pending
            let is_pending = if let Ok(pending) = self.pending.read() {
                pending.contains_key(path)
            } else {
                false
            };

            if !is_pending {
                // Mark as pending and trigger download
                if let Ok(mut pending) = self.pending.write() {
                    pending.insert(path.to_string(), true);
                }

                let url = format!("/assets/{}", path);
                let path_clone = path.to_string();
                let cache = self.cache.clone();
                let pending = self.pending.clone();

                spawn_local(async move {
                    match Self::fetch_icon(&url).await {
                        Ok(bytes) => {
                            if let Ok(mut cache) = cache.write() {
                                cache.insert(path_clone.clone(), bytes);
                            }
                            if let Ok(mut pending) = pending.write() {
                                pending.remove(&path_clone);
                            }
                        }
                        Err(e) => {
                            log::error!("Failed to download icon {}: {:?}", path_clone, e);
                            if let Ok(mut pending) = pending.write() {
                                pending.remove(&path_clone);
                            }
                        }
                    }
                });
            }

            // Return error so GPUI won't cache and will retry
            Err(anyhow!("Icon not loaded yet: {}", path))
        } else {
            Ok(None)
        }
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        let _ = path;
        Ok(Vec::new())
    }
}
