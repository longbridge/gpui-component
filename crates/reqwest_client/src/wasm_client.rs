use anyhow::{Result, anyhow};
use bytes::Bytes;
use gpui::http_client::{self, AsyncBody, HttpClient, RedirectPolicy, Url, http};
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use web_sys::{Request, RequestInit, RequestMode, Response as WebResponse};

// Re-export HeaderValue from http crate
use http::HeaderValue;

pub struct WasmHttpClient {
    user_agent: Option<String>,
}

impl WasmHttpClient {
    pub fn new() -> Self {
        Self { user_agent: None }
    }

    pub fn user_agent(agent: &str) -> Result<Self> {
        Ok(Self {
            user_agent: Some(agent.to_string()),
        })
    }
}

impl Default for WasmHttpClient {
    fn default() -> Self {
        Self::new()
    }
}

impl HttpClient for WasmHttpClient {
    fn proxy(&self) -> Option<&Url> {
        None
    }

    fn user_agent(&self) -> Option<&HeaderValue> {
        None
    }

    fn send(
        &self,
        req: http::Request<AsyncBody>,
    ) -> futures::future::BoxFuture<'static, Result<http_client::Response<AsyncBody>>> {
        let user_agent = self.user_agent.clone();

        // SAFETY: In WASM, we're in a single-threaded environment, so Send is not relevant.
        // We need to use unsafe to bypass the Send requirement from the trait.
        let future = async move {
            // Extract request parts early to avoid holding non-Send types across await
            let (parts, body) = req.into_parts();
            let method = parts.method.as_str().to_string();
            let uri = parts.uri.to_string();
            let headers: Vec<(String, String)> = parts
                .headers
                .iter()
                .filter_map(|(k, v)| {
                    v.to_str()
                        .ok()
                        .map(|v| (k.as_str().to_string(), v.to_string()))
                })
                .collect();
            let redirect_policy = parts.extensions.get::<RedirectPolicy>().cloned();

            // Convert body to bytes
            let body_bytes = match body.0 {
                http_client::Inner::Empty => None,
                http_client::Inner::Bytes(cursor) => Some(cursor.into_inner().to_vec()),
                http_client::Inner::AsyncReader(_) => {
                    return Err(anyhow!("AsyncReader body not supported in WASM"));
                }
            };

            // Now do the actual fetch in a way that keeps non-Send types local
            perform_fetch(
                method,
                uri,
                headers,
                body_bytes,
                redirect_policy,
                user_agent,
            )
            .await
        };

        // Wrap the future to make it Send
        // This is safe in WASM because it's single-threaded
        use std::pin::Pin;
        use std::task::{Context, Poll};
        struct SendWrapper<F>(F);
        // SAFETY: WASM is single-threaded, so Send is trivially satisfied
        unsafe impl<F> Send for SendWrapper<F> {}

        impl<F: std::future::Future> std::future::Future for SendWrapper<F> {
            type Output = F::Output;
            fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
                // SAFETY: This is safe because we're just forwarding the poll to the inner future
                unsafe { self.map_unchecked_mut(|s| &mut s.0).poll(cx) }
            }
        }

        Box::pin(SendWrapper(future))
    }
}

// This function does the actual fetch and keeps all web-sys types local
async fn perform_fetch(
    method: String,
    uri: String,
    headers: Vec<(String, String)>,
    body_bytes: Option<Vec<u8>>,
    redirect_policy: Option<RedirectPolicy>,
    user_agent: Option<String>,
) -> Result<http_client::Response<AsyncBody>> {
    // Create request init
    let opts = RequestInit::new();
    opts.set_method(&method);
    opts.set_mode(RequestMode::Cors);

    // Handle redirect policy
    if let Some(policy) = redirect_policy {
        match policy {
            RedirectPolicy::NoFollow => {
                opts.set_redirect(web_sys::RequestRedirect::Manual);
            }
            RedirectPolicy::FollowLimit(_) | RedirectPolicy::FollowAll => {
                opts.set_redirect(web_sys::RequestRedirect::Follow);
            }
        }
    }

    // Set body if provided
    if let Some(bytes) = body_bytes {
        let uint8_array = js_sys::Uint8Array::new_with_length(bytes.len() as u32);
        uint8_array.copy_from(&bytes);
        opts.set_body(&uint8_array);
    }

    // Create request
    let request = Request::new_with_str_and_init(&uri, &opts)
        .map_err(|e| anyhow!("Failed to create request: {:?}", e))?;

    // Set headers
    let req_headers = request.headers();
    for (key, value) in headers {
        req_headers
            .set(&key, &value)
            .map_err(|e| anyhow!("Failed to set header: {:?}", e))?;
    }

    // Set user agent if provided
    if let Some(ua) = user_agent {
        let _ = req_headers.set("user-agent", &ua);
    }

    // Get window and fetch
    let window = web_sys::window().ok_or_else(|| anyhow!("No window object"))?;
    let resp_value = JsFuture::from(window.fetch_with_request(&request))
        .await
        .map_err(|e| anyhow!("Fetch failed: {:?}", e))?;

    let resp: WebResponse = resp_value
        .dyn_into()
        .map_err(|_| anyhow!("Response is not a Response object"))?;

    // Build HTTP response
    let status = resp.status();
    let mut builder = http::Response::builder().status(status);

    // Copy headers
    let resp_headers = resp.headers();
    let headers_iter = js_sys::try_iter(&resp_headers)
        .map_err(|e| anyhow!("Failed to iterate headers: {:?}", e))?
        .ok_or_else(|| anyhow!("Headers iterator is not iterable"))?;

    for item in headers_iter {
        let item = item.map_err(|e| anyhow!("Failed to get header item: {:?}", e))?;
        let array: js_sys::Array = item.into();
        if array.length() == 2 {
            let key = array.get(0).as_string().unwrap_or_default();
            let value = array.get(1).as_string().unwrap_or_default();
            if let Ok(header_name) = http::HeaderName::from_bytes(key.as_bytes()) {
                if let Ok(header_value) = http::HeaderValue::from_str(&value) {
                    builder = builder.header(header_name, header_value);
                }
            }
        }
    }

    // Get response body
    let array_buffer = JsFuture::from(
        resp.array_buffer()
            .map_err(|e| anyhow!("Failed to get array buffer: {:?}", e))?,
    )
    .await
    .map_err(|e| anyhow!("Failed to read array buffer: {:?}", e))?;

    let uint8_array = js_sys::Uint8Array::new(&array_buffer);
    let bytes = uint8_array.to_vec();
    let body = AsyncBody::from(Bytes::from(bytes));

    builder.body(body).map_err(|e| anyhow!(e))
}
