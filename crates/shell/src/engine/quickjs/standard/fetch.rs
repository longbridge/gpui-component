use std::{io::Read as _, time::Duration};

use rquickjs::{Ctx, Exception, IntoJs, Object, Promise, Result, Value, function::Func};

use super::super::{host, scheduler};

const MAX_BODY_BYTES: u64 = 8 * 1024 * 1024;
const MAX_REDIRECTS: usize = 10;
const TIMEOUT: Duration = Duration::from_secs(30);

pub(super) fn install(ctx: &Ctx<'_>) -> Result<()> {
    ctx.globals().set("fetch", Func::from(fetch))
}

fn fetch<'js>(ctx: Ctx<'js>, url: String) -> Result<Promise<'js>> {
    let initial = reqwest::Url::parse(&url)
        .map_err(|error| Exception::throw_type(&ctx, &format!("invalid fetch URL: {error}")))?;
    authorize(&ctx, &initial)?;
    let capabilities = host::capabilities();

    scheduler::blocking(&ctx, "fetch(url)", move || request(capabilities, initial))
}

fn authorize(ctx: &Ctx<'_>, url: &reqwest::Url) -> Result<()> {
    let host_name = url.host_str().unwrap_or_default().to_ascii_lowercase();
    if host::capabilities().may_reach(&host_name) {
        Ok(())
    } else {
        Err(Exception::throw_type(
            ctx,
            &format!(
                "network access to `{host_name}` is not granted; add it to capabilities.network.hosts"
            ),
        ))
    }
}

fn request(
    capabilities: crate::Capabilities,
    mut url: reqwest::Url,
) -> std::result::Result<FetchResponse, String> {
    let client = reqwest::blocking::Client::builder()
        .timeout(TIMEOUT)
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|error| format!("creating HTTP client failed: {error}"))?;

    for _ in 0..=MAX_REDIRECTS {
        let response = client
            .get(url.clone())
            .send()
            .map_err(|error| format!("fetching {url} failed: {error}"))?;
        if response.status().is_redirection() {
            let location = response
                .headers()
                .get(reqwest::header::LOCATION)
                .ok_or_else(|| format!("redirect from {url} has no Location header"))?
                .to_str()
                .map_err(|error| format!("redirect from {url} has an invalid Location: {error}"))?;
            let next = url
                .join(location)
                .map_err(|error| format!("redirect from {url} is invalid: {error}"))?;
            let host_name = next.host_str().unwrap_or_default().to_ascii_lowercase();
            if !capabilities.may_reach(&host_name) {
                return Err(format!(
                    "redirect target `{host_name}` is not granted by capabilities.network.hosts"
                ));
            }
            url = next;
            continue;
        }

        let status = response.status().as_u16();
        let final_url = response.url().to_string();
        let mut bytes = Vec::new();
        response
            .take(MAX_BODY_BYTES + 1)
            .read_to_end(&mut bytes)
            .map_err(|error| format!("reading response from {final_url} failed: {error}"))?;
        if bytes.len() as u64 > MAX_BODY_BYTES {
            return Err(format!(
                "response body from {final_url} exceeded the {MAX_BODY_BYTES} byte limit"
            ));
        }
        return Ok(FetchResponse {
            status,
            url: final_url,
            body: String::from_utf8_lossy(&bytes).into_owned(),
        });
    }
    Err(format!("fetch exceeded the {MAX_REDIRECTS} redirect limit"))
}

struct FetchResponse {
    status: u16,
    url: String,
    body: String,
}

impl<'js> IntoJs<'js> for FetchResponse {
    fn into_js(self, ctx: &Ctx<'js>) -> Result<Value<'js>> {
        let response = Object::new(ctx.clone())?;
        response.set("status", self.status)?;
        response.set("ok", (200..300).contains(&self.status))?;
        response.set("url", self.url)?;
        let text = self.body;
        response.set("text", Func::from(move || text.clone()))?;
        Ok(response.into_value())
    }
}
