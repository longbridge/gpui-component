use anyhow::{anyhow, Result};
use bytes::Bytes;
use gpui::http_client::{self, http, AsyncBody, HttpClient, Url};
use std::pin::Pin;
use std::task::{Context, Poll};

// Re-export HeaderValue from http crate
use http::HeaderValue;

// Wrapper to make non-Send WASM futures Send
// SAFETY: WASM is single-threaded, so Send is trivially satisfied
struct SendWrapper<F>(F);
unsafe impl<F> Send for SendWrapper<F> {}

impl<F: std::future::Future> std::future::Future for SendWrapper<F> {
    type Output = F::Output;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        // SAFETY: This is safe because we're just forwarding the poll to the inner future
        unsafe { self.map_unchecked_mut(|s| &mut s.0).poll(cx) }
    }
}

pub struct WasmHttpClient {
    client: reqwest::Client,
}

impl WasmHttpClient {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }

    pub fn user_agent(_agent: &str) -> Result<Self> {
        // In WASM/browser environment, we should not set custom User-Agent
        // as it triggers CORS preflight requests that many servers don't allow.
        // The browser will automatically set an appropriate User-Agent.
        Ok(Self::new())
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
        let client = self.client.clone();

        let future = async move {
            // Extract request parts
            let (parts, body) = req.into_parts();

            // Convert method
            let method = reqwest::Method::from_bytes(parts.method.as_str().as_bytes())
                .map_err(|e| anyhow!("Invalid method: {}", e))?;

            // Build request
            let url = parts.uri.to_string();
            let mut request_builder = client.request(method, &url);

            // Add headers (skip headers that trigger CORS preflight)
            for (name, value) in parts.headers.iter() {
                let name_str = name.as_str();

                // Skip headers that trigger CORS preflight in browsers
                if name_str.eq_ignore_ascii_case("user-agent")
                    || name_str.eq_ignore_ascii_case("referer")
                {
                    continue;
                }

                if let Ok(value_str) = value.to_str() {
                    request_builder = request_builder.header(name_str, value_str);
                }
            }

            // Add body
            match body.0 {
                http_client::Inner::Empty => {}
                http_client::Inner::Bytes(cursor) => {
                    request_builder = request_builder.body(cursor.into_inner().to_vec());
                }
                http_client::Inner::AsyncReader(_) => {
                    return Err(anyhow!("AsyncReader body not supported in WASM"));
                }
            }

            // Send request
            let response = request_builder
                .send()
                .await
                .map_err(|e| anyhow!("Request failed: {}", e))?;

            // Build response
            let status = response.status();
            let mut builder = http::Response::builder().status(status.as_u16());

            // Copy headers
            for (name, value) in response.headers().iter() {
                builder = builder.header(name.as_str(), value.as_bytes());
            }

            // Get body
            let bytes = response
                .bytes()
                .await
                .map_err(|e| anyhow!("Failed to read response body: {}", e))?;
            let body = AsyncBody::from(Bytes::from(bytes));

            builder.body(body).map_err(|e| anyhow!("{}", e))
        };

        Box::pin(SendWrapper(future))
    }
}
